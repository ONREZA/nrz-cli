use nrz_api::{ProjectRequestBody2, Token200Response, User200Response};
use serde_json::json;

#[test]
fn project_patch_preserves_omission_null_and_value() {
    let mut body: ProjectRequestBody2 = serde_json::from_value(json!({})).unwrap();
    assert_eq!(serde_json::to_value(&body).unwrap(), json!({}));
    body.git_url = Some(None);
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        json!({"gitUrl": null})
    );
    body.git_url = Some(Some("https://github.com/example/project".to_string()));
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        json!({"gitUrl": "https://github.com/example/project"})
    );
}

#[test]
fn token_defaults_do_not_hide_missing_required_fields() {
    for body in [json!({}), json!({"access_token": "secret"})] {
        assert!(serde_json::from_value::<Token200Response>(body).is_err());
    }
}

#[test]
fn user_response_nullability_matches_the_server_schema() {
    let valid = json!({
        "id": "00000000-0000-0000-0000-000000000001", "email": "test@example.com",
        "name": "Test", "username": null
    });
    let user: User200Response = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(user).unwrap(), valid);
    let mut missing = valid;
    missing.as_object_mut().unwrap().remove("username");
    assert!(serde_json::from_value::<User200Response>(missing).is_err());
}

#[test]
fn deployment_diagnostics_preserve_recursive_json_nulls() {
    let response = json!({
        "id":"00000000-0000-0000-0000-000000000001", "status":"failed", "url":null,
        "production":false, "error":"failed", "errorCode":"BUILD_FAILED",
        "errorDetails":{"runtimeStartupFailure":{"lastError":null}, "values":[null, "text"]},
        "runtimeArtifactGraphDigest":null, "runtimeArtifactGraph":null,
        "createdAt":"2026-09-12T00:00:00Z", "readyAt":null
    });
    let parsed: nrz_api::Status200Response = serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), response);
}

#[test]
fn function_invocation_preserves_runtime_extensions_and_recursive_json_values() {
    let response = json!({
        "invocation": { "invocationId": "request-1", "ok": false,
            "protocolVersion": "onreza-functions-poc/v2",
            "error": { "cause": null }, "logs": [{ "message": "failed", "properties": { "value": null } }] },
        "debugTrace": { "stages": [], "serverTiming": null },
        "revision": { "id": "revision-1", "functionId": "function-1", "sourceSnapshotId": "snapshot-1" }
    });
    let parsed: nrz_api::FunctionTestInvokeResponse =
        serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), response);
    assert!(
        serde_json::from_value::<nrz_api::FunctionTestInvokeResponse>(json!({
            "invocation": { "ok":true }, "debugTrace":null, "revision":{}
        }))
        .is_err()
    );
}

#[test]
fn invocation_headers_are_fixed_pairs_of_strings() {
    let request: nrz_api::TestInvokeRequestBody = serde_json::from_value(json!({
        "headers":[["accept","application/json"]]
    }))
    .unwrap();
    assert_eq!(
        request.headers.unwrap(),
        vec![("accept".to_owned(), "application/json".to_owned())]
    );
    for pair in [
        json!([]),
        json!(["name"]),
        json!(["name", "value", "extra"]),
        json!(["name", 2]),
    ] {
        assert!(
            serde_json::from_value::<nrz_api::TestInvokeRequestBody>(json!({"headers":[pair]}))
                .is_err()
        );
    }
}
