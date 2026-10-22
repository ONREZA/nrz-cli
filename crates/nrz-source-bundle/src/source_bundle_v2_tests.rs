use serde_json::{Value, json};

use super::*;

pub(super) fn budget() -> SourceBundleV2ManifestBudget {
    SourceBundleV2ManifestBudget {
        max_file_count: 20,
        max_logical_bytes: 1000,
        max_static_file_bytes: 500,
    }
}

pub(super) fn manifest() -> Value {
    json!({
        "schemaVersion": "SOURCE_BUNDLE_V2.0", "capabilities": [],
        "files": [{"path":"public/index.html", "entryType":"file", "role":"static",
            "blobDigest":{"algorithm":"BLAKE3", "value":"a".repeat(64)},
            "size":12, "executable":false}],
        "layers":[], "routes":[], "entrypoints":[]
    })
}

fn compute_manifest() -> Value {
    let mut value = manifest();
    value["files"][0]["role"] = json!("compute");
    value["files"][0]["layerName"] = json!("server");
    value["capabilities"] = json!(["COMPUTE_NETWORK_V1"]);
    value["layers"] = json!([{"name":"server", "target":"COMPUTE", "rootPath":"public",
        "entrypoint":"public/index.html", "runtimeConfig":{
            "applicationRuntime":{"family":"BUN", "args":[]},
            "buildRuntimeVersion":"bun-1.4.2", "memoryMb":256,
            "network":crate::ComputeNetwork::public_default()}}]);
    value["routes"] = json!([{"pattern":"/", "layerName":"server"}]);
    value["entrypoints"] = json!(["public/index.html"]);
    value
}

#[test]
fn canonical_manifest_roundtrip_binds_exact_bytes_and_preserves_summary() {
    let value = manifest();
    let bytes = encode_source_bundle_v2_manifest(value.clone(), budget()).unwrap();
    let decoded = decode_source_bundle_v2_manifest(&bytes, budget()).unwrap();
    assert_eq!(
        decoded.manifest_blake3(),
        blake3::hash(&bytes).to_hex().as_str()
    );
    assert_eq!(
        decoded.summary(),
        SourceBundleV2ManifestSummary {
            file_count: 1,
            logical_static_bytes: 12,
            artifact_size_bytes: 0,
            max_static_file_size_bytes: 12
        }
    );
    assert_eq!(serde_json::to_value(decoded.wire()).unwrap(), value);
    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    assert!(decode_source_bundle_v2_manifest(&noncanonical, budget()).is_err());
}

#[test]
fn editing_raw_clones_and_summary_snapshots_preserves_the_verified_proof() {
    let verified = verify_source_bundle_v2_manifest(manifest(), budget()).unwrap();
    let mut wire = verified.wire().clone();
    let mut summary = verified.summary();
    wire.files.clear();
    summary.file_count = 0;
    assert_eq!(verified.wire().files.len(), 1);
    assert_eq!(verified.summary().file_count, 1);
    assert_ne!(summary, verified.summary());

    let reverified =
        verify_source_bundle_v2_manifest(serde_json::to_value(wire).unwrap(), budget()).unwrap();
    assert_eq!(reverified.summary().file_count, 0);
    assert_ne!(reverified.manifest_blake3(), verified.manifest_blake3());
}

#[test]
fn symlink_entrypoints_and_native_order_directory_resolution_are_preserved() {
    let mut value = compute_manifest();
    value["files"].as_array_mut().unwrap().push(json!({
        "path":"public/start", "entryType":"symlink", "linkTarget":"index.html",
        "size":0, "role":"compute", "layerName":"server"
    }));
    value["layers"][0]["entrypoint"] = json!("public/start");
    value["entrypoints"] = json!(["public/start"]);
    let verified = verify_source_bundle_v2_manifest(value, budget()).unwrap();
    assert_eq!(verified.summary().file_count, 2);
    assert_eq!(verified.summary().artifact_size_bytes, 12);

    let mut value = manifest();
    value["files"] = json!([
        {"path":"alias", "entryType":"symlink", "linkTarget":"physical/nested", "size":0, "role":"static"},
        {"path":"physical/nested/file", "entryType":"file", "blobDigest":{"algorithm":"BLAKE3", "value":"b".repeat(64)}, "size":1, "executable":false, "role":"static"},
        {"path":"physical/target", "entryType":"file", "blobDigest":{"algorithm":"BLAKE3", "value":"c".repeat(64)}, "size":1, "executable":false, "role":"static"},
        {"path":"result", "entryType":"symlink", "linkTarget":"alias/../target", "size":0, "role":"static"}
    ]);
    assert!(verify_source_bundle_v2_manifest(value, budget()).is_ok());
}

