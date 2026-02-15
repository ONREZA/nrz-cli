use super::config::{WorkspaceConfig, load_from, save_to};

fn make_config() -> WorkspaceConfig {
    let mut config = WorkspaceConfig::empty();
    config.add_workspace("john-doe", "nrz_xxx".into(), "John Doe".into());
    config
}

fn config_path(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    tmp.path().join("nrz").join("config.json")
}

fn legacy_path(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    tmp.path().join("nrz").join("credentials.json")
}

#[test]
fn save_and_load_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let cp = config_path(&tmp);
    let lp = legacy_path(&tmp);
    let config = make_config();

    save_to(&cp, &config).unwrap();

    let loaded = load_from(&cp, &lp);
    assert_eq!(loaded.workspaces.len(), 1);
    assert_eq!(loaded.workspaces["john-doe"].token, "nrz_xxx");
    assert_eq!(loaded.workspaces["john-doe"].name, "John Doe");
    assert_eq!(loaded.default_workspace.as_deref(), Some("john-doe"));
}

#[test]
fn load_returns_empty_when_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let loaded = load_from(&config_path(&tmp), &legacy_path(&tmp));
    assert!(loaded.workspaces.is_empty());
    assert!(loaded.default_workspace.is_none());
}

#[test]
fn migrate_from_legacy_credentials() {
    let tmp = tempfile::tempdir().unwrap();
    let cp = config_path(&tmp);
    let lp = legacy_path(&tmp);

    std::fs::create_dir_all(lp.parent().unwrap()).unwrap();
    std::fs::write(
        &lp,
        r#"{"access_token":"nrz_old","workspace_slug":"my-team","workspace_name":"My Team"}"#,
    )
    .unwrap();

    let loaded = load_from(&cp, &lp);
    assert_eq!(loaded.workspaces.len(), 1);
    assert_eq!(loaded.workspaces["my-team"].token, "nrz_old");
    assert_eq!(loaded.default_workspace.as_deref(), Some("my-team"));

    // Legacy file should be removed
    assert!(!lp.exists());
    // New config should exist
    assert!(cp.exists());
}

#[test]
fn migrate_empty_slug_becomes_personal() {
    let tmp = tempfile::tempdir().unwrap();
    let cp = config_path(&tmp);
    let lp = legacy_path(&tmp);

    std::fs::create_dir_all(lp.parent().unwrap()).unwrap();
    std::fs::write(
        &lp,
        r#"{"access_token":"nrz_tok","workspace_slug":"","workspace_name":"Personal"}"#,
    )
    .unwrap();

    let loaded = load_from(&cp, &lp);
    assert!(loaded.workspaces.contains_key("personal"));
    assert_eq!(loaded.default_workspace.as_deref(), Some("personal"));
}

#[test]
fn add_workspace_sets_default_on_first() {
    let mut config = WorkspaceConfig::empty();
    config.add_workspace("first", "tok1".into(), "First".into());
    assert_eq!(config.default_workspace.as_deref(), Some("first"));

    config.add_workspace("second", "tok2".into(), "Second".into());
    // Default stays at first
    assert_eq!(config.default_workspace.as_deref(), Some("first"));
}

#[test]
fn remove_workspace_updates_default() {
    let mut config = WorkspaceConfig::empty();
    config.add_workspace("a", "tok_a".into(), "A".into());
    config.add_workspace("b", "tok_b".into(), "B".into());
    config.default_workspace = Some("a".into());

    config.remove_workspace("a");
    assert!(!config.workspaces.contains_key("a"));
    // Only one left, so it becomes default
    assert_eq!(config.default_workspace.as_deref(), Some("b"));
}

#[test]
fn remove_workspace_clears_default_when_multiple_remain() {
    let mut config = WorkspaceConfig::empty();
    config.add_workspace("a", "tok_a".into(), "A".into());
    config.add_workspace("b", "tok_b".into(), "B".into());
    config.add_workspace("c", "tok_c".into(), "C".into());
    config.default_workspace = Some("a".into());

    config.remove_workspace("a");
    // Multiple remain, default cleared
    assert!(config.default_workspace.is_none());
}

#[cfg(unix)]
#[test]
fn save_sets_permissions_600() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let path = config_path(&tmp);

    save_to(&path, &make_config()).unwrap();

    let perms = std::fs::metadata(&path).unwrap().permissions();
    assert_eq!(perms.mode() & 0o777, 0o600);
}

#[test]
fn load_ignores_corrupt_config() {
    let tmp = tempfile::tempdir().unwrap();
    let cp = config_path(&tmp);
    let lp = legacy_path(&tmp);

    std::fs::create_dir_all(cp.parent().unwrap()).unwrap();
    std::fs::write(&cp, "not json").unwrap();

    let loaded = load_from(&cp, &lp);
    assert!(loaded.workspaces.is_empty());
}
