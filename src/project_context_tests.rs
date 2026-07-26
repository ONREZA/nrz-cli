use super::project_context;

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
