#[test]
fn create_project_body_marks_user_supplied_build_settings() {
    let body = super::projects_handler::user_build_settings(
        Some("pnpm install".to_string()),
        Some("pnpm build".to_string()),
        Some(".next".to_string()),
    );

    let value = serde_json::to_value(body).unwrap();
    assert_eq!(value["installCommand"], "pnpm install");
    assert_eq!(value["buildCommand"], "pnpm build");
    assert_eq!(value["outputDirectory"], ".next");
    assert_eq!(value["installCommandSource"], "USER");
    assert_eq!(value["buildCommandSource"], "USER");
    assert_eq!(value["outputDirectorySource"], "USER");
}

#[test]
fn omitted_project_build_settings_preserve_default_sources() {
    let body = super::projects_handler::user_build_settings(None, None, None);
    let value = serde_json::to_value(body).unwrap();
    for field in [
        "installCommand",
        "installCommandSource",
        "buildCommand",
        "buildCommandSource",
        "outputDirectory",
        "outputDirectorySource",
    ] {
        assert!(
            value.get(field).is_none(),
            "unexpected setting {field}: {value}"
        );
    }
}

#[tokio::test]
async fn malformed_project_id_is_rejected_before_transport() {
    let client = crate::api::ApiClient::with_http_client(
        "http://127.0.0.1:1".to_string(),
        reqwest::Client::new(),
    )
    .unwrap();
    let error = client
        .project("project/../victim?force=true")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("invalid project ID"));
}
