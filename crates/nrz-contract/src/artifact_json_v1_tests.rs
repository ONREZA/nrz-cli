use serde::Deserialize;
use serde_json::{Value, json};

use super::artifact_json_v1::{Error, Limits, MAX_SUPPORTED_DEPTH, PROFILE_ID, decode, encode};

fn limits(max_bytes: usize, max_depth: usize) -> Limits {
    Limits {
        max_bytes,
        max_depth,
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureLimits {
    max_bytes: usize,
    max_depth: usize,
}

#[derive(Deserialize)]
struct Fixture {
    profile: String,
    limits: FixtureLimits,
    accepted: Vec<Accepted>,
    rejected: Vec<Rejected>,
}

#[derive(Deserialize)]
struct Accepted {
    name: String,
    input: String,
    canonical: String,
    blake3: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rejected {
    name: String,
    input: Option<String>,
    input_hex: Option<String>,
}

#[test]
fn shared_conformance_vectors() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../fixtures/artifact-json-v1-conformance.json"
    ))
    .unwrap();
    assert_eq!(fixture.profile, PROFILE_ID);
    let limits = limits(fixture.limits.max_bytes, fixture.limits.max_depth);
    for vector in fixture.accepted {
        let bytes = vector.canonical.as_bytes();
        assert_eq!(
            blake3::hash(bytes).to_hex().as_str(),
            vector.blake3,
            "{}",
            vector.name
        );
        let decoded = decode(bytes, limits).unwrap();
        assert_eq!(encode(&decoded, limits).unwrap(), bytes, "{}", vector.name);
        let value: Value = serde_json::from_str(&vector.input).unwrap();
        if vector.name == "integral-exponent-producer" {
            assert_eq!(vector.input, "1e3");
            assert_eq!(vector.canonical, "1000");
        }
        // AP exponent spelling cannot roundtrip through serde_json's f64 representation.
        let preserved_exponent = vector.name == "integral-exponent-producer"
            && vector.input == "1e3"
            && vector.canonical == "1000"
            && value.as_number().is_some_and(|number| {
                Some(number) != serde_json::Number::from_f64(1000.0).as_ref()
            });
        if preserved_exponent {
            assert!(matches!(encode(&value, limits), Err(Error::InvalidNumber)));
        } else {
            assert_eq!(encode(&value, limits).unwrap(), bytes, "{}", vector.name);
        }
    }
    for vector in fixture.rejected {
        let bytes = match (vector.input, vector.input_hex) {
            (Some(input), None) => input.into_bytes(),
            (None, Some(hex)) => {
                assert_eq!(hex.len() % 2, 0);
                hex.as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect()
            }
            _ => panic!("{}: expected exactly one input representation", vector.name),
        };
        assert!(decode(&bytes, limits).is_err(), "{}", vector.name);
    }
}

#[test]
fn depth_boundaries_match_native_parser_and_encoder() {
    for depth in [0, 1, MAX_SUPPORTED_DEPTH, MAX_SUPPORTED_DEPTH + 1] {
        let bytes = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        let mut value = json!(0);
        for _ in 0..depth {
            value = Value::Array(vec![value]);
        }
        let budget = limits(bytes.len(), MAX_SUPPORTED_DEPTH);
        if depth <= MAX_SUPPORTED_DEPTH {
            assert_eq!(decode(bytes.as_bytes(), budget).unwrap(), value);
            assert_eq!(encode(&value, budget).unwrap(), bytes.as_bytes());
        } else {
            assert!(matches!(
                decode(bytes.as_bytes(), budget),
                Err(Error::DepthLimit)
            ));
            assert!(matches!(encode(&value, budget), Err(Error::DepthLimit)));
        }
    }
    let scalar_only = limits(1024, 0);
    for value in [Value::Null, json!(true), json!(1), json!("[{}]")] {
        let bytes = encode(&value, scalar_only).unwrap();
        assert_eq!(decode(&bytes, scalar_only).unwrap(), value);
    }
    for value in [json!([]), json!({})] {
        assert!(matches!(
            encode(&value, scalar_only),
            Err(Error::DepthLimit)
        ));
        assert!(matches!(
            decode(value.to_string().as_bytes(), scalar_only),
            Err(Error::DepthLimit)
        ));
    }
    for input in [b"0".as_slice(), b"[]"] {
        assert!(matches!(
            decode(input, limits(1024, 128)),
            Err(Error::UnsupportedDepthLimit)
        ));
    }
    assert!(matches!(
        encode(&Value::Null, limits(1024, 128)),
        Err(Error::UnsupportedDepthLimit)
    ));
}

