use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Redirect;
use axum::routing::get;

use super::client::{build_api_http_client, extract_api_error};

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

#[test]
fn unrecognized_server_errors_keep_retry_policy_and_body() {
    for body in [
        "upstream unavailable",
        r#"{"message":"upstream unavailable"}"#,
    ] {
        let error = extract_api_error(StatusCode::BAD_GATEWAY, body, Some(17));
        let structured = error.downcast_ref::<super::StructuredApiError>().unwrap();
        assert_eq!(structured.status, StatusCode::BAD_GATEWAY);
        assert_eq!(structured.message, "upstream unavailable");
        assert_eq!(
            super::classify_api_retry(&error).unwrap().retry_after,
            Some(std::time::Duration::from_secs(17))
        );
    }
}

#[tokio::test]
async fn generated_user_operation_checks_the_response_contract() {
    let response = serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000001", "email": "test@example.com",
        "name": "Test", "username": null
    });
    let app = Router::new().route(
        "/v1/user",
        get(move |headers: HeaderMap| {
            let response = response.clone();
            async move {
                assert_eq!(headers["x-api-key"], "test-key");
                axum::Json(response)
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", "test-key".parse().unwrap());
    let client = super::ApiClient::with_http_client(
        format!("http://{address}"),
        build_api_http_client(headers).unwrap(),
    )
    .unwrap();
    let user = client.user().await.unwrap();
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test");
    assert!(user.username.is_none());
    server.abort();
}
