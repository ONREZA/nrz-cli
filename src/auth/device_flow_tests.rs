use axum::{Json, Router, http::StatusCode, routing::post};
use serde_json::json;

use crate::api::ApiClient;

use super::device_flow::{poll_for_token, request_device_code};

async fn server(app: Router) -> (ApiClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client =
        ApiClient::with_http_client(format!("http://{address}"), reqwest::Client::new()).unwrap();
    (client, task)
}

#[tokio::test]
async fn device_timing_must_be_positive() {
    let app = Router::new().route(
        "/v1/device/",
        post(|| async {
            Json(json!({
                "device_code": "a".repeat(40), "user_code": "BCDF-GHJK",
                "verification_uri": "https://example.com/device",
                "verification_uri_complete": "https://example.com/device?code=BCDF-GHJK",
                "expires_in": 600, "interval": 0
            }))
        }),
    );
    let (client, task) = server(app).await;
    let error = request_device_code(&client).await.unwrap_err();
    assert!(error.to_string().contains("poll interval"), "{error:#}");
    task.abort();
}

#[tokio::test]
async fn token_poll_preserves_http_failures_and_rejects_incomplete_success() {
    for (status, body) in [
        (
            StatusCode::BAD_GATEWAY,
            json!({"error": "authorization_pending"}),
        ),
        (StatusCode::OK, json!({"access_token": "secret"})),
    ] {
        let app = Router::new().route(
            "/v1/device/token",
            post(move |Json(request): Json<serde_json::Value>| {
                let body = body.clone();
                async move {
                    assert_eq!(request["device_code"], "a".repeat(40));
                    assert_eq!(
                        request["grant_type"],
                        "urn:ietf:params:oauth:grant-type:device_code"
                    );
                    (status, Json(body))
                }
            }),
        );
        let (client, task) = server(app).await;
        let error = poll_for_token(&client, &"a".repeat(40), 1, 4)
            .await
            .unwrap_err();
        if status == StatusCode::BAD_GATEWAY {
            assert_eq!(
                error
                    .downcast_ref::<crate::api::StructuredApiError>()
                    .unwrap()
                    .status,
                status
            );
        } else {
            assert!(
                format!("{error:#}").contains("OpenAPI contract"),
                "{error:#}"
            );
        }
        task.abort();
    }
}