#[test]
fn depth_scan_ignores_quoted_and_escaped_brackets() {
    let value = json!({"brackets": "[{}]\\\"[[["});
    let bytes = encode(&value, limits(1024, 1)).unwrap();
    assert_eq!(decode(&bytes, limits(bytes.len(), 1)).unwrap(), value);
    let nested = b"{\"a\":[{}]}";
    assert!(matches!(
        decode(nested, limits(1024, 2)),
        Err(Error::DepthLimit)
    ));
    assert!(decode(nested, limits(1024, 3)).is_ok());
}

#[test]
fn byte_budget_counts_emitted_utf8_and_escapes() {
    for value in [json!("é😀"), json!("\n\u{0}\\\""), json!({"é": [1, 2]})] {
        let bytes = encode(&value, limits(1024, 127)).unwrap();
        assert_eq!(encode(&value, limits(bytes.len(), 127)).unwrap(), bytes);
        assert_eq!(decode(&bytes, limits(bytes.len(), 127)).unwrap(), value);
        assert!(matches!(
            encode(&value, limits(bytes.len() - 1, 127)),
            Err(Error::ByteLimit)
        ));
        assert!(matches!(
            decode(&bytes, limits(bytes.len() - 1, 127)),
            Err(Error::ByteLimit)
        ));
    }
    assert!(matches!(
        encode(&json!("x".repeat(1_000_000)), limits(8, 0)),
        Err(Error::ByteLimit)
    ));
    assert!(matches!(
        encode(&Value::Null, limits(0, 0)),
        Err(Error::ByteLimit)
    ));
    assert!(decode(b"", limits(0, 0)).is_err());
}

#[test]
fn producer_numbers_are_safe_integers_and_normalize_float_storage() {
    for (value, canonical) in [
        (json!(-0.0), "0"),
        (json!(1.0), "1"),
        (json!(1e3), "1000"),
        (json!(9_007_199_254_740_991_i64), "9007199254740991"),
        (json!(-9_007_199_254_740_991_i64), "-9007199254740991"),
    ] {
        assert_eq!(
            encode(&value, limits(100, 0)).unwrap(),
            canonical.as_bytes()
        );
    }
    for value in [
        json!(1.5),
        json!(9_007_199_254_740_992_u64),
        json!(-9_007_199_254_740_992_i64),
        json!(u64::MAX),
        json!(1e100),
    ] {
        assert!(matches!(
            encode(&value, limits(100, 0)),
            Err(Error::InvalidNumber)
        ));
    }
}

#[test]
fn producer_rejects_nonroundtrip_precision_when_preserved() {
    for raw in ["9007199254740991.1", "1e-999"] {
        let value: Value = serde_json::from_str(raw).unwrap();
        let stored = value.to_string();
        if stored == raw {
            assert!(matches!(
                encode(&value, limits(100, 0)),
                Err(Error::InvalidNumber)
            ));
        } else {
            // Standard serde_json has already rounded these inputs before the codec sees Value.
            assert!(encode(&value, limits(100, 0)).is_ok());
        }
    }
}

