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

#[tokio::test]
async fn recovered_unconditional_put_409_observes_committed_object_before_replay() {
    let bytes = Bytes::from_static(b"committed source whose PUT response was lost");
    let sha256 = sha256_hex(&bytes);
    let state = ObjectState {
        put_status: StatusCode::CONFLICT,
        head_checksum: Some(
            base64::engine::general_purpose::STANDARD.encode(hex::decode(&sha256).unwrap()),
        ),
        head_length: Some(bytes.len() as u64),
        observed: Arc::new(Mutex::new(Vec::new())),
    };
    let first = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let observed = state.observed.clone();
    let (base_url, handle) = serve(Router::new().route(
        "/object",
        any(move |request: Request| {
            let state = state.clone();
            let first = first.clone();
            async move {
                let lose_response = request.method() == Method::PUT
                    && first.swap(false, std::sync::atomic::Ordering::SeqCst);
                let response = object_handler(State(state), request).await;
                if lose_response {
                    std::future::pending::<()>().await;
                }
                response
            }
        }),
    ))
    .await;
    let upload_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();
    let transport = HttpSourcePublicationTransport::from_clients(
        &base_url,
        reqwest::Client::new(),
        upload_client,
    );
    let mut request = object_request(&base_url, bytes, sha256);
    request.headers.if_none_match = None;

    let result = transport
        .put_object(request)
        .await
        .expect("409 must reconcile the committed recovered upload");

    assert_eq!(result.e_tag, None);
    let observed = observed.lock().unwrap();
    assert_eq!(
        observed
            .iter()
            .map(|r| r.method.clone())
            .collect::<Vec<_>>(),
        vec![Method::PUT, Method::PUT, Method::HEAD]
    );
    assert!(
        observed
            .iter()
            .filter(|r| r.method == Method::PUT)
            .all(|r| !r.headers.contains_key(IF_NONE_MATCH))
    );
    handle.abort();
}

#[tokio::test]
async fn recovered_source_put_409_repairs_confirmed_incomplete_object() {
    for mismatch in ["size", "checksum"] {
        let bytes = Bytes::from(vec![b'x'; 46]);
        let mut state = conflict_state(&bytes);
        if mismatch == "size" {
            state.head_length = Some(4);
            state.head_checksum = None;
        } else {
            state.head_checksum = Some(
                base64::engine::general_purpose::STANDARD
                    .encode(hex::decode(sha256_hex(b"part")).unwrap()),
            );
        }
        let observed = state.observed.clone();
        let (url, handle) = serve_scripted_object(
            state,
            vec![StatusCode::CONFLICT, StatusCode::OK],
            StatusCode::OK,
            false,
        )
        .await;
        let mut request = object_request(&url, bytes.clone(), sha256_hex(&bytes));
        request.headers.if_none_match = None;

        let result = transport(&url)
            .put_object(request)
            .await
            .expect("authorized source overwrite must retry after a known partial object");

        assert_eq!(result.e_tag.as_deref(), Some("fixture-etag"));
        assert_exact_payload_replayed_after_head(&observed, &bytes, None);
        handle.abort();
    }
}

#[tokio::test]
async fn conditional_put_409_observes_matching_object_without_replay() {
    let bytes = Bytes::from_static(b"conditional source");
    let state = conflict_state(&bytes);
    let observed = state.observed.clone();
    let (url, handle) = serve_object(state).await;
    let result = transport(&url)
        .put_object(object_request(&url, bytes.clone(), sha256_hex(&bytes)))
        .await
        .unwrap();
    assert_eq!(result.e_tag, None);
    assert_eq!(methods(&observed), vec![Method::PUT, Method::HEAD]);
    handle.abort();
}

#[tokio::test]
async fn put_409_retries_only_after_confirmed_head_absence() {
    let bytes = Bytes::from_static(b"absent source");
    let state = conflict_state(&bytes);
    let observed = state.observed.clone();
    let (url, handle) = serve_scripted_object(
        state,
        vec![StatusCode::CONFLICT, StatusCode::OK],
        StatusCode::NOT_FOUND,
        false,
    )
    .await;
    let result = transport(&url)
        .put_object(object_request(&url, bytes.clone(), sha256_hex(&bytes)))
        .await
        .unwrap();
    assert_eq!(result.e_tag.as_deref(), Some("fixture-etag"));
    assert_exact_payload_replayed_after_head(&observed, &bytes, Some("*"));
    handle.abort();
}