#[test]
fn unsafe_graphs_and_semantic_claims_are_rejected() {
    for target in ["missing", "/public/index.html", "..", ".", "alias"] {
        let mut value = manifest();
        value["files"].as_array_mut().unwrap().insert(
            0,
            json!({
                "path":"alias", "entryType":"symlink", "linkTarget":target,
                "size":0, "role":"static"
            }),
        );
        assert!(
            verify_source_bundle_v2_manifest(value, budget()).is_err(),
            "target={target}"
        );
    }
    for (field, changed) in [
        ("path", json!(".__onreza/hidden")),
        ("path", json!("public/../escape")),
        ("role", json!("dependency")),
        ("layerName", json!("unknown")),
        ("size", json!(MAX_SAFE_INTEGER + 1)),
        ("size", json!(-1)),
        ("executable", Value::Null),
        ("mode", json!(493)),
        (
            "blobDigest",
            json!({"algorithm":"SHA256", "value":"a".repeat(64)}),
        ),
    ] {
        let mut value = manifest();
        value["files"][0][field] = changed;
        assert!(
            verify_source_bundle_v2_manifest(value, budget()).is_err(),
            "field={field}"
        );
    }
    let mut value = compute_manifest();
    value["entrypoints"] = json!([]);
    assert!(verify_source_bundle_v2_manifest(value, budget()).is_err());
    let mut value = compute_manifest();
    value["layers"][0]["runtimeConfig"]["memoryMb"] = json!(31);
    assert!(verify_source_bundle_v2_manifest(value, budget()).is_err());
}

#[test]
fn caller_budgets_are_mandatory_limits_including_zero() {
    for limit in [
        SourceBundleV2ManifestBudget {
            max_file_count: 0,
            ..budget()
        },
        SourceBundleV2ManifestBudget {
            max_logical_bytes: 11,
            ..budget()
        },
        SourceBundleV2ManifestBudget {
            max_static_file_bytes: 11,
            ..budget()
        },
        SourceBundleV2ManifestBudget {
            max_logical_bytes: MAX_SAFE_INTEGER + 1,
            ..budget()
        },
    ] {
        assert!(verify_source_bundle_v2_manifest(manifest(), limit).is_err());
    }
    let mut value = manifest();
    value["files"] = json!([]);
    let zero = SourceBundleV2ManifestBudget {
        max_file_count: 0,
        max_logical_bytes: 0,
        max_static_file_bytes: 0,
    };
    assert!(verify_source_bundle_v2_manifest(value, zero).is_ok());
}

#[test]
fn producer_integral_float_sizes_use_the_canonical_integer_value() {
    let mut value = manifest();
    value["files"][0]["size"] = json!(12.0);
    let verified = verify_source_bundle_v2_manifest(value.clone(), budget()).unwrap();
    let (producer_bytes, producer_verified) =
        encode_verified_source_bundle_v2_manifest(value.clone(), budget()).unwrap();
    let bytes = encode_source_bundle_v2_manifest(value, budget()).unwrap();
    assert_eq!(producer_bytes, bytes);
    for verified in [
        verified,
        producer_verified,
        decode_source_bundle_v2_manifest(&bytes, budget()).unwrap(),
    ] {
        assert_eq!(verified.manifest_size_bytes(), bytes.len() as u64);
        assert_eq!(
            verified.manifest_blake3(),
            blake3::hash(&bytes).to_hex().as_str()
        );
        assert_eq!(verified.summary().logical_static_bytes, 12);
        assert_eq!(
            serde_json::to_value(verified.wire()).unwrap()["files"][0]["size"],
            json!(12)
        );
    }
}

#[test]
fn logical_sum_must_remain_safe_even_when_each_file_size_is_safe() {
    let mut value = manifest();
    value["files"][0]["size"] = json!(MAX_SAFE_INTEGER);
    let mut second = value["files"][0].clone();
    second["path"] = json!("second");
    second["size"] = json!(1);
    value["files"].as_array_mut().unwrap().push(second);
    let maximum = SourceBundleV2ManifestBudget {
        max_file_count: 2,
        max_logical_bytes: MAX_SAFE_INTEGER,
        max_static_file_bytes: MAX_SAFE_INTEGER,
    };
    assert!(verify_source_bundle_v2_manifest(value, maximum).is_err());
}

#[test]
fn empty_static_roots_do_not_require_an_archive_entry() {
    let mut value = manifest();
    value["layers"] = json!([{"name":"empty", "target":"STATIC", "rootPath":"empty"}]);
    assert!(verify_source_bundle_v2_manifest(value, budget()).is_ok());
}

