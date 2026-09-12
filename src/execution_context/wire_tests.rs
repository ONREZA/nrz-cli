use super::*;
use axum::{Json, Router, routing::post};
use serde_json::{Value, json};

const PROJECT_ID: &str = "00000000-0000-0000-0000-000000000001";
const ENVIRONMENT_ID: &str = "00000000-0000-0000-0000-000000000002";
const DEPLOYMENT_ID: &str = "00000000-0000-0000-0000-000000000003";

fn context_json() -> Value {
    json!({"workspaceId":"00000000-0000-0000-0000-000000000004", "workspaceSlug":"workspace",
        "projectId":PROJECT_ID, "projectName":"project", "environmentId":ENVIRONMENT_ID,
        "environmentName":"Production", "environmentType":"PRODUCTION", "sourceRef":null, "selectionSource":"EXPLICIT"})
}

#[tokio::test]
async fn generated_execution_requests_preserve_selection_and_validate_snapshot_binding() {
    let app = Router::new()
        .route("/v1/projects/{id}/execution-context/resolve", post(|Json(body): Json<Value>| async move {
            assert_eq!(body, json!({"environment":"Production", "sourceRef":null, "selectionSource":"EXPLICIT"}));
            Json(json!({"protocolVersion":EXECUTION_CONTEXT_PROTOCOL, "context":context_json()}))
        }))
        .route("/v1/projects/{id}/execution-context/materialize", post(|Json(body): Json<Value>| async move {
            assert_eq!(body, json!({"environmentId":ENVIRONMENT_ID, "sourceRef":null, "purpose":"EXEC", "selectionSource":"EXPLICIT"}));
            Json(json!({"protocolVersion":EXECUTION_CONTEXT_PROTOCOL, "context":context_json(),
                "variables":{"TOKEN":"test-value"}, "secretKeys":["TOKEN"],
                "snapshot":{"fingerprint":format!("v1:{}", "a".repeat(64)), "resolvedAt":"2026-09-12T00:00:00Z",
                    "source":"DESIRED_STATE", "deploymentId":null}}))
        }))
        .route("/v1/deployments/{id}/execution-context/materialize", post(|Json(body): Json<Value>| async move {
            assert_eq!(body, json!({"purpose":"DEPLOY"}));
            Json(json!({"protocolVersion":EXECUTION_CONTEXT_PROTOCOL, "context":context_json(),
                "variables":{}, "secretKeys":[],
                "snapshot":{"fingerprint":format!("v1:{}", "a".repeat(64)), "resolvedAt":"2026-09-12T00:00:00Z",
                    "source":"DEPLOYMENT", "deploymentId":"00000000-0000-0000-0000-000000000099"}}))
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let context = resolve(&client, PROJECT_ID, "Production", None, "EXPLICIT")
        .await
        .unwrap();
    assert_eq!(context.environment_id, ENVIRONMENT_ID);
    let materialized = materialize_desired(&client, &context, None, "EXEC")
        .await
        .unwrap();
    assert_eq!(secret_values(&materialized), ["test-value"]);
    assert_eq!(materialized.snapshot.resolved_at, "2026-09-12T00:00:00Z");
    let error = materialize_deployment(&client, DEPLOYMENT_ID, "DEPLOY")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("ENV_SNAPSHOT_SCOPE_MISMATCH"));
    server.abort();
}