#[test]
fn decoder_rejects_noncanonical_and_invalid_spellings() {
    for bytes in [
        b"{\"a\":1,\"a\":1}".as_slice(),
        b"{\"b\":0,\"a\":0}",
        b" 1",
        b"1\n",
        b"1.0",
        b"1e0",
        b"-0",
        b"-0.0",
        b"9007199254740991.1",
        b"0.00000000000000000000000000001",
        b"\"\\u0061\"",
        b"\"\\/\"",
        b"\"\\ud800\"",
        b"\xef\xbb\xbfnull",
        b"\"\xff\"",
        b"[}",
        b"[",
    ] {
        assert!(decode(bytes, limits(1024, 127)).is_err(), "{bytes:?}");
    }
}

#[test]
fn comparison_preserves_complete_encoding_errors_and_length_checks() {
    for bytes in [b"0 ".as_slice(), b"1.0", br#""\u0008""#] {
        assert!(matches!(
            decode(bytes, limits(1024, 127)),
            Err(Error::NonCanonical)
        ));
    }
    let exponent: Value = serde_json::from_slice(b"1e3").unwrap();
    if matches!(
        encode(&exponent, limits(1024, 0)),
        Err(Error::InvalidNumber)
    ) {
        assert!(matches!(
            decode(b"1e3", limits(3, 0)),
            Err(Error::InvalidNumber)
        ));
    } else {
        assert!(matches!(
            decode(b"1e3", limits(3, 0)),
            Err(Error::ByteLimit)
        ));
    }
    assert!(matches!(
        decode(br#"{"b":1.5,"a":0}"#, limits(1024, 127)),
        Err(Error::InvalidNumber)
    ));
}

fn literal_object(key: &str, value: Value) -> Value {
    Value::Object([(key.to_owned(), value)].into_iter().collect())
}

#[test]
fn literal_serde_magic_keys_roundtrip_as_plain_json_objects() {
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
    ] {
        for payload in [
            Value::String("[0,0]".into()),
            Value::String("9007199254740992".into()),
            Value::String("not JSON or a number".into()),
            Value::Null,
            json!(1),
            json!([0, 1]),
        ] {
            let value = literal_object(key, payload);
            let bytes = encode(&value, limits(4096, 127)).unwrap();
            let decoded = decode(&bytes, limits(4096, 127)).unwrap();
            assert_eq!(decoded, value, "literal key {key}");
            assert_eq!(encode(&decoded, limits(4096, 127)).unwrap(), bytes);
        }
    }
}

#[test]
fn nested_magic_objects_keep_quoted_depth_and_numbers_as_strings() {
    let quoted = format!("{}9007199254740992{}", "[".repeat(128), "]".repeat(128));
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
    ] {
        let object = literal_object(key, Value::String(quoted.clone()));
        let bytes = encode(&object, limits(4096, 1)).unwrap();
        assert_eq!(decode(&bytes, limits(4096, 1)).unwrap(), object);
        let nested = literal_object("outer", Value::Array(vec![object]));
        let bytes = encode(&nested, limits(4096, 3)).unwrap();
        assert_eq!(decode(&bytes, limits(4096, 3)).unwrap(), nested);
    }
}

#[test]
fn escaped_and_late_magic_keys_keep_canonical_rejection() {
    for token in ["RawValue", "Number"] {
        let escaped = format!("{{\"\\u0024serde_json::private::{token}\":\"9007199254740992\"}}");
        let late = format!("{{\"a\":0,\"$serde_json::private::{token}\":\"9007199254740992\"}}");
        for bytes in [escaped.as_bytes(), late.as_bytes()] {
            assert!(
                matches!(decode(bytes, limits(4096, 127)), Err(Error::NonCanonical)),
                "{bytes:?}"
            );
        }
    }
}

