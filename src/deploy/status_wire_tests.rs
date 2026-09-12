use super::activation::{ActivationWait, is_activation_observation_error, wait_for_activation};
use crate::api::ApiClient;
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::time::Duration;

const DEPLOYMENT: &str = "00000000-0000-0000-0000-000000000001";

fn status() -> Value {
    json!({"id":DEPLOYMENT, "status":"failed", "url":null, "production":false,
        "error":"Build failed", "errorCode":"BUILD_FAILED", "errorDetails":null,
        "runtimeArtifactGraphDigest":null, "runtimeArtifactGraph":null,
        "createdAt":"2026-09-12T00:00:00Z", "readyAt":null})
}

#[tokio::test]
async fn generated_status_preserves_failure_authority_despite_unrecognized_diagnostics() {
    let mut failed = status();
    failed["errorDetails"] = json!({"runtimeStartupFailure":{"expectedPort":"unknown"}});
    let mut mismatched = status();
    mismatched["id"] = json!("00000000-0000-0000-0000-000000000002");
    for (body, expected, observation) in [
        (failed, "DEPLOY_FAILED", false),
        (mismatched, "DEPLOY_STATUS_INVALID", true),
        (
            json!({"id":DEPLOYMENT,"status":"live"}),
            "DEPLOY_STATUS_UNAVAILABLE",
            true,
        ),
    ] {
        let app = Router::new().route(
            "/v1/deployments/{id}/status",
            get(move || {
                let body = body.clone();
                async move { Json(body) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = ApiClient::with_http_client(
            format!("http://{}", listener.local_addr().unwrap()),
            reqwest::Client::new(),
        )
        .unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let error = wait_for_activation(
            ActivationWait {
                deployment_id: DEPLOYMENT,
                url: "https://example.test",
                timeout: Duration::from_secs(2),
            },
            || async { client.deployment_status(DEPLOYMENT).await?.try_into() },
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(
            crate::errors::find_cli_error(&error).unwrap().code,
            expected
        );
        assert_eq!(is_activation_observation_error(&error), observation);
        server.abort();
    }
}
