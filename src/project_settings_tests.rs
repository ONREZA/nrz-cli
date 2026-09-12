use axum::{Json, Router, http::StatusCode, routing::get};
use serde_json::json;

use crate::api::ApiClient;
use crate::project_settings::{ProjectSettingsFetch, fetch_for_effective_config};

#[tokio::test]
async fn only_retryable_settings_failures_allow_fallback() {
    let project = "00000000-0000-0000-0000-000000000001";
    for (status, body, transient) in [
        (
            StatusCode::BAD_GATEWAY,
            json!({"message":"unavailable"}),
            true,
        ),
        (
            StatusCode::NOT_FOUND,
            json!({"message":"missing project"}),
            false,
        ),
        (StatusCode::OK, json!({"unexpected":"schema"}), false),
    ] {
        let app = Router::new().route(
            &format!("/v1/projects/{project}"),
            get(move || {
                let body = body.clone();
                async move { (status, Json(body)) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client =
            ApiClient::with_http_client(format!("http://{address}"), reqwest::Client::new())
                .unwrap();
        let result = fetch_for_effective_config(&client, project).await;
        if transient {
            assert!(matches!(
                result,
                Ok(ProjectSettingsFetch::TransientFailure { .. })
            ));
        } else {
            assert!(result.is_err());
        }
        server.abort();
    }
}
