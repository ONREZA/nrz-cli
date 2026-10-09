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

#[test]
fn declarations_and_intents_admit_only_compatible_runtime_targets() {
    for family in [
        ApplicationRuntimeFamily::Bun,
        ApplicationRuntimeFamily::Node,
        ApplicationRuntimeFamily::Python,
        ApplicationRuntimeFamily::Executable,
    ] {
        let declaration = ApplicationRuntimeDeclaration {
            family,
            python_version: None,
            entry: Some("app/main".into()),
            args: Vec::new(),
        };
        for target in [
            None,
            Some("bun-1.4.2"),
            Some("bun-2.0.0"),
            Some("node-22"),
            Some("node-24"),
            Some("node-26"),
            Some("node-23"),
            Some("python-3.12"),
            Some("python-3.13"),
            Some("python-3.14"),
            Some("python-3.15"),
            Some("native-linux-x86_64-glibc"),
            Some("native-linux-aarch64-glibc"),
        ] {
            let allowed = match family {
                ApplicationRuntimeFamily::Bun => matches!(target, None | Some("bun-1.4.2")),
                ApplicationRuntimeFamily::Node => {
                    matches!(target, Some("node-22" | "node-24" | "node-26"))
                }
                ApplicationRuntimeFamily::Python => {
                    matches!(target, Some("python-3.12" | "python-3.13" | "python-3.14"))
                }
                ApplicationRuntimeFamily::Executable => target == Some("native-linux-x86_64-glibc"),
            };
            assert_eq!(
                declaration.validate_target(target).is_ok(),
                allowed,
                "{family:?}/{target:?}"
            );
            assert_eq!(
                declaration.intent().validate_target(target).is_ok(),
                allowed
            );
        }
    }
}

#[test]
fn validation_preserves_launch_argument_and_entry_byte_boundaries() {
    let mut declaration = ApplicationRuntimeDeclaration {
        family: ApplicationRuntimeFamily::Bun,
        python_version: None,
        entry: None,
        args: Vec::new(),
    };
    for (args, allowed) in [
        (Vec::new(), true),
        (vec![String::new(); 64], true),
        (vec![String::new(); 65], false),
        (vec!["a".repeat(4096)], true),
        (vec!["a".repeat(4097)], false),
        (vec!["я".repeat(2048)], true),
        (vec!["я".repeat(2049)], false),
        (vec!["argument\0suffix".into()], false),
    ] {
        declaration.args = args;
        assert_eq!(declaration.validate().is_ok(), allowed);
        assert_eq!(declaration.validate_target(None).is_ok(), allowed);
        assert_eq!(declaration.intent().validate().is_ok(), allowed);
    }
    declaration.args.clear();
    for (entry, allowed) in [
        ("a".repeat(4096), true),
        ("a".repeat(4097), false),
        ("я".repeat(2048), true),
        ("я".repeat(2049), false),
        ("app:main".into(), false),
        ("app\0main".into(), false),
    ] {
        declaration.entry = Some(entry);
        assert_eq!(declaration.validate().is_ok(), allowed);
        assert_eq!(declaration.validate_target(None).is_ok(), allowed);
    }
}

#[test]
fn source_runtime_summary_preserves_consistent_and_mixed_layer_families() {
    for (families, expected) in [
        (vec![], None),
        (vec!["BUN"], Some(ApplicationRuntimeFamily::Bun)),
        (vec!["NODE", "NODE"], Some(ApplicationRuntimeFamily::Node)),
        (
            vec!["PYTHON", "PYTHON"],
            Some(ApplicationRuntimeFamily::Python),
        ),
        (
            vec!["EXECUTABLE"],
            Some(ApplicationRuntimeFamily::Executable),
        ),
        (vec!["BUN", "NODE"], None),
        (vec!["BUN", "NODE", "BUN"], None),
        (vec!["PYTHON", "EXECUTABLE"], None),
    ] {
        let mut layers = families
            .iter()
            .enumerate()
            .map(|(index, family)| {
                json!({
                    "name": format!("app-{index}"), "target": "COMPUTE",
                    "runtimeConfig": {"applicationRuntime": {"family":family, "args":[]}}
                })
            })
            .collect::<Vec<_>>();
        layers.push(json!({"name":"legacy", "target":"COMPUTE"}));
        layers.push(json!({"name":"static", "target":"STATIC"}));
        let manifest = serde_json::from_value(json!({
            "schemaVersion":crate::SOURCE_BUNDLE_V1_SCHEMA_VERSION, "layers": layers
        }))
        .unwrap();

        assert_eq!(
            source_application_runtime(&manifest).unwrap(),
            expected,
            "{families:?}"
        );
    }
    let manifest = serde_json::from_value(json!({
        "schemaVersion":crate::SOURCE_BUNDLE_V1_SCHEMA_VERSION,
        "layers":[{"name":"static", "target":"STATIC", "runtimeConfig":{
            "applicationRuntime":{"family":"BUN", "args":[]}
        }}]
    }))
    .unwrap();
    assert!(source_application_runtime(&manifest).is_err());
}