#[test]
fn file_digests_require_the_complete_lowercase_blake3_encoding() {
    for digest in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        format!("{}\n", "a".repeat(64)),
    ] {
        let mut value = manifest();
        value["files"][0]["blobDigest"]["value"] = json!(digest);
        assert!(
            verify_source_bundle_v2_manifest(value, budget()).is_err(),
            "digest={digest:?}"
        );
    }
}

#[test]
fn runtime_policy_lineage_preserves_the_supported_version_grammar() {
    for lineage in [
        "BUN:CURRENT",
        "NODE:1",
        "NODE:999999999",
        "PYTHON:1.0",
        "PYTHON:999999999.999999999",
    ] {
        let mut value = compute_manifest();
        value["layers"][0]["runtimeConfig"]["runtimePolicyLineage"] = json!(lineage);
        assert!(
            verify_source_bundle_v2_manifest(value, budget()).is_ok(),
            "lineage={lineage}"
        );
    }
    for lineage in [
        "",
        "bun:CURRENT",
        "BUN:NEXT",
        "NODE:0",
        "NODE:01",
        "NODE:1000000000",
        "NODE:+1",
        "NODE:1\n",
        "PYTHON:0.1",
        "PYTHON:1",
        "PYTHON:1.01",
        "PYTHON:1.1000000000",
        "PYTHON:1.2.3",
    ] {
        let mut value = compute_manifest();
        value["layers"][0]["runtimeConfig"]["runtimePolicyLineage"] = json!(lineage);
        assert!(
            verify_source_bundle_v2_manifest(value, budget()).is_err(),
            "lineage={lineage:?}"
        );
    }
}

#[test]
fn route_headers_keep_the_authoring_alphabet_and_native_size_bounds() {
    for (name, accepted) in [
        ("!#$%&'*+-.^`|~0123456789Az", true),
        ("X-Valid", true),
        ("X_Underscore", false),
        ("", false),
        ("X Space", false),
        ("X:Colon", false),
        ("X\nLine", false),
        ("X-Юникод", false),
    ] {
        let mut value = manifest();
        value["layers"] = json!([{"name":"web", "target":"STATIC"}]);
        value["routes"] = json!([{"pattern":"/", "layerName":"web", "headers":{name:"ok"}}]);
        assert_eq!(
            verify_source_bundle_v2_manifest(value, budget()).is_ok(),
            accepted,
            "header={name:?}"
        );
    }
    for (name_bytes, value_bytes, accepted) in
        [(256, 8192, true), (257, 0, false), (1, 8193, false)]
    {
        let name = "x".repeat(name_bytes);
        let mut value = manifest();
        value["layers"] = json!([{"name":"web", "target":"STATIC"}]);
        value["routes"] = json!([
            {"pattern":"/plain", "layerName":"web"},
            {"pattern":"/valid", "layerName":"web", "headers":{"x-valid":"ok"}},
            {"pattern":"/", "layerName":"web", "headers":{name:"a".repeat(value_bytes)}}
        ]);
        assert_eq!(
            verify_source_bundle_v2_manifest(value, budget()).is_ok(),
            accepted,
            "header bytes={name_bytes}, value bytes={value_bytes}"
        );
    }
}