#[tokio::test]
async fn put_409_with_persistent_absence_or_authorized_mismatch_exhausts_finite_attempts() {
    for known_mismatch in [false, true] {
        let bytes = Bytes::from_static(b"still incomplete source");
        let mut state = conflict_state(&bytes);
        if known_mismatch {
            state.head_length = Some(4);
        }
        let observed = state.observed.clone();
        let (url, handle) = serve_scripted_object(
            state,
            vec![StatusCode::CONFLICT],
            if known_mismatch {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            },
            false,
        )
        .await;
        let mut request = object_request(&url, bytes.clone(), sha256_hex(&bytes));
        if known_mismatch {
            request.headers.if_none_match = None;
        }
        let error = transport(&url).put_object(request).await.unwrap_err();
        assert!(
            matches!(error,SourcePublicationError::ObjectUpload(ref message) if message.contains("exhausted after 8 attempts"))
        );
        assert_eq!(
            methods(&observed),
            [Method::PUT, Method::HEAD]
                .into_iter()
                .cycle()
                .take(16)
                .collect::<Vec<_>>()
        );
        handle.abort();
    }
}

#[tokio::test]
async fn put_409_mismatched_or_missing_metadata_never_accepts_or_overwrites() {
    for conditional in [false, true] {
        for mismatch in ["size", "checksum", "missing-checksum", "malformed-checksum"] {
            if !conditional && matches!(mismatch, "size" | "checksum") {
                continue;
            }
            let bytes = Bytes::from_static(b"expected source");
            let mut state = conflict_state(&bytes);
            match mismatch {
                "size" => state.head_length = Some(bytes.len() as u64 + 1),
                "checksum" => {
                    state.head_checksum =
                        Some(base64::engine::general_purpose::STANDARD.encode([0_u8; 32]))
                }
                "malformed-checksum" => state.head_checksum = Some("not-base64".to_string()),
                _ => state.head_checksum = None,
            }
            let observed = state.observed.clone();
            let (url, handle) = serve_object(state).await;
            let mut request = object_request(&url, bytes.clone(), sha256_hex(&bytes));
            if !conditional {
                request.headers.if_none_match = None;
            }
            let error = transport(&url).put_object(request).await.unwrap_err();
            assert_eq!(
                matches!(error, SourcePublicationError::ConditionalUploadConflict(_)),
                conditional && matches!(mismatch, "size" | "checksum")
            );
            assert_eq!(methods(&observed), vec![Method::PUT, Method::HEAD]);
            handle.abort();
        }
    }
}

