use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::put;
use bytes::Bytes;

use super::client::{
    ApiClient, ConditionalUploadConflict, PresignedHeadVerify, PresignedPutHeaders,
    UploadRetryPolicy, explain_s3_failure, extract_api_error, sha256_hex_to_base64,
};

#[derive(Clone)]
struct MockState {
    attempts: Arc<AtomicU32>,
    fail_times: u32,
    fail_status: StatusCode,
    retry_after: Option<&'static str>,
}

async fn handler(State(state): State<MockState>) -> impl IntoResponse {
    let n = state.attempts.fetch_add(1, Ordering::SeqCst);
    if n < state.fail_times {
        let mut response = (state.fail_status, "rate limited").into_response();
        if let Some(ra) = state.retry_after {
            response
                .headers_mut()
                .insert("Retry-After", ra.parse().unwrap());
        }
        response
    } else {
        (StatusCode::OK, "ok").into_response()
    }
}

async fn conditional_header_handler(headers: HeaderMap) -> impl IntoResponse {
    match headers
        .get("if-none-match")
        .and_then(|value| value.to_str().ok())
    {
        Some("*") => (StatusCode::OK, "ok"),
        _ => (StatusCode::BAD_REQUEST, "missing if-none-match"),
    }
}

async fn signed_source_headers_handler(headers: HeaderMap) -> impl IntoResponse {
    let has_content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        == Some("application/zstd");
    let has_if_none_match = headers
        .get("if-none-match")
        .and_then(|value| value.to_str().ok())
        == Some("*");

    if has_content_type && has_if_none_match {
        return (StatusCode::OK, "ok");
    }
    (
        StatusCode::BAD_REQUEST,
        "missing content-type or if-none-match",
    )
}

async fn precondition_failed_handler() -> impl IntoResponse {
    (StatusCode::PRECONDITION_FAILED, "exists")
}

async fn delayed_precondition_failed_handler() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_millis(10)).await;
    (StatusCode::PRECONDITION_FAILED, "exists")
}

async fn verify_head_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("16"));
    headers.insert(
        "x-amz-checksum-sha256",
        HeaderValue::from_str(&sha256_hex_to_base64(FAKE_SHA).unwrap()).unwrap(),
    );
    (StatusCode::OK, headers)
}

async fn verify_head_size_mismatch_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("15"));
    headers.insert(
        "x-amz-checksum-sha256",
        HeaderValue::from_str(&sha256_hex_to_base64(FAKE_SHA).unwrap()).unwrap(),
    );
    (StatusCode::OK, headers)
}

async fn transient_verify_head_handler(
    State(attempts): State<Arc<AtomicU32>>,
) -> impl IntoResponse {
    if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
        return (StatusCode::SERVICE_UNAVAILABLE, HeaderMap::new());
    }

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("16"));
    headers.insert(
        "x-amz-checksum-sha256",
        HeaderValue::from_str(&sha256_hex_to_base64(FAKE_SHA).unwrap()).unwrap(),
    );
    (StatusCode::OK, headers)
}

async fn initially_missing_verify_head_handler(
    State(attempts): State<Arc<AtomicU32>>,
) -> impl IntoResponse {
    if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
        return (StatusCode::NOT_FOUND, HeaderMap::new());
    }

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("16"));
    headers.insert(
        "x-amz-checksum-sha256",
        HeaderValue::from_str(&sha256_hex_to_base64(FAKE_SHA).unwrap()).unwrap(),
    );
    (StatusCode::OK, headers)
}

