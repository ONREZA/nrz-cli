use std::time::Duration;

use super::build_logs::{
    ExactValueRedactor, collect_upload_batch, error_code, error_details, parse_env_toggle,
    sanitize_message, truncate_utf8,
};

#[test]
fn parses_build_log_upload_environment_toggles() {
    assert_eq!(parse_env_toggle(Some("1")), Some(true));
    assert_eq!(parse_env_toggle(Some("true")), Some(true));
    assert_eq!(parse_env_toggle(Some("0")), Some(false));
    assert_eq!(parse_env_toggle(Some("off")), Some(false));
    assert_eq!(parse_env_toggle(Some("unexpected")), None);
}

#[test]
fn redacts_secrets_ansi_and_url_credentials_before_upload() {
    let redactor =
        ExactValueRedactor::from_values(["exact-environment-value".to_string()]).unwrap();
    let sanitized = sanitize_message(
        "\u{1b}[31mAuthorization: Bearer secret\u{1b}[0m AWS_SECRET_ACCESS_KEY=hidden GITHUB_TOKEN=\"secret with spaces\" Cookie=session-id exact-environment-value https://user:pass@example.test/a?token=secret",
        &redactor,
    );

    assert!(!sanitized.contains("secret"));
    assert!(!sanitized.contains("pass"));
    assert!(!sanitized.contains("token="));
    assert!(!sanitized.contains("\u{1b}"));
    assert!(sanitized.contains("[REDACTED]"));
    assert!(sanitized.contains("AWS_SECRET_ACCESS_KEY=[REDACTED]"));
    assert!(sanitized.contains("GITHUB_TOKEN=[REDACTED]"));
    assert!(sanitized.contains("Cookie=[REDACTED]"));
    assert!(!sanitized.contains("exact-environment-value"));
}

#[test]
fn masks_exact_environment_values_only_in_sanitized_copy() {
    let raw = "build printed tiny and api-secret-value";
    let redactor =
        ExactValueRedactor::from_values(["tiny".to_string(), "api-secret-value".to_string()])
            .unwrap();

    let sanitized = sanitize_message(raw, &redactor);

    assert_eq!(raw, "build printed tiny and api-secret-value");
    assert_eq!(sanitized, "build printed [REDACTED] and [REDACTED]");
}

#[test]
fn masks_exact_environment_values_inside_structured_details() {
    let redactor =
        ExactValueRedactor::from_values(["materialized-secret-value".to_string()]).unwrap();
    let details = serde_json::json!({
        "fields": [{
            "field": "functions.edgeRules",
            "message": "failed with materialized-secret-value"
        }]
    });

    assert_eq!(
        redactor.sanitize_json(&details),
        serde_json::json!({
            "fields": [{
                "field": "functions.edgeRules",
                "message": "failed with [REDACTED]"
            }]
        })
    );
}

#[test]
fn terminal_diagnostic_is_available_to_build_log_finalization() {
    let details = serde_json::json!({
        "fields": [{"field": "functions.edgeRules", "message": "invalid generated rules"}]
    });
    let error = crate::output::report_terminal_error(
        "deploy",
        "source registration failed",
        "VALIDATION_ERROR",
        Some(&details),
    );

    assert_eq!(error_code(&error).as_deref(), Some("VALIDATION_ERROR"));
    assert_eq!(error_details(&error).as_ref(), Some(&details));
}

#[test]
fn preserves_common_non_sensitive_build_vocabulary() {
    let redactor = ExactValueRedactor::from_values([
        "production".to_string(),
        "actual-secret-value".to_string(),
    ])
    .unwrap();

    let sanitized = sanitize_message(
        "vite building for production with actual-secret-value",
        &redactor,
    );

    assert_eq!(sanitized, "vite building for production with [REDACTED]");
}

#[test]
fn masks_multiline_environment_values_after_output_is_split_into_lines() {
    let redactor =
        ExactValueRedactor::from_values(["first-secret\nsecond-secret".to_string()]).unwrap();

    assert_eq!(sanitize_message("first-secret", &redactor), "[REDACTED]");
    assert_eq!(sanitize_message("second-secret", &redactor), "[REDACTED]");
}

#[test]
fn masks_environment_values_before_message_truncation() {
    let secret = "s".repeat(5_000);
    let redactor = ExactValueRedactor::from_values([secret.clone()]).unwrap();

    let sanitized = sanitize_message(&format!("value={secret}"), &redactor);

    assert_eq!(sanitized, "value=[REDACTED]");
}

#[test]
fn truncates_on_a_utf8_boundary() {
    let truncated = truncate_utf8("я".repeat(20), 24);

    assert!(truncated.len() <= 24);
    assert!(truncated.ends_with("…[TRUNCATED]"));
}

#[tokio::test]
async fn upload_batch_collects_events_arriving_inside_flush_window() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    sender.send(1).await.unwrap();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        sender.send(2).await.unwrap();
    });

    let (batch, closed) = collect_upload_batch(&mut receiver, 4, Duration::from_millis(100)).await;

    assert_eq!(batch, vec![1, 2]);
    assert!(closed);
}
