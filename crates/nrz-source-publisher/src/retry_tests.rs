use super::*;

#[tokio::test]
async fn retry_budget_bounds_a_request_that_never_completes() {
    let result = retry_control_plane(
        &NoopPublicationObserver,
        "pending",
        Duration::from_millis(5),
        std::future::pending::<Result<(), SourcePublicationError>>,
    )
    .await;
    assert!(matches!(result, Err(SourcePublicationError::Deadline(_))));
}

#[test]
fn retry_policy_preserves_http_status_when_an_error_body_is_not_json() {
    let error = StructuredControlPlaneError {
        status: 503,
        code: "HTTP_503".into(),
        message: "Service Unavailable".into(),
        retry_after: Some(Duration::from_secs(3)),
        details: None,
    }
    .into();
    assert_eq!(retry_hint(&error), Some(Some(Duration::from_secs(3))));
}
