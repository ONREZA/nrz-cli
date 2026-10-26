use super::*;
use std::ffi::OsStr;

#[test]
fn the_installed_v2_profile_is_selected_without_environment_overrides() {
    validate_values(None, None).unwrap();
    validate_values(
        Some(OsStr::new("V2")),
        Some(OsStr::new(APPLICATION_EROFS_PROFILE_V1)),
    )
    .unwrap();
}

#[test]
fn legacy_and_misconfigured_transfers_fail_without_a_fallback() {
    for (transfer, profile, code) in [
        (Some("V1"), None, "CLI_UPDATE_REQUIRED"),
        (Some(""), None, "INVALID_ARGUMENT"),
        (Some("unknown"), None, "INVALID_ARGUMENT"),
        (None, Some(""), "INVALID_ARGUMENT"),
        (None, Some("uninstalled-profile"), "INVALID_ARGUMENT"),
    ] {
        let error = validate_values(transfer.map(OsStr::new), profile.map(OsStr::new)).unwrap_err();
        assert_eq!(
            error.downcast_ref::<output::CodedError>().unwrap().code,
            code
        );
    }
}

fn admission_body() -> nrz_api::AdmitRequestBody {
    serde_json::from_value(serde_json::json!({
        "deploymentId":"00000000-0000-4000-8000-000000000001",
        "operationId":"00000000-0000-4000-8000-000000000002",
        "cliIdempotencyKey":"00000000-0000-4000-8000-000000000003",
        "cliAttemptId":"00000000-0000-4000-8000-000000000004",
        "protocolVersion":2,"packageProfile":APPLICATION_EROFS_PROFILE_V1,
        "manifest":{"algorithm":"BLAKE3","value":"ab".repeat(32)},
        "manifestSizeBytes":100,"fileCount":2,"logicalStaticBytes":18,
        "artifactBytes":0,"maxFileSize":11,"cliEndpointId":"cd".repeat(32)
    }))
    .unwrap()
}

#[tokio::test]
async fn only_an_initial_owner_quota_refusal_releases_pre_source_custody() {
    use axum::{Json, Router, http::StatusCode, routing::post};
    use std::sync::{Arc, Mutex};
    for (statuses, code, protected) in [
        (vec![StatusCode::FORBIDDEN], "LIMIT_EXCEEDED", false),
        (
            vec![StatusCode::SERVICE_UNAVAILABLE, StatusCode::FORBIDDEN],
            "LIMIT_EXCEEDED",
            true,
        ),
        (vec![StatusCode::CONFLICT], "CONFLICT", true),
        (vec![StatusCode::BAD_REQUEST], "VALIDATION_ERROR", true),
        (vec![StatusCode::FORBIDDEN], "UNRECOGNIZED_REFUSAL", true),
    ] {
        let expected_requests = statuses.len();
        let statuses = Arc::new(Mutex::new(statuses.into_iter()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new().route("/v1/deployments/{id}/static-source-v2/admit", post({
            let requests = requests.clone();
            move |Json(body):Json<serde_json::Value>| {
                requests.lock().unwrap().push(body);
                let status = statuses.lock().unwrap().next().expect("unexpected admission retry");
                async move { (status, Json(serde_json::json!({"code":code,"message":"owner refusal","retryAfterSeconds":0}))) }
            }
        }));
        let (client, server) = crate::test_support::serve_api(app).await;
        let mut protected_custody = false;
        let request = admission_body();
        admit(
            &client,
            &request.deployment_id.to_string(),
            &request,
            &mut protected_custody,
        )
        .await
        .unwrap_err();
        assert_eq!(protected_custody, protected, "{code}");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), expected_requests);
        assert!(requests.iter().all(|body| body == &requests[0]));
        server.abort();
    }
}

#[tokio::test]
async fn timeout_before_a_quota_refusal_keeps_unknown_custody() {
    use axum::{Json, Router, http::StatusCode, routing::post};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let requests = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(tokio::sync::Notify::new());
    let app = Router::new().route(
        "/v1/deployments/{id}/static-source-v2/admit",
        post({
            let requests = requests.clone();
            let entered = entered.clone();
            move || {
                let first = requests.fetch_add(1, Ordering::SeqCst) == 0;
                let entered = entered.clone();
                async move {
                    if first {
                        entered.notify_one();
                        std::future::pending::<()>().await;
                    }
                    (
                        StatusCode::FORBIDDEN,
                        Json(serde_json::json!({"code":"LIMIT_EXCEEDED","message":"quota"})),
                    )
                }
            }
        }),
    );
    let (client, server) = crate::test_support::serve_api(app).await;
    let task = tokio::spawn(async move {
        let request = admission_body();
        let mut protected = false;
        admit(
            &client,
            &request.deployment_id.to_string(),
            &request,
            &mut protected,
        )
        .await
        .unwrap_err();
        protected
    });
    entered.notified().await;
    tokio::time::pause();
    tokio::time::advance(SOURCE_REGISTRATION_REQUEST_TIMEOUT).await;
    // Resume real network scheduling after the request deadline has fired.
    tokio::time::resume();
    assert!(task.await.unwrap());
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    server.abort();
}