async fn spawn_mock(state: MockState) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/upload", put(handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_conditional_header_mock() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route("/upload", put(conditional_header_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_signed_source_headers_mock() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route("/upload", put(signed_source_headers_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_precondition_with_head_mock() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route(
        "/upload",
        put(precondition_failed_handler).head(verify_head_handler),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_precondition_with_mismatched_head_mock() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route(
        "/upload",
        put(precondition_failed_handler).head(verify_head_size_mismatch_handler),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_precondition_with_transient_head_mock(
    attempts: Arc<AtomicU32>,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/upload",
            put(precondition_failed_handler).head(transient_verify_head_handler),
        )
        .with_state(attempts);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_delayed_precondition_with_transient_head_mock(
    attempts: Arc<AtomicU32>,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/upload",
            put(delayed_precondition_failed_handler).head(transient_verify_head_handler),
        )
        .with_state(attempts);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

async fn spawn_precondition_with_initially_missing_head_mock(
    attempts: Arc<AtomicU32>,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route(
            "/upload",
            put(precondition_failed_handler).head(initially_missing_verify_head_handler),
        )
        .with_state(attempts);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/upload"), handle)
}

fn test_client() -> ApiClient {
    ApiClient::anonymous().expect("client build")
}

fn payload() -> Bytes {
    Bytes::from_static(&[0u8; 16])
}

/// 64 lowercase hex chars — valid input for `put_blob_with_policy`'s SHA-256
/// argument. The mock S3 doesn't verify the checksum header (it accepts any
/// PUT), so the value just needs to pass the upload-side format check.
const FAKE_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn structured_api_error_uses_retry_after_header_hint() {
    let error = extract_api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"code":"SERVICE_UNAVAILABLE","message":"overloaded"}"#,
        Some(2),
    );
    let structured = error
        .downcast_ref::<super::client::StructuredApiError>()
        .expect("structured API error");

    assert_eq!(structured.retry_after_seconds, Some(2));
}

#[test]
fn structured_api_error_accepts_camel_case_retry_after_body_hint() {
    let error = extract_api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        r#"{"code":"SERVICE_UNAVAILABLE","message":"overloaded","retryAfterSeconds":3}"#,
        None,
    );
    let structured = error
        .downcast_ref::<super::client::StructuredApiError>()
        .expect("structured API error");

    assert_eq!(structured.retry_after_seconds, Some(3));
}

#[test]
fn structured_api_error_accepts_numeric_code() {
    let error = extract_api_error(
        StatusCode::BAD_REQUEST,
        r#"{"code":1234,"message":"invalid project"}"#,
        None,
    );
    let structured = error
        .downcast_ref::<super::client::StructuredApiError>()
        .expect("structured API error");

    assert_eq!(structured.code, "1234");
    assert_eq!(structured.message, "invalid project");
}

#[tokio::test]
async fn succeeds_on_first_attempt() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: 0,
        fail_status: StatusCode::OK,
        retry_after: None,
    })
    .await;

    let client = test_client();
    client
        .put_blob_with_policy(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .expect("upload should succeed");

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn sends_required_presigned_put_headers() {
    let (url, _h) = spawn_conditional_header_mock().await;

    let client = test_client();
    client
        .put_blob_with_headers(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders::if_none_match_any(),
        )
        .await
        .expect("conditional upload should send If-None-Match");
}

#[tokio::test]
async fn sends_source_bundle_content_type_from_presigned_headers() {
    let (url, _h) = spawn_signed_source_headers_mock().await;

    let client = test_client();
    client
        .put_blob_with_headers(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders {
                content_type: Some("application/zstd".to_string()),
                if_none_match: Some("*".to_string()),
            },
        )
        .await
        .expect("SOURCE_BUNDLE_V1 upload should send all server-signed headers");
}

#[tokio::test]
async fn conditional_precondition_failed_verifies_existing_object() {
    let (url, _h) = spawn_precondition_with_head_mock().await;
    let verify_head = PresignedHeadVerify {
        url: url.clone(),
        content_length: 16,
        sha256: FAKE_SHA.to_string(),
    };

    let client = test_client();
    client
        .put_blob_with_headers_and_verify(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders::if_none_match_any(),
            Some(&verify_head),
        )
        .await
        .expect("conditional 412 with verified matching object should be idempotent success");
}

#[tokio::test]
async fn conditional_precondition_failed_retries_transient_head() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_precondition_with_transient_head_mock(attempts.clone()).await;
    let verify_head = PresignedHeadVerify {
        url: url.clone(),
        content_length: 16,
        sha256: FAKE_SHA.to_string(),
    };

    let client = test_client();
    client
        .put_blob_with_headers_and_verify(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders::if_none_match_any(),
            Some(&verify_head),
        )
        .await
        .expect("conditional HEAD should recover from a transient provider response");

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn conditional_head_uses_its_own_retry_budget() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_delayed_precondition_with_transient_head_mock(attempts.clone()).await;
    let verify_head = PresignedHeadVerify {
        url: url.clone(),
        content_length: 16,
        sha256: FAKE_SHA.to_string(),
    };

    let client = test_client();
    client
        .put_blob_with_policy_headers_and_verify(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::expires_before_head_for_tests(),
            &PresignedPutHeaders::if_none_match_any(),
            Some(&verify_head),
        )
        .await
        .expect("HEAD verification must not inherit an exhausted PUT retry budget");

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn conditional_head_retries_initial_not_found() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_precondition_with_initially_missing_head_mock(attempts.clone()).await;
    let verify_head = PresignedHeadVerify {
        url: url.clone(),
        content_length: 16,
        sha256: FAKE_SHA.to_string(),
    };

    let client = test_client();
    client
        .put_blob_with_headers_and_verify(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders::if_none_match_any(),
            Some(&verify_head),
        )
        .await
        .expect("HEAD should recover when object visibility briefly lags the 412 PUT");

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn conditional_precondition_failed_with_mismatch_reports_recoverable_conflict() {
    let (url, _h) = spawn_precondition_with_mismatched_head_mock().await;
    let verify_head = PresignedHeadVerify {
        url: url.clone(),
        content_length: 16,
        sha256: FAKE_SHA.to_string(),
    };

    let client = test_client();
    let error = client
        .put_blob_with_headers_and_verify(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders::if_none_match_any(),
            Some(&verify_head),
        )
        .await
        .unwrap_err();

    assert!(
        error.downcast_ref::<ConditionalUploadConflict>().is_some(),
        "{error}"
    );
}

#[tokio::test]
async fn conditional_precondition_failed_without_verify_head_is_error() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: u32::MAX,
        fail_status: StatusCode::PRECONDITION_FAILED,
        retry_after: None,
    })
    .await;

    let client = test_client();
    let err = client
        .put_blob_with_headers(
            &url,
            payload(),
            FAKE_SHA,
            &PresignedPutHeaders::if_none_match_any(),
        )
        .await
        .unwrap_err();

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(err.to_string().contains("verifyHead"), "{err}");
}

