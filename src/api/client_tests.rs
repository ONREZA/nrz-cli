use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::put;
use bytes::Bytes;

use super::client::{ApiClient, UploadRetryPolicy};

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

fn test_client() -> ApiClient {
    ApiClient::anonymous().expect("client build")
}

fn payload() -> Bytes {
    Bytes::from_static(&[0u8; 16])
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
        .put_bytes_with_policy(
            &url,
            payload(),
            "application/octet-stream",
            &UploadRetryPolicy::fast_for_tests(),
        )
        .await
        .expect("upload should succeed");

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
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
        .put_bytes_with_policy(
            &url,
            payload(),
            "application/octet-stream",
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
        .put_bytes_with_policy(
            &url,
            payload(),
            "application/octet-stream",
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
        .put_bytes_with_policy(
            &url,
            payload(),
            "application/octet-stream",
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
        .put_bytes_with_policy(&url, payload(), "application/octet-stream", &policy)
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
        .put_bytes_with_policy(&url, payload(), "application/octet-stream", &policy)
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
        .put_bytes_with_policy(&url, payload(), "application/octet-stream", &policy)
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
        .put_bytes_with_policy(
            &url,
            payload(),
            "application/octet-stream",
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
        .put_bytes_with_policy(&url, payload(), "application/octet-stream", &policy)
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
        .put_bytes_with_policy(&url, payload(), "application/octet-stream", &policy)
        .await
        .expect_err("should exhaust on budget");
    let elapsed = start.elapsed();

    // Budget is 250ms, base 50ms, cap 200ms. A handful of retries will consume it
    // long before max_attempts=100. Must bail with well below that many attempts.
    let attempts_made = attempts.load(Ordering::SeqCst);
    assert!(
        (2..20).contains(&attempts_made),
        "expected budget-path exhaustion, attempts={attempts_made}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "should bail shortly after budget expires, took {elapsed:?}"
    );
    assert!(format!("{err:#}").contains("attempt"));
}

#[tokio::test]
async fn retries_transport_error_then_exhausts() {
    // Bind and immediately drop — port is free, connections get refused deterministically.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let url = format!("http://{addr}/upload");

    let client = test_client();
    let policy = UploadRetryPolicy::fast_for_tests();
    let start = Instant::now();
    let err = client
        .put_bytes_with_policy(&url, payload(), "application/octet-stream", &policy)
        .await
        .expect_err("connection refused must eventually bail");
    let elapsed = start.elapsed();

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
}
