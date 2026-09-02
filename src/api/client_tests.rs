use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Redirect;
use axum::routing::get;

use super::client::{build_api_http_client, extract_api_error};
use super::{path_segment, query_value};

#[test]
fn api_components_cannot_change_path_or_query_structure() {
    assert_eq!(
        path_segment("project/../other?admin=true#x"),
        "project%2F%2E%2E%2Fother%3Fadmin%3Dtrue%23x"
    );
    assert_eq!(
        query_value("project&admin=true#x"),
        "project%26admin%3Dtrue%23x"
    );
}

#[tokio::test]
async fn api_client_does_not_follow_redirects() {
    let app = Router::new()
        .route("/start", get(|| async { Redirect::temporary("/target") }))
        .route("/target", get(|| async { "unexpected" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = build_api_http_client(HeaderMap::new()).unwrap();

    let response = client
        .get(format!("http://{addr}/start"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    handle.abort();
}

#[test]
fn structured_api_error_uses_retry_after_header_hint() {
    let error = extract_api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"code":"SERVICE_UNAVAILABLE","message":"overloaded"}"#,
        Some(2),
    );
    let structured = error
        .downcast_ref::<super::client::StructuredApiError>()
        .expect("structured API error");

    assert_eq!(structured.retry_after_seconds, Some(2));
}

#[test]
fn structured_api_error_accepts_camel_case_retry_after_body_hint() {
    let error = extract_api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"code":"SERVICE_UNAVAILABLE","message":"overloaded","retryAfterSeconds":3}"#,
        None,
    );
    let structured = error
        .downcast_ref::<super::client::StructuredApiError>()
        .expect("structured API error");

    assert_eq!(structured.retry_after_seconds, Some(3));
}

#[test]
fn structured_api_error_accepts_numeric_code() {
    let error = extract_api_error(
        StatusCode::BAD_REQUEST,
        r#"{"code":1234,"message":"invalid project"}"#,
        None,
    );
    let structured = error
        .downcast_ref::<super::client::StructuredApiError>()
        .expect("structured API error");

    assert_eq!(structured.code, "1234");
    assert_eq!(structured.message, "invalid project");
}