#[tokio::test]
async fn lost_transport_reply_before_a_quota_refusal_keeps_unknown_custody() {
    use axum::{Json, Router, http::StatusCode, routing::post};
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = ApiClient::with_http_client(
        format!("http://{}", listener.local_addr().unwrap()),
        reqwest::Client::new(),
    )
    .unwrap();
    let server = tokio::spawn(async move {
        let (mut connection, _) = listener.accept().await.unwrap();
        assert!(connection.read(&mut [0; 4096]).await.unwrap() > 0);
        // A sent request loses its reply before the caller can learn custody.
        drop(connection);
        let app = Router::new().route(
            "/v1/deployments/{id}/static-source-v2/admit",
            post(|| async {
                (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"code":"LIMIT_EXCEEDED","message":"quota"})),
                )
            }),
        );
        axum::serve(listener, app).await.unwrap();
    });
    let request = admission_body();
    let mut protected = false;
    let error = tokio::time::timeout(
        Duration::from_secs(5),
        admit(
            &client,
            &request.deployment_id.to_string(),
            &request,
            &mut protected,
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<crate::api::StructuredApiError>()
            .unwrap()
            .code,
        "LIMIT_EXCEEDED"
    );
    assert!(protected);
    server.abort();
}

#[test]
fn reconciliation_accepts_only_closed_published_without_replacing_input() {
    let mut response = nrz_api::Admit200Response::default();
    for (state, reason, published) in [
        (nrz_api::Admit200ResponseState::Active, None, false),
        (
            nrz_api::Admit200ResponseState::Active,
            Some("PUBLISHED"),
            false,
        ),
        (
            nrz_api::Admit200ResponseState::Closed,
            Some("EXPIRED"),
            false,
        ),
        (
            nrz_api::Admit200ResponseState::Closed,
            Some("PUBLISHED"),
            true,
        ),
    ] {
        response.state = state;
        response.terminal_reason = reason.map(str::to_owned);
        assert_eq!(require_published(&response).is_ok(), published);
    }
}

#[tokio::test]
async fn reconciliation_admit_keeps_exact_body_and_original_deadline() {
    use axum::{Json, Router, http::StatusCode, routing::post};
    use std::sync::{Arc, Mutex};
    let received = Arc::new(Mutex::new(Vec::new()));
    let entered = Arc::new(tokio::sync::Notify::new());
    let app = Router::new().route(
        "/v1/deployments/{id}/static-source-v2/admit",
        post({
            let received = received.clone();
            let entered = entered.clone();
            move |Json(body): Json<serde_json::Value>| {
                received.lock().unwrap().push(body);
                let entered = entered.clone();
                async move {
                    entered.notify_one();
                    std::future::pending::<(StatusCode, Json<serde_json::Value>)>().await
                }
            }
        }),
    );
    let (client, server) = crate::test_support::serve_api(app).await;
    let request = admission_body();
    let expected = serde_json::to_value(&request).unwrap();
    let task = tokio::spawn(async move {
        let mut protected = true;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        let error = admit_before(
            &client,
            &request.deployment_id.to_string(),
            &request,
            &mut protected,
            deadline,
        )
        .await
        .unwrap_err();
        (protected, error)
    });
    entered.notified().await;
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(60)).await;
    let (protected, error) = task.await.unwrap();
    assert!(protected);
    assert!(error.to_string().contains("admission deadline"));
    let received = received.lock().unwrap();
    assert_eq!(received.as_slice(), &[expected]);
    server.abort();
}

#[test]
fn historical_publication_must_preserve_original_receiver_custody() {
    let original = nrz_api::Admit200Response {
        receiver_session_id: Uuid::from_u128(1),
        receiver_instance_id: Uuid::from_u128(2),
        receiver_endpoint_id: "ab".repeat(32),
        ..Default::default()
    };
    let mut published = original.clone();
    published.state = nrz_api::Admit200ResponseState::Closed;
    published.terminal_reason = Some("PUBLISHED".into());
    validate_receiver_custody(&published, &original).unwrap();
    require_published(&published).unwrap();
    for field in 0..3 {
        let mut changed = published.clone();
        match field {
            0 => changed.receiver_session_id = Uuid::from_u128(3),
            1 => changed.receiver_instance_id = Uuid::from_u128(3),
            _ => changed.receiver_endpoint_id = "cd".repeat(32),
        }
        assert!(validate_receiver_custody(&changed, &original).is_err());
    }
}
