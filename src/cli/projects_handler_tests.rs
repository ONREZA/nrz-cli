use nrz_api::{ProjectRequestBody, ProjectRequestBodyInstallCommandSource};

#[test]
fn create_project_body_marks_user_supplied_build_settings() {
    let body = ProjectRequestBody {
        name: "app".to_string(),
        display_name: None,
        git_url: None,
        branch: None,
        framework_preset: Some("nextjs".to_string()),
        install_command: Some(Some("pnpm install".to_string())),
        install_command_source: Some(ProjectRequestBodyInstallCommandSource::User),
        build_command: Some(Some("pnpm build".to_string())),
        build_command_source: Some(ProjectRequestBodyInstallCommandSource::User),
        output_directory: Some(".next".to_string()),
        output_directory_source: Some(ProjectRequestBodyInstallCommandSource::User),
        ..Default::default()
    };

    let value = serde_json::to_value(body).unwrap();
    assert_eq!(value["installCommandSource"], "USER");
    assert_eq!(value["buildCommandSource"], "USER");
    assert_eq!(value["outputDirectorySource"], "USER");
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
