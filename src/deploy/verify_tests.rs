use super::verify::{response_needs_preview_bypass, verification_url};

#[test]
fn verification_url_replaces_path_and_query() {
    assert_eq!(
        verification_url("https://example.test/old?x=1", "/health").unwrap(),
        "https://example.test/health"
    );
}

#[test]
fn preview_auth_redirect_requests_temporary_bypass() {
    assert!(response_needs_preview_bypass(
        302,
        Some("https://app.onreza-stage.ru/preview-auth?projectId=1")
    ));
}

#[test]
fn successful_or_unrelated_responses_do_not_request_bypass() {
    assert!(!response_needs_preview_bypass(
        200,
        Some("https://app.onreza.ru/preview-auth?projectId=1")
    ));
    assert!(!response_needs_preview_bypass(
        302,
        Some("https://example.test/login")
    ));
}

#[test]
fn protected_preview_forbidden_response_requests_access() {
    assert!(response_needs_preview_bypass(403, None));
}

#[tokio::test]
async fn verification_waits_for_new_preview_access_to_reach_edge() {
    use super::verify::{PreviewAccessResponse, wait_for_preview_access};
    use std::time::Duration;

    let mut statuses = [401, 429, 503, 204].into_iter();
    let response = wait_for_preview_access(Duration::from_secs(1), Duration::ZERO, || {
        let status_code = statuses.next().unwrap();
        async move {
            Ok(PreviewAccessResponse {
                status_code,
                cookie: None,
                retry_after: None,
            })
        }
    })
    .await
    .unwrap();
    assert_eq!(response.status_code, 204);
}

#[tokio::test]
async fn preview_readiness_exchange_uses_cookie_and_preserves_application_failure() {
    use axum::{
        Json, Router,
        http::{HeaderMap, StatusCode},
        routing::{get, post},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let attempts = Arc::new(AtomicUsize::new(0));
    let calls = attempts.clone();
    let app = Router::new().route("/.onreza/preview/credential", post(move |Json(body): Json<serde_json::Value>| {
        let calls = calls.clone();
        async move {
            assert_eq!(body, serde_json::json!({"kind":"BYPASS","credential":"credential"}));
            if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                (StatusCode::TOO_MANY_REQUESTS, [("retry-after", "0")], "RATE_LIMITED")
            } else {
                (StatusCode::NO_CONTENT, [("set-cookie", "__Host-onreza-preview-capability=P2.test; Path=/; Secure; HttpOnly")], "")
            }
        }
    })).route("/health", get(|headers: HeaderMap| async move {
        assert_eq!(headers["cookie"], "__Host-onreza-preview-capability=P2.test");
        assert!(!headers.contains_key("x-onreza-protection-bypass"));
        StatusCode::TOO_MANY_REQUESTS
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/health", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let response = super::verify::verify_with_preview_access(&url, "credential")
        .await
        .unwrap();
    assert_eq!(response.status_code, 429);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    server.abort();
}

#[tokio::test(start_paused = true)]
async fn preview_readiness_respects_retry_after_and_total_deadline() {
    use super::verify::{PreviewAccessResponse, wait_for_preview_access};
    use std::time::Duration;
    let mut calls = 0;
    let result = wait_for_preview_access(Duration::from_secs(5), Duration::from_secs(1), || {
        calls += 1;
        async {
            Ok(PreviewAccessResponse {
                status_code: 429,
                cookie: None,
                retry_after: Some(Duration::from_secs(30)),
            })
        }
    })
    .await;
    assert!(result.is_err());
    assert_eq!(calls, 1);
}