#[test]
fn nested_wire_shapes_and_network_facets_remain_strict() {
    let base = compute_manifest();
    assert!(verify_source_bundle_v2_manifest(base.clone(), budget()).is_ok());

    for (parent, field, changed) in [
        ("/files/0/blobDigest", "extra", json!(true)),
        ("/layers/0", "extra", json!(true)),
        ("/layers/0/runtimeConfig", "extra", json!(true)),
        (
            "/layers/0/runtimeConfig",
            "runtimeFamily",
            json!("JAVASCRIPT"),
        ),
        ("/layers/0/runtimeConfig", "isBinaryEntry", json!(false)),
        ("/layers/0/runtimeConfig", "maxConcurrency", json!(1)),
        (
            "/layers/0/runtimeConfig",
            "readiness",
            json!({"protocol":"TCP"}),
        ),
        ("/layers/0/runtimeConfig", "memoryMb", Value::Null),
        (
            "/layers/0/runtimeConfig/applicationRuntime",
            "extra",
            json!(true),
        ),
        (
            "/layers/0/runtimeConfig/applicationRuntime",
            "family",
            json!("bun"),
        ),
        (
            "/layers/0/runtimeConfig/applicationRuntime",
            "args",
            json!([0]),
        ),
        (
            "/layers/0/runtimeConfig/applicationRuntime",
            "args",
            json!(["a".repeat(4097)]),
        ),
        (
            "/layers/0/runtimeConfig/applicationRuntime",
            "args",
            json!(vec![""; 65]),
        ),
        ("/layers/0/runtimeConfig/network", "extra", json!(true)),
        (
            "/layers/0/runtimeConfig/network",
            "schemaVersion",
            json!("OTHER"),
        ),
        (
            "/layers/0/runtimeConfig/network",
            "exposure",
            json!("other"),
        ),
        ("/layers/0/runtimeConfig/network", "ports", json!([])),
        (
            "/layers/0/runtimeConfig/network/ports/0",
            "extra",
            json!(true),
        ),
        (
            "/layers/0/runtimeConfig/network/ports/0",
            "name",
            json!("_leading"),
        ),
        (
            "/layers/0/runtimeConfig/network/ports/0",
            "name",
            json!("a".repeat(65)),
        ),
        (
            "/layers/0/runtimeConfig/network/ports/0",
            "protocol",
            json!("HTTP"),
        ),
        ("/layers/0/runtimeConfig/network/ports/0", "port", json!(0)),
        (
            "/layers/0/runtimeConfig/network/ports/0",
            "port",
            json!(65536),
        ),
        (
            "/layers/0/runtimeConfig/network/ports/0",
            "binding",
            json!("other"),
        ),
        (
            "/layers/0/runtimeConfig/network/readiness",
            "extra",
            json!(true),
        ),
        (
            "/layers/0/runtimeConfig/network/readiness",
            "path",
            Value::Null,
        ),
        (
            "/layers/0/runtimeConfig/network/readiness",
            "port",
            json!("missing"),
        ),
        (
            "/layers/0/runtimeConfig/network/readiness",
            "protocol",
            json!("TCP"),
        ),
        (
            "/layers/0/runtimeConfig/network",
            "allowFrom",
            json!([{
                "serviceId":"00000000-0000-0000-0000-000000000000", "layerName":"caller"
            }]),
        ),
        (
            "/layers/0/runtimeConfig/network",
            "allowFrom",
            json!([{
                "serviceId":"11111111-1111-1111-1111-111111111111", "layerName":""
            }]),
        ),
        ("/routes/0", "extra", json!(true)),
        ("/routes/0", "methods", json!(["OTHER"])),
        ("/routes/0", "priority", Value::Null),
        ("/routes/0", "fallthroughWhen", json!([])),
        (
            "/routes/0",
            "fallthroughWhen",
            json!([{"type":"other", "name":"x"}]),
        ),
        (
            "/routes/0",
            "fallthroughWhen",
            json!([{"type":"query", "name":"x", "extra":0}]),
        ),
        (
            "/routes/0",
            "fallthroughWhen",
            json!([{"type":"query", "name":"x", "value":null}]),
        ),
        (
            "/routes/0",
            "fallthroughWhen",
            json!([{"type":"query", "name":"a".repeat(65)}]),
        ),
    ] {
        let mut value = base.clone();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), changed.clone());
        assert!(
            verify_source_bundle_v2_manifest(value, budget()).is_err(),
            "{parent}/{field}={changed}"
        );
    }
    for parent in [
        "/layers/0/runtimeConfig/applicationRuntime",
        "/layers/0/runtimeConfig/network",
        "/layers/0/runtimeConfig/network/ports/0",
        "/layers/0/runtimeConfig/network/readiness",
    ] {
        let fields = base.pointer(parent).unwrap().as_object().unwrap();
        for field in fields.keys() {
            let mut value = base.clone();
            value
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(
                verify_source_bundle_v2_manifest(value, budget()).is_err(),
                "missing {parent}/{field}"
            );
        }
    }
    for (protocol, path) in [("tcp", Some("/")), ("http", None)] {
        let mut value = compute_manifest();
        value["layers"][0]["runtimeConfig"]["network"]["readiness"] =
            json!({"protocol":protocol, "port":"http"});
        if let Some(path) = path {
            value["layers"][0]["runtimeConfig"]["network"]["readiness"]["path"] = json!(path);
        }
        assert!(verify_source_bundle_v2_manifest(value, budget()).is_err());
    }
}