#[tokio::test]
async fn precondition_failed_without_conditional_header_is_error() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: u32::MAX,
        fail_status: StatusCode::PRECONDITION_FAILED,
        retry_after: None,
    })
    .await;

    let client = test_client();
    let err = client
        .put_blob_with_policy(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .unwrap_err();

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(err.to_string().contains("412"), "{err}");
}

#[tokio::test]
async fn retries_on_429_then_succeeds() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: 3,
        fail_status: StatusCode::TOO_MANY_REQUESTS,
        retry_after: None,
    })
    .await;

    let client = test_client();
    client
        .put_blob_with_policy(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .expect("upload should eventually succeed");

    assert_eq!(attempts.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn retries_on_408_then_succeeds() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: 2,
        fail_status: StatusCode::REQUEST_TIMEOUT,
        retry_after: None,
    })
    .await;

    let client = test_client();
    client
        .put_blob_with_policy(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .expect("408 should be treated as transient");

    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn retries_on_5xx() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: 2,
        fail_status: StatusCode::INTERNAL_SERVER_ERROR,
        retry_after: None,
    })
    .await;

    let client = test_client();
    client
        .put_blob_with_policy(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .expect("upload should recover from 5xx");

    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn honors_retry_after_header() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: 1,
        fail_status: StatusCode::TOO_MANY_REQUESTS,
        retry_after: Some("1"),
    })
    .await;

    let client = test_client();
    let policy = UploadRetryPolicy::fast_for_tests();
    let start = Instant::now();
    client
        .put_blob_with_policy(&url, payload(), FAKE_SHA, &policy)
        .await
        .expect("upload should succeed after Retry-After sleep");
    let elapsed = start.elapsed();

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    // Retry-After: 1s dominates over the tiny test backoff (cap 80ms).
    assert!(
        elapsed >= Duration::from_millis(900),
        "expected to wait ~1s per Retry-After, waited {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "should not wait much longer than Retry-After, waited {elapsed:?}"
    );
}

#[tokio::test]
async fn non_numeric_retry_after_falls_back_to_backoff() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: 1,
        fail_status: StatusCode::TOO_MANY_REQUESTS,
        retry_after: Some("Wed, 21 Oct 2015 07:28:00 GMT"),
    })
    .await;

    let client = test_client();
    let policy = UploadRetryPolicy::fast_for_tests();
    let start = Instant::now();
    client
        .put_blob_with_policy(&url, payload(), FAKE_SHA, &policy)
        .await
        .expect("non-numeric Retry-After should silently fall back to exp backoff");
    let elapsed = start.elapsed();

    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    // HTTP-date is ignored → backoff is within fast_for_tests cap (80ms).
    assert!(
        elapsed < Duration::from_millis(500),
        "should use fast exp backoff when Retry-After is HTTP-date, waited {elapsed:?}"
    );
}

