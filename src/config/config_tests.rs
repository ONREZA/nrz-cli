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

[build]
output_dirs = ["out", "public"]

[db]
database = "my-db"
branch = "dev"
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_123"));
    assert_eq!(config.project.name.as_deref(), Some("my-app"));
    assert!(config.project.workspace.is_none());
    assert_eq!(config.dev.command.as_deref(), Some("astro dev"));
    assert_eq!(config.dev_port(), 3000);
    assert_eq!(config.dev_host(), "0.0.0.0");
    assert_eq!(config.data_dir_relative(), "custom/data");
    assert_eq!(config.output_dirs(), vec!["out", "public"]);
    assert_eq!(config.db_database(), Some("my-db"));
    assert_eq!(config.db_branch(), Some("dev"));
    assert!(config.build_command().is_none());
    assert!(config.dev.aliases.is_empty());
}

#[test]
fn load_config_with_build_command() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[build]\ncommand = \"pnpm build\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.build_command(), Some("pnpm build"));
}

#[test]
fn load_config_with_dev_aliases() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"
[dev.aliases]
network = "npm run dev -- --host 0.0.0.0"
staging = "npm run dev -- --port 3001"
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.dev.aliases.len(), 2);
    assert_eq!(
        config.dev_alias_command("network"),
        Some("npm run dev -- --host 0.0.0.0")
    );
    assert_eq!(
        config.dev_alias_command("staging"),
        Some("npm run dev -- --port 3001")
    );
    assert!(config.dev_alias_command("nonexistent").is_none());
}

#[test]
fn default_config_has_empty_aliases_and_no_build_command() {
    let config = ProjectConfig::default();
    assert!(config.dev.aliases.is_empty());
    assert!(config.build_command().is_none());
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
    assert_eq!(
        config.output_dirs(),
        vec![
            "dist",
            ".output",
            "build",
            "out",
            "_site",
            "www",
            ".vitepress/dist"
        ]
    );
    assert!(config.db_database().is_none());
    assert!(config.db_branch().is_none());
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
    let content = generate_template(Some("proj_test"), None, None);
    assert!(content.contains("id = \"proj_test\""));
    assert!(content.contains("[project]"));
    assert!(content.contains("# port = 4321"));
    assert!(content.contains("#:schema"));
}

#[test]
fn generate_template_with_name_and_workspace() {
    let content = generate_template(Some("proj_test"), Some("my-app"), Some("team-x"));
    assert!(content.contains("id = \"proj_test\""));
    assert!(content.contains("name = \"my-app\""));
    assert!(content.contains("workspace = \"team-x\""));
}

#[test]
fn generate_template_without_project_id() {
    let content = generate_template(None, None, None);
    assert!(content.contains("# id = \"\""));
    assert!(content.contains("[project]"));
}

#[test]
fn resolve_project_id_explicit_wins() {
    let mut config = ProjectConfig::default();
    config.project.id = Some("from_config".into());
    let result = resolve_project_id(Some("explicit_id"), &config).unwrap();
    assert_eq!(result, "explicit_id");
}

#[test]
fn resolve_project_id_from_config() {
    let mut config = ProjectConfig::default();
    config.project.id = Some("from_config".into());
    let result = resolve_project_id(None, &config).unwrap();
    assert_eq!(result, "from_config");
}

#[test]
fn resolve_project_id_no_source_fails() {
    let config = ProjectConfig::default();
    let result = resolve_project_id(None, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no project specified"), "got: {err}");
}

#[test]
fn save_or_update_creates_new_file() {
    let dir = tempfile::tempdir().unwrap();
    save_or_update(dir.path(), "proj_new", None, None).unwrap();

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

    save_or_update(dir.path(), "proj_new", None, None).unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_new"));
    assert_eq!(config.dev_port(), 3000);
}

#[test]
fn save_or_update_noop_when_same_id() {
    let dir = tempfile::tempdir().unwrap();
    let original = "[project]\nid = \"proj_same\"\n\n[dev]\nport = 5000\n";
    std::fs::write(dir.path().join("onreza.toml"), original).unwrap();

    save_or_update(dir.path(), "proj_same", None, None).unwrap();

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

    save_or_update(dir.path(), "proj_new", None, None).unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_new"));
    assert_eq!(config.project.name.as_deref(), Some("my-app"));
    assert_eq!(config.dev_port(), 3000);
}

#[test]
fn save_or_update_adds_project_section_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "[dev]\nport = 3000\n").unwrap();

    save_or_update(dir.path(), "proj_new", None, None).unwrap();

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

    save_or_update(dir.path(), "proj_new", None, None).unwrap();

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

    save_or_update(dir.path(), "proj_new", None, None).unwrap();

    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    assert!(content.contains("# My project config"));
    assert!(content.contains("# Project ID"));
    assert!(content.contains("id = \"proj_new\""));
}

#[test]
fn save_or_update_fails_on_corrupt_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "{{invalid}}").unwrap();

    let result = save_or_update(dir.path(), "proj_new", None, None);
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