#[tokio::test]
async fn put_409_transient_or_timed_out_head_never_replays_put() {
    for timeout in [false, true] {
        let bytes = Bytes::from_static(b"uncertain source");
        let state = conflict_state(&bytes);
        let observed = state.observed.clone();
        let (url, handle) = serve_scripted_object(
            state,
            vec![StatusCode::CONFLICT],
            if timeout {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            timeout,
        )
        .await;
        let upload_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();
        let transport = HttpSourcePublicationTransport::from_clients(
            &url,
            reqwest::Client::new(),
            upload_client,
        );
        let mut request = object_request(&url, bytes.clone(), sha256_hex(&bytes));
        request.headers.if_none_match = None;
        let error = transport.put_object(request).await.unwrap_err();
        assert!(
            matches!(error, SourcePublicationError::ObjectUpload(ref message) if message.contains("HEAD exhausted after 4 attempts"))
        );
        let requests = methods(&observed);
        if timeout {
            // A timed-out request can fail before the server receives it.
            assert_eq!(requests.first(), Some(&Method::PUT));
            assert!((2..=5).contains(&requests.len()));
            assert!(requests[1..].iter().all(|method| *method == Method::HEAD));
        } else {
            assert_eq!(
                requests,
                vec![
                    Method::PUT,
                    Method::HEAD,
                    Method::HEAD,
                    Method::HEAD,
                    Method::HEAD
                ]
            );
        }
        handle.abort();
    }
}

#[tokio::test]
async fn put_409_without_verify_head_preserves_failure_and_part_etag_requirement() {
    let bytes = Bytes::from_static(b"multipart part");
    let state = conflict_state(&bytes);
    let observed = state.observed.clone();
    let (url, handle) = serve_object(state).await;
    let mut request = object_request(&url, bytes.clone(), sha256_hex(&bytes));
    request.verify_head = None;
    let error = transport(&url).put_object(request).await.unwrap_err();
    assert!(matches!(error, SourcePublicationError::ObjectUpload(_)));
    assert_eq!(methods(&observed), vec![Method::PUT]);
    handle.abort();
}

#[tokio::test]
async fn runtime_file_409_reuses_head_but_never_takes_source_overwrite_recovery() {
    use nrz_source_publisher::{RuntimeArtifactPublicationTransport, RuntimeFileUploadRequest};
    use std::io::Write;
    for known_mismatch in [false, true] {
        let bytes = Bytes::from_static(b"runtime dependency image");
        let mut state = conflict_state(&bytes);
        if known_mismatch {
            state.head_length = Some(4);
        }
        let observed = state.observed.clone();
        let (url, handle) = serve_object(state).await;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&bytes).unwrap();
        let mut request = object_request(&url, bytes.clone(), sha256_hex(&bytes));
        if known_mismatch {
            request.headers.if_none_match = None;
        }
        let result = transport(&url)
            .put_runtime_file(RuntimeFileUploadRequest {
                url: request.url,
                path: file.path().to_path_buf(),
                content_length: bytes.len() as u64,
                sha256: request.sha256,
                headers: request.headers,
                verify_head: request.verify_head,
            })
            .await;
        if known_mismatch {
            assert!(matches!(
                result,
                Err(SourcePublicationError::ObjectUpload(_))
            ));
        } else {
            assert_eq!(result.unwrap().e_tag, None);
        }
        assert_eq!(methods(&observed), vec![Method::PUT, Method::HEAD]);
        handle.abort();
    }
}

fn assert_exact_payload_replayed_after_head(
    observed: &Mutex<Vec<ObservedRequest>>,
    bytes: &Bytes,
    if_none_match: Option<&str>,
) {
    assert_eq!(
        methods(observed),
        vec![Method::PUT, Method::HEAD, Method::PUT]
    );
    for request in observed
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.method == Method::PUT)
    {
        assert_eq!(request.body, *bytes);
        assert_eq!(
            request
                .headers
                .get(IF_NONE_MATCH)
                .map(|v| v.to_str().unwrap()),
            if_none_match
        );
    }
}

fn conflict_state(bytes: &Bytes) -> ObjectState {
    ObjectState {
        put_status: StatusCode::CONFLICT,
        head_checksum: Some(
            base64::engine::general_purpose::STANDARD
                .encode(hex::decode(sha256_hex(bytes)).unwrap()),
        ),
        head_length: Some(bytes.len() as u64),
        observed: Arc::new(Mutex::new(Vec::new())),
    }
}
fn methods(observed: &Mutex<Vec<ObservedRequest>>) -> Vec<Method> {
    observed
        .lock()
        .unwrap()
        .iter()
        .map(|r| r.method.clone())
        .collect()
}
async fn serve_scripted_object(
    state: ObjectState,
    put_statuses: Vec<StatusCode>,
    head_status: StatusCode,
    stall_head: bool,
) -> (String, tokio::task::JoinHandle<()>) {
    let statuses = Arc::new(Mutex::new(std::collections::VecDeque::from(put_statuses)));
    serve(Router::new().route(
        "/object",
        any(move |request: Request| {
            let state = state.clone();
            let statuses = statuses.clone();
            async move {
                let is_put = request.method() == Method::PUT;
                let mut response = object_handler(State(state), request).await;
                if is_put {
                    let mut statuses = statuses.lock().unwrap();
                    *response.status_mut() = if statuses.len() > 1 {
                        statuses.pop_front().unwrap()
                    } else {
                        statuses[0]
                    };
                } else {
                    if stall_head {
                        std::future::pending::<()>().await;
                    }
                    *response.status_mut() = head_status;
                }
                response
            }
        }),
    ))
    .await
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