#[tokio::test]
async fn retry_after_exceeding_remaining_budget_fails_fast() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: u32::MAX,
        fail_status: StatusCode::TOO_MANY_REQUESTS,
        retry_after: Some("60"),
    })
    .await;

    let client = test_client();
    let policy = UploadRetryPolicy::fast_for_tests();
    let start = Instant::now();
    let err = client
        .put_blob_with_policy(&url, payload(), FAKE_SHA, &policy)
        .await
        .expect_err("Retry-After > budget must bail immediately");
    let elapsed = start.elapsed();

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    // Must NOT sleep the full Retry-After (60s); we bail within milliseconds.
    assert!(
        elapsed < Duration::from_secs(2),
        "bail should be immediate, took {elapsed:?}"
    );
    let chain = format!("{err:#}");
    assert!(
        chain.contains("Retry-After") && chain.contains("budget"),
        "error should explain Retry-After/budget mismatch: {chain}"
    );
}

#[tokio::test]
async fn fails_fast_on_403() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: u32::MAX,
        fail_status: StatusCode::FORBIDDEN,
        retry_after: None,
    })
    .await;

    let client = test_client();
    let err = client
        .put_blob_with_policy(
            &url,
            payload(),
            FAKE_SHA,
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .expect_err("403 must not be retried");

    assert!(
        format!("{err:#}").contains("403"),
        "error should mention 403: {err:#}"
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fails_after_exhausting_attempts() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: u32::MAX,
        fail_status: StatusCode::TOO_MANY_REQUESTS,
        retry_after: None,
    })
    .await;

    let client = test_client();
    let policy = UploadRetryPolicy::fast_for_tests();
    let max = policy.max_attempts();
    let err = client
        .put_blob_with_policy(&url, payload(), FAKE_SHA, &policy)
        .await
        .expect_err("should exhaust retries");

    assert!(
        format!("{err:#}").contains("attempt"),
        "error should mention attempts: {err:#}"
    );
    assert_eq!(attempts.load(Ordering::SeqCst), max);
}

#[tokio::test]
async fn fails_when_budget_expires_before_attempts() {
    let attempts = Arc::new(AtomicU32::new(0));
    let (url, _h) = spawn_mock(MockState {
        attempts: attempts.clone(),
        fail_times: u32::MAX,
        fail_status: StatusCode::TOO_MANY_REQUESTS,
        retry_after: None,
    })
    .await;

    let policy = UploadRetryPolicy::budget_exhaust_for_tests();
    let client = test_client();
    let start = Instant::now();
    let err = client
        .put_blob_with_policy(&url, payload(), FAKE_SHA, &policy)
        .await
        .expect_err("should exhaust on budget");
    let elapsed = start.elapsed();

    // Budget is 2s, base 500ms, cap 1s. A handful of retries will consume it
    // long before max_attempts=100. Must bail with well below that many attempts.
    let attempts_made = attempts.load(Ordering::SeqCst);
    assert!(
        (2..20).contains(&attempts_made),
        "expected budget-path exhaustion, attempts={attempts_made}"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "should bail shortly after budget expires, took {elapsed:?}"
    );
    assert!(format!("{err:#}").contains("attempt"));
}

#[tokio::test]
async fn retries_transport_error_then_exhausts() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!(
        "http://{addr}/upload?X-Amz-Signature=signature-secret&X-Amz-Credential=credential-secret"
    );
    let handle = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            drop(stream);
        }
    });

    let client = test_client();
    let policy = UploadRetryPolicy::fast_for_tests();
    let start = Instant::now();
    let err = client
        .put_blob_with_policy(&url, payload(), FAKE_SHA, &policy)
        .await
        .expect_err("connection refused must eventually bail");
    let elapsed = start.elapsed();
    handle.abort();

    // We should have retried (took non-trivial time, but bounded by budget).
    assert!(
        elapsed < Duration::from_secs(11),
        "should bail within budget, took {elapsed:?}"
    );
    let chain = format!("{err:#}");
    assert!(
        chain.contains("attempt"),
        "error should mention attempts after exhaustion: {chain}"
    );
    assert!(!chain.contains("X-Amz-Signature"), "{chain}");
    assert!(!chain.contains("signature-secret"), "{chain}");
    assert!(!chain.contains("credential-secret"), "{chain}");
}

