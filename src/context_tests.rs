use crate::context::CommandContext;
use nrz::config::{BuildSettingSource, ProjectBuildSettings, ProjectConfig, SourceAwareSetting};

#[test]
fn platform_context_uses_immutable_root_and_build_settings() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("apps/web");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        app.join("onreza.toml"),
        "[build]\ncommand = \"local build\"\n",
    )
    .unwrap();

    let mut root_config = ProjectConfig::default();
    root_config.deploy.app = Some("missing-local-app".to_string());
    let mut context =
        CommandContext::resolve_platform_root(root.path(), &root_config, true).unwrap();
    context
        .apply_platform_runner_settings(&ProjectBuildSettings {
            root_directory: "apps/web".to_string(),
            package_manager: "YARN".to_string(),
            build_command: Some("yarn build".to_string()),
            build_command_source: Some(BuildSettingSource::User),
            ..Default::default()
        })
        .unwrap();

    assert_eq!(context.project_dir, app.canonicalize().unwrap());
    assert_eq!(context.effective.project_dir(), context.project_dir);
    assert_eq!(
        context
            .effective
            .build_command()
            .and_then(SourceAwareSetting::value),
        Some("yarn build")
    );
    assert!(context.selected_app.is_none());
}

#[test]
fn platform_context_reports_invalid_config_from_the_snapshot_root() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("apps/web");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("onreza.toml"), "not valid toml = [").unwrap();
    let mut context =
        CommandContext::resolve_platform_root(root.path(), &ProjectConfig::default(), true)
            .unwrap();

    let error = context
        .apply_platform_runner_settings(&ProjectBuildSettings {
            root_directory: "apps/web".to_string(),
            package_manager: "YARN".to_string(),
            ..Default::default()
        })
        .unwrap_err();

    let coded = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .unwrap();
    assert_eq!(coded.code, "INVALID_CONFIG");
}
