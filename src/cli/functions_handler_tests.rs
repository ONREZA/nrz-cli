use base64::Engine;
use serde_json::json;

use crate::cli::functions::FunctionsInvokeArgs;

use super::functions_handler::{
    FunctionInvokeResponse, build_test_invoke_request, render_response_body,
};

#[test]
fn invoke_request_without_payload_is_get_root() {
    let request = build_test_invoke_request(&invoke_args()).unwrap();

    assert_eq!(request["method"], "GET");
    assert_eq!(request["path"], "/");
    assert_eq!(request["host"], "test-invoke.onreza.internal");
    assert_eq!(request["headers"], json!([]));
    assert!(request.get("bodyBase64").is_none());
}

#[test]
fn invoke_request_with_payload_posts_compact_json_body() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("payload.json");
    std::fs::write(&path, "{\n  \"ok\": true\n}\n").unwrap();

    let request = build_test_invoke_request(&FunctionsInvokeArgs {
        payload: Some(path.display().to_string()),
        ..invoke_args()
    })
    .unwrap();
    let body = base64::engine::general_purpose::STANDARD
        .decode(request["bodyBase64"].as_str().unwrap())
        .unwrap();

    assert_eq!(request["method"], "POST");
    assert_eq!(
        request["headers"],
        json!([["content-type", "application/json"]])
    );
    assert_eq!(body, br#"{"ok":true}"#);
}

#[test]
fn invoke_request_accepts_full_fetch_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let body_path = tmp.path().join("body.txt");
    let debug_path = tmp.path().join("debug.json");
    std::fs::write(&body_path, "hello").unwrap();
    std::fs::write(&debug_path, r#"{"waitUntilMode":"drain"}"#).unwrap();

    let request = build_test_invoke_request(&FunctionsInvokeArgs {
        method: Some("PATCH".to_string()),
        path: Some("/api/run".to_string()),
        query_string: Some("a=1&b=2".to_string()),
        host: Some("example.test".to_string()),
        headers: vec![
            "Authorization: Bearer test".to_string(),
            "X-Trace: abc".to_string(),
        ],
        body: Some(body_path.display().to_string()),
        debug: Some(debug_path.display().to_string()),
        ..invoke_args()
    })
    .unwrap();

    assert_eq!(request["method"], "PATCH");
    assert_eq!(request["path"], "/api/run");
    assert_eq!(request["queryString"], "a=1&b=2");
    assert_eq!(request["host"], "example.test");
    assert_eq!(
        request["headers"],
        json!([["Authorization", "Bearer test"], ["X-Trace", "abc"]])
    );
    assert_eq!(request["bodyBase64"], "aGVsbG8=");
    assert!(request.get("event").is_none());
    assert_eq!(request["debug"], json!({"waitUntilMode": "drain"}));
}

#[test]
fn invoke_request_accepts_event_surface_without_fetch_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let event_path = tmp.path().join("event.json");
    let debug_path = tmp.path().join("debug.json");
    std::fs::write(
        &event_path,
        r#"{"type":"manual","event":{"reason":"smoke"}}"#,
    )
    .unwrap();
    std::fs::write(&debug_path, r#"{"waitUntilMode":"drain"}"#).unwrap();

    let request = build_test_invoke_request(&FunctionsInvokeArgs {
        event: Some(event_path.display().to_string()),
        debug: Some(debug_path.display().to_string()),
        ..invoke_args()
    })
    .unwrap();

    assert_eq!(request["method"], "GET");
    assert_eq!(request["path"], "/");
    assert_eq!(request["host"], "test-invoke.onreza.internal");
    assert_eq!(request["headers"], json!([]));
    assert!(request.get("bodyBase64").is_none());
    assert_eq!(
        request["event"],
        json!({"type": "manual", "event": {"reason": "smoke"}})
    );
    assert_eq!(request["debug"], json!({"waitUntilMode": "drain"}));
}

#[test]
fn invoke_request_rejects_event_with_fetch_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let event_path = tmp.path().join("event.json");
    std::fs::write(
        &event_path,
        r#"{"type":"manual","event":{"reason":"smoke"}}"#,
    )
    .unwrap();

    let error = build_test_invoke_request(&FunctionsInvokeArgs {
        event: Some(event_path.display().to_string()),
        path: Some("/api/run".to_string()),
        headers: vec!["X-Trace: abc".to_string()],
        ..invoke_args()
    })
    .unwrap_err();

    let text = format!("{error:#}");
    assert!(text.contains("--event cannot be combined"));
    assert!(text.contains("--path"));
    assert!(text.contains("--header"));
}

#[test]
fn invoke_request_rejects_header_without_colon() {
    let error = build_test_invoke_request(&FunctionsInvokeArgs {
        headers: vec!["broken".to_string()],
        ..invoke_args()
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("expected 'Name: value'"));
}

#[test]
fn invoke_request_rejects_invalid_body_base64() {
    let error = build_test_invoke_request(&FunctionsInvokeArgs {
        body_base64: Some("not-base64!".to_string()),
        ..invoke_args()
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("bodyBase64 must be standard base64"));
}

#[test]
fn invoke_request_rejects_oversized_body_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("body.bin");
    std::fs::write(&path, vec![b'a'; 1_048_577]).unwrap();

    let error = build_test_invoke_request(&FunctionsInvokeArgs {
        body: Some(path.display().to_string()),
        ..invoke_args()
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("request body must be at most 1 MiB"));
}

#[test]
fn invoke_request_rejects_path_without_leading_slash() {
    let error = build_test_invoke_request(&FunctionsInvokeArgs {
        path: Some("api/run".to_string()),
        ..invoke_args()
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains("path must start with /"));
}

#[test]
fn render_response_body_decodes_utf8_body_base64() {
    let response = FunctionInvokeResponse {
        status: Some(200),
        headers: Vec::new(),
        body_base64: Some("aGVsbG8=".to_string()),
        body_preview: None,
    };

    assert_eq!(render_response_body(&response).as_deref(), Some("hello"));
}

fn invoke_args() -> FunctionsInvokeArgs {
    FunctionsInvokeArgs {
        name: "api".to_string(),
        dir: ".".to_string(),
        project_id: None,
        env: None,
        method: None,
        path: None,
        query_string: None,
        host: None,
        headers: Vec::new(),
        payload: None,
        body: None,
        body_base64: None,
        event: None,
        debug: None,
    }
}
