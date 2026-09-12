use axum::{Json, Router, http::HeaderMap, routing::post};
use nrz_api::{UploadComplete200Response, UploadCompleteRequestBody};
use nrz_source_publisher::{HttpSourcePublicationTransport, SourcePublicationTransport};
use serde_json::{Value, json};
use uuid::Uuid;

fn request() -> UploadCompleteRequestBody {
    UploadCompleteRequestBody {
        deployment_id: Uuid::now_v7(),
        upload_session_id: Uuid::now_v7(),
        deployment_attempt_id: Uuid::now_v7(),
        operation_id: Uuid::now_v7(),
        source_artifact_id: "a".repeat(64),
        source_sha256: "b".repeat(64),
        source_size_bytes: "1024".into(),
        logical_manifest_sha256: "c".repeat(64),
        ..Default::default()
    }
}

async fn complete(
    request: &UploadCompleteRequestBody,
    response: Value,
) -> Result<UploadComplete200Response, nrz_source_publisher::SourcePublicationError> {
    let expected = serde_json::to_value(request).unwrap();
    let app = Router::new().route(
        &format!("/v1/deployments/{}/upload-complete", request.deployment_id),
        post(
            move |headers: HeaderMap, Json(body): Json<Value>| async move {
                assert_eq!(headers["x-api-key"], "fixture-token");
                assert_eq!(body, expected);
                Json(response)
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let transport = HttpSourcePublicationTransport::authenticated(
        format!("http://{address}"),
        b"fixture-token",
        "nrz-test",
    )
    .unwrap();
    let result = transport
        .complete_upload(request.deployment_id, request)
        .await;
    server.abort();
    result
}

#[tokio::test]
async fn distinguishes_completion_states_with_identical_fields() {
    let request = request();
    for kind in [
        "source-upload-completed",
        "source-fast-path-completed",
        "source-verified-awaiting-runtime",
    ] {
        let result = complete(
            &request,
            json!({
                "kind": kind,
                "deploymentId": request.deployment_id,
                "uploadSessionId": request.upload_session_id,
            }),
        )
        .await
        .unwrap();
        match (kind, result) {
            ("source-upload-completed", UploadComplete200Response::Object(_))
            | ("source-fast-path-completed", UploadComplete200Response::Object2(_))
            | ("source-verified-awaiting-runtime", UploadComplete200Response::Object3(_)) => {}
            unexpected => panic!("wrong completion state: {unexpected:?}"),
        }
    }
}

#[tokio::test]
async fn rejects_unknown_and_missing_discriminators_without_echoing_response_data() {
    let request = request();
    for kind in [Some("untrusted-secret-kind"), None] {
        let mut response = json!({"deploymentId": request.deployment_id, "uploadSessionId": request.upload_session_id});
        if let Some(kind) = kind {
            response["kind"] = json!(kind);
        }
        let error = complete(&request, response).await.unwrap_err().to_string();
        assert!(error.contains("OpenAPI contract"));
        assert!(!error.contains("untrusted-secret-kind"));
    }
}

#[tokio::test]
async fn rejects_completion_for_another_deployment_or_session() {
    let request = request();
    for (deployment_id, upload_session_id) in [
        (Uuid::now_v7(), request.upload_session_id),
        (request.deployment_id, Uuid::now_v7()),
    ] {
        let error = complete(&request, json!({"kind": "source-upload-completed", "deploymentId": deployment_id, "uploadSessionId": upload_session_id})).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("another deployment or upload session")
        );
    }
}

#[tokio::test]
async fn incomplete_and_expired_responses_remain_distinct_from_success() {
    let request = request();
    let incomplete = complete(
        &request,
        json!({"kind": "incomplete", "missingSourceObject": true}),
    )
    .await
    .unwrap();
    assert!(
        matches!(incomplete, UploadComplete200Response::Object5(value) if value.missing_source_object)
    );
    let expired = complete(&request, json!({"kind": "expired", "deploymentId": request.deployment_id, "expiredAt": "2026-09-01T12:00:00Z"})).await.unwrap();
    assert!(matches!(expired, UploadComplete200Response::Object4(_)));
}
