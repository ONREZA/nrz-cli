use super::*;
use serde_json::json;

#[test]
fn executable_intent_rejects_managed_runtime_family_labels() {
    for family in ["JAVASCRIPT", "PYTHON"] {
        let config = json!({
            "applicationRuntime":{"family":"EXECUTABLE","args":[]},
            "runtimeFamily":family,
            "isBinaryEntry":true,
            "buildRuntimeVersion":"native-linux-x86_64-glibc"
        });
        assert!(
            layer_application_runtime(Some(&config)).is_err(),
            "EXECUTABLE accepted legacy runtimeFamily={family}"
        );
    }
}

#[test]
fn legacy_family_consistency_preserves_all_supported_application_families() {
    for (family, compatible) in [
        ("NODE", Some("JAVASCRIPT")),
        ("BUN", Some("JAVASCRIPT")),
        ("PYTHON", Some("PYTHON")),
        ("EXECUTABLE", None),
    ] {
        let mut config = json!({"applicationRuntime":{"family":family,"args":[]}});
        assert!(layer_application_runtime(Some(&config)).unwrap().is_some());
        for label in [
            json!("JAVASCRIPT"),
            json!("PYTHON"),
            json!("RUBY"),
            json!(7),
        ] {
            config["runtimeFamily"] = label.clone();
            assert_eq!(
                layer_application_runtime(Some(&config)).is_ok(),
                label.as_str().is_some() && label.as_str() == compatible,
                "{family} / {label}"
            );
        }
    }
    // DEPRECATED: untyped binary entries retain their target-less launch path.
    assert!(
        layer_application_runtime(Some(&json!({"isBinaryEntry":true})))
            .unwrap()
            .is_none()
    );
}
