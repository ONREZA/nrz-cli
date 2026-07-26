use super::project_context;

#[test]
fn selected_app_resolves_recursive_workspace_package() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("apps/group/web");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(
        root.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/**"]}"#,
    )
    .unwrap();
    std::fs::write(app.join("package.json"), r#"{"name":"web"}"#).unwrap();

    let context = project_context::resolve(
        root.path(),
        &nrz::config::ProjectConfig::default(),
        Some("web"),
    )
    .unwrap();

    assert_eq!(context.project_dir, app);
    assert_eq!(
        context.selected_app.as_ref().map(|app| app.path.as_str()),
        Some("apps/group/web")
    );
}

#[test]
fn selected_app_requires_an_unambiguous_name() {
    let root = tempfile::tempdir().unwrap();
    for path in ["apps/web", "examples/web"] {
        let package = root.path().join(path);
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("package.json"),
            format!(r#"{{"name":"@{path}"}}"#),
        )
        .unwrap();
    }
    std::fs::write(
        root.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/*","examples/*"]}"#,
    )
    .unwrap();

    let error = project_context::resolve(
        root.path(),
        &nrz::config::ProjectConfig::default(),
        Some("web"),
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("matches multiple monorepo packages")
    );
    assert!(error.to_string().contains("apps/web, examples/web"));
}

#[cfg(unix)]
#[test]
fn selected_app_must_resolve_inside_monorepo_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/*"]}"#,
    )
    .unwrap();
    std::fs::create_dir_all(root.path().join("apps")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("apps/escaped")).unwrap();
    std::fs::write(outside.path().join("package.json"), r#"{"name":"escaped"}"#).unwrap();

    let error = project_context::resolve(
        root.path(),
        &nrz::config::ProjectConfig::default(),
        Some("escaped"),
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("not found")
            || error.to_string().contains("escapes the monorepo root")
    );
}