#[test]
fn compute_layers_require_explicit_application_and_network_declarations() {
    let base = compute_manifest();
    assert!(verify_source_bundle_v2_manifest(base.clone(), budget()).is_ok());
    for (parent, field) in [
        ("/layers/0", "runtimeConfig"),
        ("/layers/0/runtimeConfig", "applicationRuntime"),
        ("/layers/0/runtimeConfig", "network"),
    ] {
        let mut value = base.clone();
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            verify_source_bundle_v2_manifest(value.clone(), budget()).is_err(),
            "missing {field}"
        );
        assert!(
            encode_source_bundle_v2_manifest(value.clone(), budget()).is_err(),
            "missing {field}"
        );
        let bytes = artifact_json_v1::encode(&value, limits()).unwrap();
        assert!(
            decode_source_bundle_v2_manifest(&bytes, budget()).is_err(),
            "missing {field}"
        );
    }
}

fn corpus_budget(value: &Value) -> Option<SourceBundleV2ManifestBudget> {
    Some(SourceBundleV2ManifestBudget {
        max_file_count: value.get("maxFileCount")?.as_u64()?,
        max_logical_bytes: value.get("maxLogicalBytes")?.as_u64()?,
        max_static_file_bytes: value.get("maxStaticFileBytes")?.as_u64()?,
    })
}

fn summary_value(summary: SourceBundleV2ManifestSummary) -> Value {
    json!({
        "fileCount":summary.file_count,
        "logicalStaticBytes":summary.logical_static_bytes,
        "artifactSizeBytes":summary.artifact_size_bytes,
        "maxStaticFileSizeBytes":summary.max_static_file_size_bytes,
    })
}

#[test]
fn shared_source_bundle_v2_conformance_corpus() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../nrz-contract/fixtures/source-bundle-v2-conformance.json"
    ))
    .unwrap();
    for case in corpus["accepted"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let budget = corpus_budget(case.get("budget").unwrap_or(&corpus["budget"])).unwrap();
        let value = case["manifest"].clone();
        let bytes = encode_source_bundle_v2_manifest(value.clone(), budget)
            .unwrap_or_else(|error| panic!("encode {name}: {error}"));
        let (producer_bytes, producer_verified) =
            encode_verified_source_bundle_v2_manifest(value.clone(), budget)
                .unwrap_or_else(|error| panic!("encode verified {name}: {error}"));
        assert_eq!(producer_bytes, bytes, "{name}");
        assert_eq!(
            std::str::from_utf8(&bytes).unwrap(),
            case["canonical"].as_str().unwrap(),
            "{name}"
        );
        let produced = verify_source_bundle_v2_manifest(value, budget)
            .unwrap_or_else(|error| panic!("verify {name}: {error}"));
        let decoded = decode_source_bundle_v2_manifest(&bytes, budget)
            .unwrap_or_else(|error| panic!("decode {name}: {error}"));
        let pool = admission::ManifestMemoryPool::new(256 * 1024 * 1024);
        let mut input = pool.reserve_input(bytes.len()).unwrap();
        input.append(&bytes).unwrap();
        let admitted = input
            .verify(budget, u64::MAX)
            .unwrap_or_else(|error| panic!("admit {name}: {error}"));
        for verified in [&produced, &producer_verified, &decoded, admitted.verified()] {
            assert_eq!(verified.manifest_size_bytes(), bytes.len() as u64, "{name}");
            assert_eq!(
                verified.manifest_blake3(),
                case["blake3"].as_str().unwrap(),
                "{name}"
            );
            assert_eq!(summary_value(verified.summary()), case["summary"], "{name}");
            assert_eq!(
                serde_json::to_value(verified.wire()).unwrap(),
                case["manifest"],
                "{name}"
            );
        }
    }
    for case in corpus["rejected"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        // Negative/fractional JSON budgets cannot inhabit the Rust u64 API.
        let Some(budget) = corpus_budget(case.get("budget").unwrap_or(&corpus["budget"])) else {
            continue;
        };
        assert!(
            verify_source_bundle_v2_manifest(case["manifest"].clone(), budget).is_err(),
            "verify {name}"
        );
        assert!(
            encode_source_bundle_v2_manifest(case["manifest"].clone(), budget).is_err(),
            "encode {name}"
        );
    }
}

#[test]
fn producer_number_normalization_preserves_literal_magic_objects() {
    for key in [
        "$serde_json::private::RawValue",
        "$serde_json::private::Number",
    ] {
        let object = Value::Object(
            [(key.to_owned(), Value::String("[0,0]".into()))]
                .into_iter()
                .collect(),
        );
        let producer = Value::Array(vec![json!(1.0), object.clone()]);
        let bytes = artifact_json_v1::encode(&producer, limits()).unwrap();
        let normalized = normalize_producer_numbers(producer, &bytes).unwrap();
        assert_eq!(normalized, Value::Array(vec![json!(1), object]));
        assert_eq!(
            artifact_json_v1::encode(&normalized, limits()).unwrap(),
            bytes
        );
    }
}