#[test]
fn genuine_safe_numbers_remain_distinct_from_literal_number_objects() {
    let bytes = b"[-9007199254740991,0,1,9007199254740991]";
    let values = decode(bytes, limits(4096, 127)).unwrap();
    assert_eq!(
        values,
        json!([-9_007_199_254_740_991_i64, 0, 1, 9_007_199_254_740_991_i64])
    );
    assert!(values.as_array().unwrap().iter().all(Value::is_number));
    let object = literal_object("$serde_json::private::Number", Value::String("1".into()));
    let bytes = encode(&object, limits(4096, 127)).unwrap();
    assert_eq!(decode(&bytes, limits(4096, 127)).unwrap(), object);
}

#[test]
fn magic_key_maps_still_validate_numbers_after_canonical_mismatches() {
    for bytes in [
        br#"{"z":1.5,"$serde_json::private::Number":"1"}"#.as_slice(),
        br#"{"z":1.5,"$serde_json::private::RawValue":"[0,0]"}"#,
        br#"{"a":0,"a":0,"z":1.5}"#,
    ] {
        assert!(
            matches!(decode(bytes, limits(4096, 127)), Err(Error::InvalidNumber)),
            "{bytes:?}"
        );
    }
    assert!(matches!(
        decode(
            br#"{"$serde_json::private::Number":"1","$serde_json::private::Number":"1"}"#,
            limits(4096, 127)
        ),
        Err(Error::NonCanonical)
    ));
}

#[test]
fn allocation_profile_counts_raw_escaped_and_unfinished_tokens() {
    use super::artifact_json_v1::allocation_profile;
    for (bytes, escaped, unquoted) in [
        (br#"{"x":"plain [123]"}"#.as_slice(), 0, 0),
        (br#"{"x":"a\"b","n":-12.3e+4}"#, 4, 8),
        (br#"{"x":"\u0001"}"#, 6, 0),
        (br#"{"x":"\uZZ"#, 4, 0),
        (b"{\"x\":12345", 0, 5),
        (b"{\"x\":invalid-tail", 0, 12),
        (b"{\"x\":\"abc\\", 4, 0),
        (b"\"raw\xff\\q", 6, 0),
    ] {
        let profile = allocation_profile(bytes);
        assert_eq!(profile.max_escaped_string_bytes, escaped, "{bytes:?}");
        assert_eq!(profile.max_unquoted_run_bytes, unquoted, "{bytes:?}");
    }
}

#[test]
fn allocation_profile_funds_potential_numeric_heap_without_rejecting_tokens() {
    use super::artifact_json_v1::allocation_profile;
    for (bytes, candidate) in [
        (b"9999999999999999999".as_slice(), false),
        (b"-999999999999999999", false),
        (b"99999999999999999999", true),
        (b"-9999999999999999999", true),
        (b"1.0", true),
        (b"1e0", true),
        (b"1E+0", true),
        (b"-e", true),
        (b"true false null", false),
        (b"invalid-suffix-123456", true),
        (br#""9999999999999999999999e0""#, false),
        (br#"{"n":19,"s":"1e999"}"#, false),
        (b"{\"n\":1e\xff", true),
    ] {
        assert_eq!(
            allocation_profile(bytes).numeric_heap_candidate,
            candidate,
            "{bytes:?}"
        );
    }
}

#[test]
fn allocation_profile_tracks_maxima_instead_of_total_document_size() {
    use super::artifact_json_v1::allocation_profile;
    let item = br#"{"a":"x\ny","n":-123}"#;
    assert_eq!(
        allocation_profile(item),
        allocation_profile(&item.repeat(1024))
    );
    let unescaped = format!("\"{}\"", "x".repeat(64 * 1024));
    assert_eq!(
        allocation_profile(unescaped.as_bytes()),
        allocation_profile(b"\"x\"")
    );
    let escaped = format!("\"{}\"", "\\u0001".repeat(1024));
    assert_eq!(
        allocation_profile(escaped.as_bytes()).max_escaped_string_bytes,
        6 * 1024
    );
}