#[test]
fn save_or_update_replaces_commented_out_fields() {
    let dir = tempfile::tempdir().unwrap();
    // Simulate what scaffold_local creates (template with commented-out fields)
    let template = generate_template(None, None, None);
    std::fs::write(dir.path().join("onreza.toml"), &template).unwrap();

    save_or_update(dir.path(), "proj_abc", Some("my-app"), Some("ws-1")).unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_abc"));
    assert_eq!(config.project.name.as_deref(), Some("my-app"));
    assert_eq!(config.project.workspace.as_deref(), Some("ws-1"));

    // Verify the file doesn't contain duplicated fields
    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    assert_eq!(
        content.matches("\nid = ").count(),
        1,
        "id should appear once, got:\n{content}"
    );
    // Use line-by-line check to avoid matching "db_name = " as substring
    let name_lines = content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("name = ") || t.starts_with("# name = ")
        })
        .count();
    assert_eq!(name_lines, 1, "name should appear once, got:\n{content}");
}

#[test]
fn resolve_project_id_rejects_empty_string() {
    let config = ProjectConfig::default();
    let result = resolve_project_id(Some(""), &config);
    assert!(result.is_err());

    let mut config_with_empty = ProjectConfig::default();
    config_with_empty.project.id = Some(String::new());
    let result = resolve_project_id(None, &config_with_empty);
    assert!(result.is_err());
}

#[test]
fn toml_values_are_escaped() {
    let content = generate_template(Some("proj_1"), Some("my \"app\""), None);
    assert!(content.contains(r#"name = "my \"app\"""#));

    // Verify it round-trips through TOML parser
    let config: ProjectConfig = toml::from_str(&content).unwrap();
    assert_eq!(config.project.name.as_deref(), Some("my \"app\""));
}

// ── save_framework ────────────────────────────────────────────

#[test]
fn save_framework_adds_to_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\nname = \"my-app\"\n\n[dev]\nport = 3000\n",
    )
    .unwrap();

    save_framework(dir.path(), "nextjs").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.framework.as_deref(), Some("nextjs"));
    // Other fields preserved
    assert_eq!(config.project.id.as_deref(), Some("proj_1"));
    assert_eq!(config.dev.port, Some(3000));
}

#[test]
fn save_framework_noop_when_same() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\nframework = \"astro\"\n",
    )
    .unwrap();

    save_framework(dir.path(), "astro").unwrap();

    let content = std::fs::read_to_string(dir.path().join("onreza.toml")).unwrap();
    // Should not duplicate
    assert_eq!(
        content.matches("framework").count(),
        1,
        "framework should appear once: {content}"
    );
}

#[test]
fn save_framework_replaces_existing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\nframework = \"vite\"\n",
    )
    .unwrap();

    save_framework(dir.path(), "nextjs").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.framework.as_deref(), Some("nextjs"));
}

#[test]
fn save_framework_noop_when_no_toml() {
    let dir = tempfile::tempdir().unwrap();
    // No onreza.toml exists — should do nothing
    save_framework(dir.path(), "nextjs").unwrap();
    assert!(!dir.path().join("onreza.toml").exists());
}

#[test]
fn save_framework_handles_commented_field() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\n# framework = \"\"\n",
    )
    .unwrap();

    save_framework(dir.path(), "nuxt").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.framework.as_deref(), Some("nuxt"));
}

#[test]
fn load_config_with_framework() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\nname = \"app\"\nframework = \"astro\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.framework.as_deref(), Some("astro"));
}

// ── deploy app ───────────────────────────────────────────────

#[test]
fn load_config_with_deploy_app() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "[deploy]\napp = \"web\"\n").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.deploy_app(), Some("web"));
}

#[test]
fn deploy_app_absent_by_default() {
    let config = ProjectConfig::default();
    assert_eq!(config.deploy_app(), None);
}

// ── health_check_path ────────────────────────────────────────

#[test]
fn health_check_path_http_string() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[deploy]\nhealth_check_path = \"/health\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(
        config.health_check_path(),
        Some(&HealthCheckPathConfig::Http("/health".to_string()))
    );
}

#[test]
fn health_check_path_tcp_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[deploy]\nhealth_check_path = false\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(
        config.health_check_path(),
        Some(&HealthCheckPathConfig::Tcp)
    );
}

#[test]
fn health_check_path_absent() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "[deploy]\n").unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.health_check_path(), None);
}

#[test]
fn health_check_path_must_start_with_slash() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[deploy]\nhealth_check_path = \"health\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("must start with '/'"), "got: {msg}");
}

#[test]
fn health_check_path_rejects_query_string() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[deploy]\nhealth_check_path = \"/health?verbose=true\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("query or fragment"), "got: {msg}");
}

#[test]
fn health_check_path_rejects_parent_traversal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[deploy]\nhealth_check_path = \"/../../etc/passwd\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("must not contain '..'"), "got: {msg}");
}

#[test]
fn health_check_path_true_is_invalid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[deploy]\nhealth_check_path = true\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
}
