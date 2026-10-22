use super::*;

fn inspect(bytes: &[u8], max_file_count: u64) -> Result<StructuralCounts, PreflightError> {
    super::inspect(
        bytes,
        max_file_count,
        nrz_contract::artifact_json_v1::allocation_profile(bytes),
    )
}

fn inspect_value(value: &serde_json::Value) -> StructuralCounts {
    inspect(&serde_json::to_vec(value).unwrap(), u64::MAX).unwrap()
}

#[test]
fn counts_every_value_member_element_and_decoded_string_in_unknown_branches() {
    let bytes = br#"{"files":[{"entryType":"file","path":"a\/b"},{"entryType":"symlink","path":"a\u002fc","linkTarget":"b"}],"unknown":{"files":[null,true,1,-2,1,"\u00e9",{}],"path":"ignored"}}"#;
    let counts = inspect(bytes, 2).unwrap();
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let mut expected = StructuralCounts::default();
    fn walk(value: &serde_json::Value, counts: &mut StructuralCounts) {
        counts.json_values += 1;
        match value {
            serde_json::Value::String(value) => count_string(value, counts),
            serde_json::Value::Array(values) => {
                counts.array_elements += values.len() as u64;
                for value in values {
                    walk(value, counts);
                }
            }
            serde_json::Value::Object(values) => {
                counts.objects += 1;
                counts.object_members += values.len() as u64;
                for (key, value) in values {
                    count_string(key, counts);
                    walk(value, counts);
                }
            }
            _ => {}
        }
    }
    fn count_string(value: &str, counts: &mut StructuralCounts) {
        counts.strings += 1;
        counts.decoded_string_bytes += value.len() as u64;
        counts.max_string_bytes = counts.max_string_bytes.max(value.len() as u64);
    }
    walk(&value, &mut expected);
    expected.allocation_profile = nrz_contract::artifact_json_v1::allocation_profile(bytes);
    expected.files = 2;
    expected.path_components = 4;
    expected.unique_path_nodes = 3;
    expected.max_path_bytes = 3;
    expected.max_target_bytes = 1;
    expected.symlinks = 1;
    expected.max_error_string_bytes = expected.max_string_bytes;
    assert_eq!(counts.objects, 5);
    assert_eq!(counts, expected);
}

#[test]
fn nonintegral_numbers_remain_conservatively_counted_before_profile_rejection() {
    let bytes = br#"{"unknown":1.5}"#;
    let counts = inspect(bytes, 0).unwrap();
    // arbitrary_precision exposes nonintegral numbers through an extra synthetic map.
    assert!(counts.json_values >= 2);
    assert!(counts.objects >= 1);
    assert!(counts.object_members >= 1);
    assert!(counts.strings >= 1);
    assert!(counts.decoded_string_bytes >= 7);
    assert!(matches!(
        nrz_contract::artifact_json_v1::decode(
            bytes,
            nrz_contract::artifact_json_v1::Limits {
                max_bytes: bytes.len(),
                max_depth: 127,
            }
        ),
        Err(nrz_contract::artifact_json_v1::Error::InvalidNumber)
    ));
}

