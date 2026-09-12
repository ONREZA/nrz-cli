use nrz_api::{FunctionHandler, FunctionPublishSpec, functions::validate_publication};
use serde_json::json;

fn fixture() -> FunctionPublishSpec {
    serde_json::from_value(json!({
        "source": {"path": "jobs/api.nrz-fn.ts", "contentText": "throw new Error('not executed by data validation')"},
        "declaration": {"name": "job", "triggers": [{"name": "run", "type": "manual", "config": {"nested": [null, false, 0]}}]},
        "handlers": ["manual"]
    })).unwrap()
}

#[test]
fn accepts_evaluated_metadata_and_preserves_json_values() {
    let spec = fixture();
    validate_publication(&spec).unwrap();
    let wire = serde_json::to_value(spec).unwrap();
    assert_eq!(
        wire["declaration"]["triggers"][0]["config"],
        json!({"nested": [null, false, 0.0]})
    );
}

#[test]
fn rejects_invalid_or_missing_handler_bindings() {
    for handlers in [
        vec![],
        vec![FunctionHandler::Fetch],
        vec![FunctionHandler::Manual, FunctionHandler::Manual],
    ] {
        let mut spec = fixture();
        spec.handlers = handlers;
        assert!(validate_publication(&spec).is_err());
    }
    let mut spec = fixture();
    spec.declaration
        .triggers
        .push(spec.declaration.triggers[0].clone());
    assert!(validate_publication(&spec).is_err());
}

#[test]
fn rejects_noncanonical_paths_and_byte_limit_bypasses() {
    for path in [
        "/api.nrz-fn.ts",
        "../api.nrz-fn.ts",
        "dir/../api.nrz-fn.ts",
        "dir//api.nrz-fn.ts",
        "./api.nrz-fn.ts",
        "C:/api.nrz-fn.ts",
        "dir\\api.nrz-fn.ts",
        "node_modules/api.nrz-fn.ts",
        "api.ts",
    ] {
        let mut spec = fixture();
        spec.source.path = path.to_string();
        assert!(validate_publication(&spec).is_err(), "{path}");
    }
    let mut spec = fixture();
    spec.source.content_text = "я".repeat(70_000);
    assert!(validate_publication(&spec).is_err());
}

#[test]
fn rejects_invalid_metadata_and_source_only_wire_body() {
    for declaration in [
        json!({"name": "invalid name", "triggers": []}),
        json!({"name": "job", "triggers": [{"name": " ", "type": "manual"}]}),
        json!({"name": "job", "triggers": [{"name": " run", "type": "manual"}]}),
    ] {
        let mut spec = fixture();
        spec.declaration = serde_json::from_value(declaration).unwrap();
        assert!(validate_publication(&spec).is_err());
    }
    assert!(
        serde_json::from_value::<FunctionPublishSpec>(
            json!({"source": {"path": "api.nrz-fn.ts", "contentText": ""}})
        )
        .is_err()
    );
}
