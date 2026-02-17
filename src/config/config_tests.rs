use super::*;

#[test]
fn load_full_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"
[project]
id = "proj_123"
name = "my-app"

[dev]
command = "astro dev"
port = 3000
host = "0.0.0.0"

data_dir = "custom/data"
db_name = "custom.db"

[build]
output_dirs = ["out", "public"]

[deploy]
skip_migrations = true

[migrations]
dir = "db/migrations"

[db]
default_env = "preview"
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_123"));
    assert_eq!(config.project.name.as_deref(), Some("my-app"));
    assert_eq!(config.dev.command.as_deref(), Some("astro dev"));
    assert_eq!(config.dev_port(), 3000);
    assert_eq!(config.dev_host(), "0.0.0.0");
    assert_eq!(config.data_dir_relative(), "custom/data");
    assert_eq!(config.db_name(), "custom.db");
    assert_eq!(config.output_dirs(), vec!["out", "public"]);
    assert!(config.skip_migrations());
    assert_eq!(config.migrations_dir(), "db/migrations");
    assert_eq!(config.db.default_env.as_deref(), Some("preview"));
}

#[test]
fn load_minimal_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_abc\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_abc"));
    // Defaults
    assert_eq!(config.dev_port(), 4321);
    assert_eq!(config.dev_host(), "127.0.0.1");
    assert_eq!(config.data_dir_relative(), ".onreza/data");
    assert_eq!(config.db_name(), "dev.db");
    assert_eq!(config.output_dirs(), vec!["dist", ".output", "build"]);
    assert!(!config.skip_migrations());
    assert_eq!(config.migrations_dir(), "migrations");
    assert!(config.db.default_env.is_none());
}

#[test]
fn load_empty_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "").unwrap();

    let config = load(dir.path()).unwrap();
    assert!(config.project.id.is_none());
    assert_eq!(config.dev_port(), 4321);
}

#[test]
fn load_missing_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let config = load(dir.path()).unwrap();
    assert!(config.project.id.is_none());
    assert_eq!(config.dev_port(), 4321);
}

#[test]
fn load_invalid_toml_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "invalid {{{}}}").unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("failed to parse"), "got: {err}");
}

#[test]
fn generate_template_contains_project_id() {
    let content = generate_template("proj_test");
    assert!(content.contains("id = \"proj_test\""));
    assert!(content.contains("[project]"));
    assert!(content.contains("# port = 4321"));
}

#[test]
fn save_or_update_creates_new_file() {
    let dir = tempfile::tempdir().unwrap();
    save_or_update(dir.path(), "proj_new").unwrap();

    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    assert!(content.contains("id = \"proj_new\""));

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_new"));
}

#[test]
fn save_or_update_preserves_existing_settings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"[project]
id = "proj_old"

[dev]
port = 3000
"#,
    )
    .unwrap();

    save_or_update(dir.path(), "proj_new").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_new"));
    assert_eq!(config.dev_port(), 3000);
}

#[test]
fn save_or_update_noop_when_same_id() {
    let dir = tempfile::tempdir().unwrap();
    let original = "[project]\nid = \"proj_same\"\n\n[dev]\nport = 5000\n";
    std::fs::write(dir.path().join("onreza.toml"), original).unwrap();

    save_or_update(dir.path(), "proj_same").unwrap();

    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    assert_eq!(content, original);
}

#[test]
fn data_dir_path_resolves_correctly() {
    let config = ProjectConfig::default();
    let path = config.data_dir_path(Path::new("/my/project"));
    assert_eq!(path, PathBuf::from("/my/project/.onreza/data"));
}

#[test]
fn data_dir_path_custom() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[dev]\ndata_dir = \"custom/data\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    let path = config.data_dir_path(dir.path());
    assert_eq!(path, dir.path().join("custom/data"));
}

#[test]
fn save_or_update_inserts_id_when_missing_in_project_section() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nname = \"my-app\"\n\n[dev]\nport = 3000\n",
    )
    .unwrap();

    save_or_update(dir.path(), "proj_new").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_new"));
    assert_eq!(config.project.name.as_deref(), Some("my-app"));
    assert_eq!(config.dev_port(), 3000);
}

#[test]
fn save_or_update_adds_project_section_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "[dev]\nport = 3000\n").unwrap();

    save_or_update(dir.path(), "proj_new").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_new"));
    assert_eq!(config.dev_port(), 3000);
}

#[test]
fn save_or_update_does_not_replace_id_in_wrong_section() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_old\"\n\n[dev]\nport = 3000\n",
    )
    .unwrap();

    save_or_update(dir.path(), "proj_new").unwrap();

    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    assert!(content.contains("id = \"proj_new\""));
    // Ensure [dev] section is preserved
    assert!(content.contains("port = 3000"));
}

#[test]
fn save_or_update_preserves_comments() {
    let dir = tempfile::tempdir().unwrap();
    let original =
        "# My project config\n[project]\n# Project ID\nid = \"proj_old\"\n\n[dev]\nport = 3000\n";
    std::fs::write(dir.path().join("onreza.toml"), original).unwrap();

    save_or_update(dir.path(), "proj_new").unwrap();

    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    assert!(content.contains("# My project config"));
    assert!(content.contains("# Project ID"));
    assert!(content.contains("id = \"proj_new\""));
}

#[test]
fn save_or_update_fails_on_corrupt_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "{{invalid}}").unwrap();

    let result = save_or_update(dir.path(), "proj_new");
    assert!(result.is_err());
}

#[test]
fn load_config_with_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\n\n[future_section]\nfoo = \"bar\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_ok());
    assert_eq!(result.unwrap().project.id.as_deref(), Some("proj_1"));
}

#[test]
fn load_config_with_unknown_fields_in_known_section() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\nfuture_field = true\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_ok());
}