// ── sha256_hex_to_base64 ────────────────────────────────────
//
// Mistakes here surface as `400 BadDigest` from S3 with no other clue, since
// the header is the entire integrity contract for blob/bundle PUTs. Worth a
// dedicated test surface even though the helper is small.

/// SHA-256 of "hello world" — used as a known-good vector across the codebase.
const HELLO_WORLD_SHA: &str = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
/// AWS-canonical base64 of the raw 32-byte SHA above (NOT base64 of the hex string).
const HELLO_WORLD_SHA_B64: &str = "uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=";

#[test]
fn sha256_hex_to_base64_known_vector() {
    let got = sha256_hex_to_base64(HELLO_WORLD_SHA).unwrap();
    assert_eq!(got, HELLO_WORLD_SHA_B64);
}

#[test]
fn sha256_hex_to_base64_rejects_uppercase() {
    let upper = HELLO_WORLD_SHA.to_uppercase();
    let err = sha256_hex_to_base64(&upper).unwrap_err();
    assert!(err.to_string().contains("lowercase"), "{err}");
}

#[test]
fn sha256_hex_to_base64_rejects_too_short() {
    let short = &HELLO_WORLD_SHA[..63];
    let err = sha256_hex_to_base64(short).unwrap_err();
    assert!(err.to_string().contains("64"), "{err}");
}

#[test]
fn sha256_hex_to_base64_rejects_too_long() {
    let mut long = HELLO_WORLD_SHA.to_string();
    long.push('a');
    let err = sha256_hex_to_base64(&long).unwrap_err();
    assert!(err.to_string().contains("64"), "{err}");
}

#[test]
fn sha256_hex_to_base64_rejects_non_hex() {
    // Right length, one bad char in the middle.
    let mut bad = HELLO_WORLD_SHA.to_string();
    bad.replace_range(30..31, "z");
    let err = sha256_hex_to_base64(&bad).unwrap_err();
    assert!(err.to_string().contains("lowercase"), "{err}");
}

#[test]
fn sha256_hex_to_base64_rejects_empty() {
    let err = sha256_hex_to_base64("").unwrap_err();
    assert!(err.to_string().contains("64"), "{err}");
}

// ── explain_s3_failure ──────────────────────────────────────
//
// `400 BadDigest` and `403 SignatureDoesNotMatch` are the two codes the
// conditioned-PUT contract produces under content/size drift; users hit them
// most often via build-cache races between scan and upload. The translation
// turns S3's XML dump into one actionable sentence.

#[test]
fn explain_s3_failure_translates_bad_digest() {
    let body = r#"<?xml version="1.0" encoding="UTF-8"?><Error><Code>BadDigest</Code><Message>The Content-MD5 you specified did not match what we received.</Message></Error>"#;
    let msg = explain_s3_failure(StatusCode::BAD_REQUEST, body);
    assert!(msg.contains("BadDigest"));
    assert!(msg.contains("SHA-256"));
    assert!(msg.contains("Rebuild"), "should hint at rebuild: {msg}");
}

#[test]
fn explain_s3_failure_translates_signature_does_not_match() {
    let body = r#"<Error><Code>SignatureDoesNotMatch</Code><Message>The request signature we calculated does not match.</Message></Error>"#;
    let msg = explain_s3_failure(StatusCode::FORBIDDEN, body);
    assert!(msg.contains("SignatureDoesNotMatch"));
    assert!(msg.contains("Content-Length"));
    assert!(msg.contains("Content-Type"));
    assert!(msg.contains("If-None-Match"));
}

#[test]
fn explain_s3_failure_falls_back_to_raw_body_for_unknown_code() {
    let body = r#"<Error><Code>EntityTooLarge</Code><Message>Your proposed upload exceeds the maximum allowed object size.</Message></Error>"#;
    let msg = explain_s3_failure(StatusCode::BAD_REQUEST, body);
    assert!(
        msg.contains("EntityTooLarge"),
        "raw body should be preserved as diagnostic when we don't know the code: {msg}"
    );
}

#[test]
fn explain_s3_failure_falls_back_when_body_has_no_code_tag() {
    // Some gateways return plaintext on edge errors — must not silently lose it.
    let body = "503 Service Unavailable\n";
    let msg = explain_s3_failure(StatusCode::SERVICE_UNAVAILABLE, body);
    assert!(msg.contains("503"));
    assert!(msg.contains("Service Unavailable"));
}
