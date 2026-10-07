use nrz_source_bundle::{
    ApplicationRuntimeDeclaration, BuildToolchainDeclaration, BuildToolchainFamily, PythonMinor,
    SourceBuildContext,
};
use serde_json::json;

#[test]
fn build_context_keeps_static_and_independent_serving_intent_separate() {
    for family in ["NODE", "BUN", "PYTHON", "NATIVE"] {
        for runtime in [
            serde_json::Value::Null,
            json!({"family":"NODE","entry":"server.js","args":[]}),
            json!({"family":"PYTHON","pythonVersion":"3.13","entry":"server.py","args":[]}),
            json!({"family":"EXECUTABLE","entry":"server","args":["two words"]}),
        ] {
            let mut value = json!({"schemaVersion":1,"buildToolchain":{"family":family},"applicationRuntime":runtime});
            if family == "PYTHON" {
                value["buildToolchain"]["pythonVersion"] = json!(PythonMinor::default().version());
            }
            let context: SourceBuildContext = serde_json::from_value(value.clone()).unwrap();
            context.validate().unwrap();
            assert_eq!(serde_json::to_value(&context).unwrap(), value);
            assert_eq!(
                context.build_toolchain.resolved_python_minor(),
                (family == "PYTHON").then(PythonMinor::default)
            );
        }
    }
    let context: SourceBuildContext = serde_json::from_value(json!({
        "schemaVersion":1,"buildToolchain":{"family":"PYTHON","pythonVersion":"3.12"},
        "applicationRuntime":{"family":"PYTHON","pythonVersion":"3.13","args":[]}
    }))
    .unwrap();
    context.validate().unwrap();
    assert_eq!(context.build_toolchain.family, BuildToolchainFamily::Python);
    assert_eq!(
        context.build_toolchain.resolved_python_minor(),
        Some(PythonMinor::Python312)
    );
}

#[test]
fn build_context_rejects_incomplete_unknown_and_invalid_declarations() {
    let valid = json!({"schemaVersion":1,"buildToolchain":{"family":"PYTHON","pythonVersion":"3.14"},"applicationRuntime":null});
    for key in ["schemaVersion", "buildToolchain", "applicationRuntime"] {
        let mut value = valid.clone();
        value.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<SourceBuildContext>(value).is_err());
    }
    for value in [
        json!({"schemaVersion":1,"buildToolchain":{"family":"python"},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"GO"},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"PYTHON","pythonVersion":"3.15"},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"PYTHON","extra":true},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"PYTHON"},"applicationRuntime":null,"extra":true}),
    ] {
        assert!(serde_json::from_value::<SourceBuildContext>(value).is_err());
    }
    // Authoring can omit a minor; frozen contexts must resolve it so future
    // catalog defaults cannot change build tools or the serving interpreter.
    let authored: BuildToolchainDeclaration =
        serde_json::from_value(json!({"family":"PYTHON"})).unwrap();
    authored.validate().unwrap();
    let authored: ApplicationRuntimeDeclaration =
        serde_json::from_value(json!({"family":"PYTHON","args":[]})).unwrap();
    authored.validate().unwrap();
    for value in [
        json!({"schemaVersion":1,"buildToolchain":{"family":"PYTHON"},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"NODE"},"applicationRuntime":{"family":"PYTHON","args":[]}}),
        json!({"schemaVersion":2,"buildToolchain":{"family":"PYTHON"},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"NODE","pythonVersion":"3.12"},"applicationRuntime":null}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"PYTHON"},"applicationRuntime":{"family":"NODE","pythonVersion":"3.12","args":[]}}),
        json!({"schemaVersion":1,"buildToolchain":{"family":"NATIVE"},"applicationRuntime":{"family":"EXECUTABLE","args":["\u{0}"]}}),
    ] {
        assert!(
            serde_json::from_value::<SourceBuildContext>(value)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    for entry in [
        "../server.ts",
        "/srv/server.ts",
        "./server.ts",
        "server\\entry.ts",
        "C:/server.ts",
    ] {
        let context: SourceBuildContext = serde_json::from_value(json!({"schemaVersion":1,"buildToolchain":{"family":"NODE"},"applicationRuntime":{"family":"NODE","entry":entry,"args":[]}})).unwrap();
        assert!(
            context.validate().is_err(),
            "unsafe serving entry admitted: {entry}"
        );
    }
    for entry in [
        None,
        Some("server.ts"),
        Some("src/server.ts"),
        Some("my server.ts"),
    ] {
        let context: SourceBuildContext = serde_json::from_value(json!({"schemaVersion":1,"buildToolchain":{"family":"NODE"},"applicationRuntime":{"family":"NODE","entry":entry,"args":[]}})).unwrap();
        context.validate().unwrap();
    }
}
