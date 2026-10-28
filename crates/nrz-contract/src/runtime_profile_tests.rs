use super::RuntimeProfile;

#[test]
fn legacy_and_arbitrary_numeric_profiles_retain_their_serialized_identity() {
    for profile in [
        "BUN",
        "EXECUTABLE",
        "NODE_22",
        "NODE_24",
        "NODE_26",
        "CPYTHON_3_12",
        "CPYTHON_3_13",
        "CPYTHON_3_14",
        "NODE_20",
        "NODE_23",
        "NODE_99",
        "CPYTHON_3_11",
        "CPYTHON_3_99",
        "CPYTHON_4_0",
    ] {
        let bytes = format!("\"{profile}\"");
        let value: RuntimeProfile = serde_json::from_str(&bytes).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), bytes);
    }
    assert_eq!(
        "NODE_24".parse::<RuntimeProfile>().unwrap(),
        RuntimeProfile::Node24
    );
    assert_eq!(
        "CPYTHON_3_14".parse::<RuntimeProfile>().unwrap(),
        RuntimeProfile::Cpython314
    );
}

#[test]
fn malformed_profiles_and_unrepresentable_programmatic_values_are_rejected() {
    for profile in [
        "NODE_01",
        "NODE_-1",
        "NODE_1.2",
        "NODE_1000000000",
        "CPYTHON_3_01",
        "CPYTHON_3_11_1",
        "CPYTHON_3",
        "RUBY_3",
        "NODE_23\0",
    ] {
        assert!(profile.parse::<RuntimeProfile>().is_err(), "{profile:?}");
    }
    assert!(serde_json::to_string(&RuntimeProfile::Node(u32::MAX)).is_err());
    assert!(serde_json::to_string(&RuntimeProfile::Cpython(3, u32::MAX)).is_err());
}

#[test]
fn family_only_profiles_roundtrip_without_changing_known_identities() {
    for (target, profile) in [
        ("node", "NODE"),
        ("python", "CPYTHON"),
        ("node-23.1.0", "NODE_23"),
        ("python-3.11.9", "CPYTHON_3_11"),
    ] {
        let parsed: RuntimeProfile = serde_json::from_value(serde_json::json!(profile)).unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::json!(profile)
        );
        let target_profile = if target.starts_with("node") {
            RuntimeProfile::node_build_target(target)
        } else {
            RuntimeProfile::python_build_target(target)
        };
        assert_eq!(target_profile, Some(parsed));
    }
}
