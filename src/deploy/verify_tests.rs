use super::verify::{
    production_alias_url_from_response, response_needs_preview_bypass, verification_url,
};

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
fn production_verification_selects_active_production_alias() {
    let response = r#"{
      "deploymentUrls": [
        {
          "fullUrl": "https://project-sha-workspace.onreza.app",
          "aliasType": "UNIQUE_URL"
        },
        {
          "fullUrl": "https://project-workspace.onreza.app",
          "aliasType": "PRODUCTION_ALIAS"
        }
      ]
    }"#;

    assert_eq!(
        production_alias_url_from_response(response).unwrap(),
        Some("https://project-workspace.onreza.app".to_string())
    );
}

#[test]
fn protected_preview_forbidden_response_requests_access() {
    assert!(response_needs_preview_bypass(403, None));
}

#[tokio::test]
async fn verification_waits_for_new_preview_access_to_reach_edge() {
    use super::verify::{VerificationResponse, wait_for_preview_access};
    use std::time::Duration;

    let mut statuses = [401, 403, 200].into_iter();
    let response = wait_for_preview_access(Duration::from_secs(1), Duration::ZERO, || {
        let status_code = statuses.next().unwrap();
        async move {
            Ok(VerificationResponse {
                status_code,
                location: None,
            })
        }
    })
    .await
    .unwrap();
    assert_eq!(response.status_code, 200);
}
