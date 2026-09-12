use super::*;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

const DEPLOYMENT: &str = "00000000-0000-0000-0000-000000000001";

async fn serve(app: Router) -> (ApiClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (client, server)
}

#[tokio::test]
async fn source_registration_retries_the_same_operation_and_snapshot() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app = Router::new().route("/v1/deployments/{id}/source", post(
        |State(requests):State<Arc<Mutex<Vec<Value>>>>, Json(body):Json<Value>| async move {
            let mut received = requests.lock().unwrap();
            received.push(body);
            if received.len() == 1 {
                (StatusCode::TOO_MANY_REQUESTS, [("retry-after","0")], Json(json!({"message":"busy"}))).into_response()
            } else { Json(json!({"id":DEPLOYMENT,"status":"UPLOADING","url":null})).into_response() }
        })).with_state(Arc::clone(&requests));
    let (client, server) = serve(app).await;
    register_deployment_source(&client, DEPLOYMENT, 3,
        json!({"version":1,"layers":[{"name":"static","target":"STATIC","directory":"."}],"routes":[{"pattern":"^/.*","layer":"static"}]}), None, true
    ).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
    assert_eq!(requests[0]["attempt"], 3);
    assert_eq!(
        requests[0]["operationId"],
        Uuid::new_v5(&DEPLOYMENT.parse().unwrap(), b"onreza:deployment-source:3").to_string()
    );
    server.abort();
}

#[tokio::test]
async fn skipped_build_requires_acceptance_for_the_requested_deployment() {
    for response in [
        json!({"id":DEPLOYMENT,"accepted":false}),
        json!({"id":Uuid::nil(),"accepted":true}),
    ] {
        let app = Router::new().route("/v1/deployments/{id}/execution-context/skip-before-source", post(move |Json(body):Json<Value>| {
            let response = response.clone();
            async move {
                assert_eq!(body, json!({"protocolVersion":"execution-context-v2","attempt":2,"reason":"unchanged"}));
                Json(response)
            }
        }));
        let (client, server) = serve(app).await;
        assert!(
            mark_pre_source_skipped(&client, DEPLOYMENT, 2, "unchanged")
                .await
                .unwrap_err()
                .to_string()
                .contains("deployment state changed")
        );
        server.abort();
    }
}

#[tokio::test]
async fn pre_source_failure_uploads_redacted_diagnostics_and_attempt() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let app = Router::new().route("/v1/deployments/{id}/execution-context/fail-before-source", post(
        |State(requests):State<Arc<Mutex<Vec<Value>>>>, Json(body):Json<Value>| async move {
            requests.lock().unwrap().push(body);
            Json(json!({"id":DEPLOYMENT,"accepted":true}))
        })).with_state(Arc::clone(&requests));
    let (client, server) = serve(app).await;
    let error = crate::errors::CliError::new("BUILD_FAILED", "failed using secret-token")
        .details(json!({"lastError":null,"value":"secret-token"}))
        .into_anyhow();
    let redactor = ExactValueRedactor::from_values(["secret-token".to_string()]).unwrap();
    report_pre_source_failure(
        Some(&client),
        DEPLOYMENT,
        4,
        PreSourceFailureCode::BuildFailed,
        Some(&error),
        Some(&redactor),
        true,
    )
    .await;
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["attempt"], 4);
    assert_eq!(
        requests[0]["diagnostic"]["details"],
        json!({"lastError":null,"value":"[REDACTED]"})
    );
    assert!(!requests[0].to_string().contains("secret-token"));
    server.abort();
}

fn context_json() -> Value {
    json!({"workspaceId":"00000000-0000-0000-0000-000000000002", "workspaceSlug":"workspace",
        "projectId":"00000000-0000-0000-0000-000000000003", "projectName":"project",
        "environmentId":"00000000-0000-0000-0000-000000000004", "environmentName":"Production",
        "environmentType":"PRODUCTION", "sourceRef":null, "selectionSource":"EXPLICIT"})
}

#[tokio::test]
async fn admission_preserves_selection_and_rejects_another_scope() {
    for wrong_scope in [false, true] {
        let app = Router::new().route("/v1/projects/{id}/deployments/admit", post(move |Json(body):Json<Value>| async move {
            assert_eq!(body,json!({"protocolVersion":"execution-context-v2", "environmentId":context_json()["environmentId"],
                "selectionSource":"EXPLICIT","branch":"feature/a","commitSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}));
            let mut context = context_json();
            if wrong_scope { context["workspaceId"] = json!(Uuid::nil()); }
            Json(json!({"protocolVersion":"execution-context-v2","context":context,
                "deployment":{"id":DEPLOYMENT,"attempt":2,"status":"BUILDING","url":""},
                "snapshot":{"fingerprint":format!("v1:{}","a".repeat(64)),"resolvedAt":"2026-09-12T00:00:00Z"}}))
        }));
        let (client, server) = serve(app).await;
        let context: crate::execution_context::ExecutionContext =
            serde_json::from_value(context_json()).unwrap();
        let result = wire::admit(
            &client,
            &context.project_id,
            &context,
            "feature/a".into(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        )
        .await;
        if wrong_scope {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("another workspace")
            );
        } else {
            let result = result.unwrap();
            assert_eq!(result.deployment.attempt, 2);
            assert_eq!(result.deployment.id, DEPLOYMENT);
        }
        server.abort();
    }
}

#[tokio::test]
async fn runner_context_validates_identity_and_protocol_before_applying_settings() {
    for (id, protocol) in [
        (DEPLOYMENT, "runner-context-v4"),
        (DEPLOYMENT, "runner-context-v99"),
        ("00000000-0000-0000-0000-000000000009", "runner-context-v4"),
    ] {
        let app = Router::new().route("/v1/deployments/{id}/runner-context", axum::routing::get(move || async move {
            Json(json!({"protocolVersion":protocol,"context":context_json(),
                "deployment":{"id":id,"attempt":5,"status":"BUILDING","url":null,"branch":"main","commitSha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
                "settings":{"frameworkPreset":null,"rootDirectory":".","gitLfsEnabled":false,"packageManager":"BUN",
                    "installCommand":null,"installCommandSource":"PRESET","buildCommand":"bun run build","buildCommandSource":"USER",
                    "outputDirectory":null,"outputDirectorySource":"DETECTED","ignoredBuildBehavior":"AUTOMATIC","ignoredBuildFolder":null,"ignoredBuildCommand":null}}))
        }));
        let (client, server) = serve(app).await;
        let result = wire::load_runner_context(&client, DEPLOYMENT.parse().unwrap()).await;
        if id != DEPLOYMENT {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("another deployment")
            );
        } else if protocol != "runner-context-v4" {
            assert!(result.unwrap_err().to_string().contains("OpenAPI contract"));
        } else {
            let result = result.unwrap();
            assert_eq!(result.deployment.attempt, 5);
            assert_eq!(
                result.settings.build_command.as_deref(),
                Some("bun run build")
            );
        }
        server.abort();
    }
}
