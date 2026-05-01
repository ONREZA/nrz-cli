use super::projects_handler::CreateProjectBody;

#[test]
fn create_project_body_marks_user_supplied_build_settings() {
    let body = CreateProjectBody {
        name: "app".to_string(),
        display_name: None,
        git_url: None,
        branch: None,
        framework_preset: Some("nextjs".to_string()),
        install_command: Some("pnpm install".to_string()),
        install_command_source: Some("USER"),
        build_command: Some("pnpm build".to_string()),
        build_command_source: Some("USER"),
        output_directory: Some(".next".to_string()),
        output_directory_source: Some("USER"),
    };

    let value = serde_json::to_value(body).unwrap();
    assert_eq!(value["installCommandSource"], "USER");
    assert_eq!(value["buildCommandSource"], "USER");
    assert_eq!(value["outputDirectorySource"], "USER");
}
