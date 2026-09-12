use super::*;
use axum::{Json, Router, http::StatusCode, routing::post};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

const SESSION: &str = "00000000-0000-0000-0000-000000000001";
const PROJECT: &str = "00000000-0000-0000-0000-000000000002";
const DEPLOYMENT: &str = "00000000-0000-0000-0000-000000000003";

fn session(status: &str) -> Value {
    json!({
        "id":SESSION, "projectId":PROJECT, "deploymentId":DEPLOYMENT, "attempt":2,
        "source":"LOCAL_CLI", "status":status, "shippingPolicy":"ENABLED",
        "nextSeq":1, "acceptedBytes":128, "terminalMessage":null, "terminalErrorCode":null,
        "terminalErrorDetails":null, "failurePhase":null, "startedAt":"2026-09-12T00:00:00Z",
        "finishedAt":null, "diagnosticPath":"/diagnostics"
    })
}

async fn server(app: Router) -> (ApiClient, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (client, task)
}

#[tokio::test]
async fn nullable_session_phase_is_accepted_and_attempt_binding_is_checked() {
    for returned_attempt in [2, 3] {
        let app = Router::new().route("/v1/build-log-sessions/", post(move |Json(body): Json<Value>| async move {
            assert_eq!(body, json!({"id":SESSION, "projectId":PROJECT, "deploymentId":DEPLOYMENT,
                "attempt":2, "producerId":SESSION, "source":"LOCAL_CLI", "cliVersion":env!("CARGO_PKG_VERSION")}));
            let mut response = session("OPEN");
            response["attempt"] = json!(returned_attempt);
            Json(json!({"created":true, "session":response}))
        }));
        let (client, task) = server(app).await;
        let result = wire::create_session(
            &client,
            wire::SessionInput {
                id: SESSION,
                project_id: PROJECT,
                deployment_id: DEPLOYMENT,
                attempt: 2,
                producer_id: SESSION,
                source: BuildLogSource::LocalCli,
                builder_version: None,
            },
        )
        .await;
        if returned_attempt == 2 {
            let cursor = result.unwrap();
            assert_eq!(cursor.id, SESSION);
            assert_eq!(cursor.next_seq, 1);
            assert_eq!(cursor.accepted_bytes, 128);
            assert!(cursor.shipping_enabled);
        } else {
            assert!(result.is_err());
        }
        task.abort();
    }
}

fn event() -> BuildLogEvent {
    BuildLogEvent {
        seq: 0,
        timestamp: "2026-09-12T00:00:00Z".into(),
        stream: BuildLogStream::User,
        level: BuildLogLevel::Info,
        phase: BuildLogPhase::Build,
        message: "Compiling".into(),
        origin: BuildLogOrigin::ChildStdout,
    }
}

#[tokio::test]
async fn event_retry_resends_the_exact_batch_and_uses_server_acknowledgement() {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let recorded = Arc::clone(&requests);
    let app = Router::new().route(
        "/v1/build-log-sessions/{id}/events",
        post(move |Json(body): Json<Value>| {
            let requests = Arc::clone(&recorded);
            async move {
                let mut requests = requests.lock().unwrap();
                requests.push(body);
                if requests.len() == 1 {
                    (
                        StatusCode::TOO_MANY_REQUESTS,
                        [("retry-after", "0")],
                        Json(json!({"message":"retry"})),
                    )
                } else {
                    (
                        StatusCode::OK,
                        [("retry-after", "0")],
                        Json(json!({"accepted":true, "nextSeq":1, "replayed":true})),
                    )
                }
            }
        }),
    );
    let (client, task) = server(app).await;
    upload_batch(&client, SESSION, &[event()]).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
    assert_eq!(
        requests[0],
        json!({"events":[{"seq":0, "timestamp":"2026-09-12T00:00:00Z",
        "stream":"USER", "level":"INFO", "phase":"BUILD", "message":"Compiling", "origin":"CHILD_STDOUT"}]})
    );
    task.abort();
}

#[tokio::test]
async fn incompatible_log_cursor_fails_instead_of_dropping_events() {
    let app = Router::new().route(
        "/v1/build-log-sessions/{id}/events",
        post(|| async { Json(json!({"accepted":true, "nextSeq":99, "replayed":false})) }),
    );
    let (client, task) = server(app).await;
    let error = upload_batch(&client, SESSION, &[event()])
        .await
        .unwrap_err();
    assert!(error.to_string().contains("cursor 99 instead of 1"));
    task.abort();
}

#[tokio::test]
async fn observation_timeout_sends_finished_without_failure_metadata() {
    let app = Router::new().route(
        "/v1/build-log-sessions/{id}/finish",
        post(|Json(body): Json<Value>| async move {
            assert_eq!(
                body,
                json!({"status":"FINISHED", "message":"wait stopped [REDACTED]"})
            );
            Json(json!({"session":session("FINISHED")}))
        }),
    );
    let (client, task) = server(app).await;
    let error = crate::errors::CliError::new("DEPLOY_WAIT_TIMEOUT", "wait stopped sensitive-value")
        .into_anyhow();
    let redactor = ExactValueRedactor::from_values(["sensitive-value".into()]).unwrap();
    let request = BuildLogOutcome::ObservationStopped {
        success: BuildLogSuccess::ArtifactsUploaded,
        error: &error,
    }
    .prepare(BuildLogPhase::Activate, &redactor, None);
    finish_session(&client, SESSION, &request).await.unwrap();
    task.abort();
}