#[test]
fn unique_components_are_exact_for_sorted_paths_and_conservative_when_unsorted() {
    for (paths, unique) in [
        (vec!["a/b/x", "a/b/y", "a/c/z", "d"], 7),
        (vec!["a/b/x", "d", "a/b/y"], 7),
        (vec!["a/b/x", "a/b/x"], 3),
    ] {
        let value = serde_json::json!({"files": paths.iter().map(|path| {
            serde_json::json!({"entryType":"file", "path":path})
        }).collect::<Vec<_>>()});
        assert_eq!(inspect_value(&value).unique_path_nodes, unique);
    }
    let counts = inspect(br#"{"files":[],"layers":[],"routes":[]}"#, 0).unwrap();
    assert_eq!(
        (
            counts.files,
            counts.path_components,
            counts.unique_path_nodes
        ),
        (0, 0, 0)
    );
}

#[test]
fn file_limit_rejects_before_parsing_the_next_element() {
    for (bytes, max_files) in [
        (br#"{"files":["\uZZZZ"]}"#.as_slice(), 0),
        (
            br#"{"files":[{"path":"a/b","entryType":"file"},"\uZZZZ"]}"#.as_slice(),
            1,
        ),
    ] {
        assert!(matches!(
            inspect(bytes, max_files),
            Err(PreflightError::ResourceLimit(
                PreflightResourceLimit::FileCount
            ))
        ));
    }
}

#[test]
fn layer_and_route_limits_reject_before_parsing_the_next_element() {
    for (field, limit) in [("layers", 10), ("routes", 200)] {
        let accepted = format!("{{\"{field}\":[{}]}}", vec!["null"; limit].join(","));
        assert!(inspect(accepted.as_bytes(), 0).is_ok());
        let excessive = format!(
            "{{\"{field}\":[{},\"\\uZZZZ\"]}}",
            vec!["null"; limit].join(",")
        );
        let Err(PreflightError::Invalid(error)) = inspect(excessive.as_bytes(), 0) else {
            panic!("expected format limit")
        };
        assert!(error.to_string().contains("layer or route count"));
    }
}

#[test]
fn rejects_ambiguous_structural_fields_and_shapes_without_producing_counts() {
    for bytes in [
        br#"[]"#.as_slice(),
        br#"{"files":{}}"#,
        br#"{"files":[[]]}"#,
        br#"{"files":[{"entryType":"file","path":{}}]}"#,
        br#"{"files":[{"entryType":"file","path":"a","path":"b"}]}"#,
        br#"{"files":[{"entryType":"file","entryType":"symlink","path":"a"}]}"#,
        br#"{"files":[{"entryType":"other","path":"a"}]}"#,
        br#"{"files":[{"entryType":"file"}]}"#,
        br#"{"files":[],"files":[]}"#,
        br#"{"layers":{}}"#,
        br#"{"routes":null}"#,
        br#"{"files":[]} trailing"#,
    ] {
        assert!(
            matches!(inspect(bytes, 10), Err(PreflightError::Invalid(_))),
            "{bytes:?}"
        );
    }
}

#[test]
fn malformed_shape_errors_never_copy_a_large_string_into_diagnostics() {
    let huge = "\\u0000".repeat(8192);
    for bytes in [
        format!("\"{huge}\""),
        format!("{{\"files\":\"{huge}\"}}"),
        format!("{{\"files\":[\"{huge}\"]}}"),
        format!("{{\"layers\":\"{huge}\"}}"),
        format!("{{\"routes\":\"{huge}\"}}"),
    ] {
        let Err(PreflightError::Invalid(error)) = inspect(bytes.as_bytes(), 1) else {
            panic!("expected shape rejection")
        };
        assert!(error.to_string().len() < 200);
    }
}

#[test]
fn unknown_duplicate_keys_are_counted_even_when_canonical_validation_will_reject() {
    let counts = inspect(br#"{"unknown":"x","unknown":"y"}"#, 0).unwrap();
    assert_eq!(counts.object_members, 2);
    assert_eq!(counts.strings, 4);
    assert_eq!(counts.decoded_string_bytes, 16);
}

#[test]
fn metadata_counts_include_only_the_entire_root_layers_subtree() {
    let counts = inspect_value(&serde_json::json!({
        "layers": [{"name":"a","runtimeConfig":{"unknown":["é",{"nested":"x"}]}},{}],
        "files": [{"entryType":"file","path":"a","contentType":"z".repeat(8192)}],
        "unknown": {"layers":[{"name":"outside"}]}
    }));
    assert_eq!(counts.metadata_objects, 4);
    assert_eq!(counts.metadata_object_members, 4);
    assert_eq!(counts.metadata_array_elements, 4);
    assert_eq!(counts.metadata_strings, 7);
    assert_eq!(counts.metadata_decoded_string_bytes, 34);
}

#[test]
fn diagnostic_string_bound_excludes_only_direct_safe_file_string_values() {
    let mut value = serde_json::json!({"files":[{
        "entryType":"symlink", "path":"a".repeat(3000),
        "linkTarget":"b".repeat(4000), "contentType":"c".repeat(5000)
    }]});
    assert_eq!(inspect_value(&value).max_error_string_bytes, 11);
    value["files"][0]["blobDigest"] = serde_json::json!({"path":"x".repeat(6000)});
    assert_eq!(inspect_value(&value).max_error_string_bytes, 6000);
    value["files"][0]["role"] = serde_json::Value::String("x".repeat(7000));
    assert_eq!(inspect_value(&value).max_error_string_bytes, 7000);
    value
        .as_object_mut()
        .unwrap()
        .insert("x".repeat(8000), serde_json::Value::Null);
    assert_eq!(inspect_value(&value).max_error_string_bytes, 8000);
}

#[test]
fn accepts_depth_127_and_rejects_depth_128_with_no_new_format_cap() {
    for (nested, accepted) in [(126, true), (127, false), (1024, false)] {
        let bytes = format!(
            "{{\"unknown\":{}0{}}}",
            "[".repeat(nested),
            "]".repeat(nested)
        );
        assert_eq!(inspect(bytes.as_bytes(), 0).is_ok(), accepted);
    }
}

#[test]
fn counts_long_escaped_strings_and_paths_without_a_length_cap() {
    let decoded = "x".repeat(8192);
    let escaped = "\\u0078".repeat(8192);
    let path = format!("{}last", "a\\u002f".repeat(4096));
    let bytes = format!(
        "{{\"files\":[{{\"entryType\":\"symlink\",\"path\":\"{path}\",\"linkTarget\":\"{escaped}\"}}],\"unknown\":\"{escaped}\"}}"
    );
    let counts = inspect(bytes.as_bytes(), 1).unwrap();
    assert_eq!(counts.path_components, 4097);
    assert_eq!(counts.unique_path_nodes, 4097);
    assert_eq!(counts.max_path_bytes, 8196);
    assert_eq!(counts.max_target_bytes, decoded.len() as u64);
    assert_eq!(counts.max_string_bytes, 8196);
}

#[test]
fn counter_overflow_is_a_resource_error() {
    let mut state = State {
        counts: StructuralCounts {
            decoded_string_bytes: u64::MAX,
            ..StructuralCounts::default()
        },
        previous_path: Cow::Borrowed(""),
        max_file_count: u64::MAX,
        resource_limit: None,
    };
    assert!(
        state
            .string::<serde_json::Error>("x", Context::Other, false, None)
            .is_err()
    );
    assert_eq!(
        state.resource_limit,
        Some(PreflightResourceLimit::CounterOverflow)
    );
}

#[test]
fn previous_path_borrows_then_owns_then_borrows_without_changing_counts() {
    let mut state = State {
        counts: StructuralCounts {
            files: 1,
            ..StructuralCounts::default()
        },
        previous_path: Cow::Borrowed(""),
        max_file_count: u64::MAX,
        resource_limit: None,
    };
    let plain = "a/b";
    state
        .string::<serde_json::Error>(plain, Context::Path, false, Some(plain))
        .unwrap();
    assert!(matches!(state.previous_path, Cow::Borrowed(_)));
    state.counts.files += 1;
    state
        .string::<serde_json::Error>("a/c\"", Context::Path, false, None)
        .unwrap();
    assert!(matches!(state.previous_path, Cow::Owned(_)));
    state.counts.files += 1;
    state
        .string::<serde_json::Error>("a/d", Context::Path, false, Some("a/d"))
        .unwrap();
    assert!(matches!(state.previous_path, Cow::Borrowed(_)));
    assert_eq!(state.counts.path_components, 6);
    assert_eq!(state.counts.unique_path_nodes, 4);
}

#[test]
fn carries_the_same_profile_for_broad_plain_and_escaped_content() {
    let plain = serde_json::json!({"files": (0..256).map(|index| {
        serde_json::json!({"entryType":"file", "path":format!("f{index:04}"), "contentType":"x".repeat(1024)})
    }).collect::<Vec<_>>()});
    let counts = inspect_value(&plain);
    assert_eq!(counts.files, 256);
    assert_eq!(counts.allocation_profile.max_escaped_string_bytes, 0);
    assert_eq!(counts.allocation_profile.max_unquoted_run_bytes, 0);
    assert!(!counts.allocation_profile.numeric_heap_candidate);
    let mut escaped = plain;
    escaped["files"][0]["contentType"] = serde_json::Value::String("\u{1}".repeat(1024));
    let counts = inspect_value(&escaped);
    assert_eq!(counts.allocation_profile.max_escaped_string_bytes, 6 * 1024);
    assert_eq!(counts.files, 256);
}

#[test]
fn synthetic_numeric_strings_keep_full_conversion_overlap_funded() {
    let number = format!("1.{}1", "0".repeat(1024));
    let stored = serde_json::from_str::<serde_json::Value>(&number)
        .unwrap()
        .to_string();
    for bytes in [
        format!("{{\"unknown\":{number}}}"),
        format!("{{\"files\":[{{\"entryType\":\"file\",\"path\":\"a\",\"size\":{number}}}]}}"),
        format!("{{\"layers\":[{{\"runtimeConfig\":{{\"timeoutMs\":{number}}}}}]}}"),
    ] {
        let counts = inspect(bytes.as_bytes(), 1).unwrap();
        assert!(counts.allocation_profile.numeric_heap_candidate);
        assert_eq!(
            counts.allocation_profile.max_unquoted_run_bytes,
            number.len() as u64
        );
        // AP's synthetic map value is Context::Other even inside file/layer metadata.
        if stored.len() >= number.len() {
            assert!(counts.max_error_string_bytes >= stored.len() as u64);
            assert!(
                crate::source_bundle_v2::memory::verification_reservation(&counts).unwrap()
                    >= 18 * stored.len() as u64
            );
        }
    }
}

#[test]
fn many_synthetic_numeric_strings_fund_aggregate_retained_capacity() {
    let number = format!("1.{}1", "0".repeat(1024));
    let stored = serde_json::from_str::<serde_json::Value>(&number)
        .unwrap()
        .to_string();
    let bytes = format!("{{\"unknown\":[{}]}}", vec![number; 128].join(","));
    let counts = inspect(bytes.as_bytes(), 0).unwrap();
    if stored.len() >= 1027 {
        assert_eq!(counts.owned_parser_string_bytes, 128 * stored.len() as u64);
        assert_eq!(counts.max_error_string_bytes, stored.len() as u64);
    } else {
        assert_eq!(counts.owned_parser_string_bytes, 0);
    }
}
