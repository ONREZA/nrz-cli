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
