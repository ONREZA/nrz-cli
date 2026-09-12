use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes as AxumBytes, to_bytes};
use axum::extract::{Request, State};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, Method, Response, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{any, get};
use axum::{Json, Router};
use base64::Engine as _;
use bytes::Bytes;
use nrz_source_bundle::sha256_hex;
use nrz_source_publisher::{
    HttpSourcePublicationTransport, ObjectHeadVerification, ObjectUploadHeaders,
    ObjectUploadRequest, SourcePublicationError, SourcePublicationTransport,
};
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
struct ObjectState {
    put_status: StatusCode,
    head_checksum: Option<String>,
    head_length: Option<u64>,
    observed: Arc<Mutex<Vec<ObservedRequest>>>,
}

#[derive(Debug)]
struct ObservedRequest {
    method: Method,
    headers: HeaderMap,
    body: AxumBytes,
}

#[tokio::test]
async fn conditional_put_forwards_the_signed_object_contract() {
    let bytes = Bytes::from_static(b"verified source bundle");
    let sha256 = sha256_hex(&bytes);
    let state = ObjectState {
        put_status: StatusCode::OK,
        head_checksum: None,
        head_length: None,
        observed: Arc::new(Mutex::new(Vec::new())),
    };
    let (base_url, handle) = serve_object(state.clone()).await;
    let transport = HttpSourcePublicationTransport::authenticated(
        &base_url,
        b"fixture-private-token",
        "nrz-test",
    )
    .unwrap();

    let result = transport
        .put_object(object_request(&base_url, bytes.clone(), sha256))
        .await
        .unwrap();

    assert_eq!(result.e_tag.as_deref(), Some("fixture-etag"));
    let observed = state.observed.lock().unwrap();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].method, Method::PUT);
    assert!(!observed[0].headers.contains_key("x-api-key"));
    assert!(!observed[0].headers.contains_key("authorization"));
    assert_eq!(observed[0].body, bytes);
    assert_eq!(
        observed[0].headers.get(CONTENT_TYPE).unwrap(),
        "application/zstd"
    );
    assert_eq!(observed[0].headers.get(IF_NONE_MATCH).unwrap(), "*");
    assert_eq!(
        observed[0]
            .headers
            .get("x-amz-checksum-sha256")
            .unwrap()
            .to_str()
            .unwrap(),
        base64::engine::general_purpose::STANDARD.encode(hex::decode(sha256_hex(&bytes)).unwrap())
    );
    handle.abort();
}

#[tokio::test]
async fn conditional_conflict_recovers_only_after_exact_head_readback() {
    let bytes = Bytes::from_static(b"already uploaded source bundle");
    let sha256 = sha256_hex(&bytes);
    let checksum = base64::engine::general_purpose::STANDARD.encode(hex::decode(&sha256).unwrap());
    let state = ObjectState {
        put_status: StatusCode::PRECONDITION_FAILED,
        head_checksum: Some(checksum),
        head_length: Some(bytes.len() as u64),
        observed: Arc::new(Mutex::new(Vec::new())),
    };
    let (base_url, handle) = serve_object(state.clone()).await;
    let transport = transport(&base_url);

    transport
        .put_object(object_request(&base_url, bytes, sha256))
        .await
        .unwrap();

    let observed = state.observed.lock().unwrap();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0].method, Method::PUT);
    assert_eq!(observed[1].method, Method::HEAD);
    handle.abort();
}

#[tokio::test]
async fn conditional_conflict_rejects_a_different_existing_object() {
    let bytes = Bytes::from_static(b"expected source bundle");
    let sha256 = sha256_hex(&bytes);
    let state = ObjectState {
        put_status: StatusCode::PRECONDITION_FAILED,
        head_checksum: Some(base64::engine::general_purpose::STANDARD.encode([0_u8; 32])),
        head_length: Some(bytes.len() as u64),
        observed: Arc::new(Mutex::new(Vec::new())),
    };
    let (base_url, handle) = serve_object(state).await;
    let transport = transport(&base_url);

    let error = transport
        .put_object(object_request(&base_url, bytes, sha256))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        SourcePublicationError::ConditionalUploadConflict(_)
    ));
    handle.abort();
}

