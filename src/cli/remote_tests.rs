use nrz::config::ProjectConfig;

use super::project_client;

#[test]
fn token_validation_precedes_project_selection() {
    let error = project_client(
        Some("invalid\ntoken"),
        None,
        None,
        &ProjectConfig::default(),
    )
    .err()
    .expect("invalid token must fail before missing project");
    assert_eq!(error.to_string(), "invalid token format");
}

#[test]
fn explicit_project_overrides_linked_project_and_missing_selection_fails() {
    let mut config = ProjectConfig::default();
    config.project.id = Some(" linked-project ".to_string());

    let (_, explicit) = project_client(Some("test-token"), None, Some(" explicit "), &config)
        .expect("explicit project selected");
    assert_eq!(explicit, "explicit");
    let (_, linked) =
        project_client(Some("test-token"), None, None, &config).expect("linked project selected");
    assert_eq!(linked, "linked-project");

    config.project.id = None;
    let error = project_client(Some("test-token"), None, None, &config)
        .err()
        .expect("missing project must fail");
    assert!(error.to_string().contains("no project specified"));
}