#[tokio::test]
async fn control_plane_error_preserves_code_details_and_retry_hint() {
    let deployment_id = Uuid::now_v7();
    let app = Router::new().route(
        &format!("/v1/deployments/{deployment_id}/status"),
        get(|| async {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                [(RETRY_AFTER, "7")],
                Json(json!({
                    "code": "SERVICE_UNAVAILABLE",
                    "message": "temporarily unavailable",
                    "details": { "phase": "runtime-readback" }
                })),
            )
        }),
    );
    let (base_url, handle) = serve(app).await;
    let transport = transport(&base_url);

    let error = transport
        .deployment_status(deployment_id)
        .await
        .unwrap_err();
    let structured = error.structured_control_plane().unwrap();
    assert_eq!(structured.status, 503);
    assert_eq!(structured.code, "SERVICE_UNAVAILABLE");
    assert_eq!(structured.retry_after.unwrap().as_secs(), 7);
    assert_eq!(
        structured.details.as_ref().unwrap()["phase"],
        "runtime-readback"
    );
    handle.abort();
}

#[tokio::test]
async fn control_plane_client_does_not_follow_redirects() {
    let deployment_id = Uuid::now_v7();
    let app = Router::new()
        .route(
            &format!("/v1/deployments/{deployment_id}/status"),
            get(|| async { Redirect::temporary("/unexpected") }),
        )
        .route(
            "/unexpected",
            get(|| async {
                Json(json!({
                    "status": "SMOKE_TESTING",
                    "runtimeArtifactGraphDigest": "f".repeat(64),
                    "error": null,
                    "errorCode": null
                }))
            }),
        );
    let (base_url, handle) = serve(app).await;
    let transport = transport(&base_url);

    let error = transport
        .deployment_status(deployment_id)
        .await
        .unwrap_err();

    assert_eq!(error.structured_control_plane().unwrap().status, 307);
    handle.abort();
}

fn object_request(base_url: &str, bytes: Bytes, sha256: String) -> ObjectUploadRequest {
    ObjectUploadRequest {
        url: format!("{base_url}/object"),
        verify_head: Some(ObjectHeadVerification {
            url: format!("{base_url}/object"),
            content_length: bytes.len() as u64,
            sha256: sha256.clone(),
        }),
        bytes,
        sha256,
        headers: ObjectUploadHeaders {
            content_type: Some("application/zstd".to_string()),
            if_none_match: Some("*".to_string()),
        },
    }
}

fn transport(base_url: &str) -> HttpSourcePublicationTransport {
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let upload_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    HttpSourcePublicationTransport::from_clients(base_url, api_client, upload_client)
}

async fn serve_object(state: ObjectState) -> (String, tokio::task::JoinHandle<()>) {
    serve(
        Router::new()
            .route("/object", any(object_handler))
            .with_state(state),
    )
    .await
}

async fn object_handler(State(state): State<ObjectState>, request: Request) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, usize::MAX).await.unwrap();
    state.observed.lock().unwrap().push(ObservedRequest {
        method: parts.method.clone(),
        headers: parts.headers,
        body,
    });

    if parts.method == Method::PUT {
        let mut response = state.put_status.into_response();
        response
            .headers_mut()
            .insert(ETAG, HeaderValue::from_static("fixture-etag"));
        return response;
    }
    if parts.method == Method::HEAD {
        let mut response = StatusCode::OK.into_response();
        if let Some(length) = state.head_length {
            response.headers_mut().insert(
                CONTENT_LENGTH,
                HeaderValue::from_str(&length.to_string()).unwrap(),
            );
        }
        if let Some(checksum) = &state.head_checksum {
            response.headers_mut().insert(
                "x-amz-checksum-sha256",
                HeaderValue::from_str(checksum).unwrap(),
            );
        }
        return response;
    }
    StatusCode::METHOD_NOT_ALLOWED.into_response()
}

async fn serve(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), handle)
}
