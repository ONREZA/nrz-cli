use super::*;

#[test]
fn parse_shorthand_sensitive() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nDATABASE_URL = \"sensitive\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.env.declarations.len(), 1);
    let decl = &config.env.declarations["DATABASE_URL"];
    assert_eq!(decl.visibility, EnvVisibility::Sensitive);
    assert!(decl.required);
}

#[test]
fn parse_shorthand_plain() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nPUBLIC_URL = \"plain\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    let decl = &config.env.declarations["PUBLIC_URL"];
    assert_eq!(decl.visibility, EnvVisibility::Plain);
    assert!(decl.required);
}

#[test]
fn parse_table_form_with_required_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nANALYTICS_ID = { visibility = \"plain\", required = false }\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    let decl = &config.env.declarations["ANALYTICS_ID"];
    assert_eq!(decl.visibility, EnvVisibility::Plain);
    assert!(!decl.required);
}

#[test]
fn parse_table_form_defaults_required_true() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nSECRET_KEY = { visibility = \"sensitive\" }\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    let decl = &config.env.declarations["SECRET_KEY"];
    assert_eq!(decl.visibility, EnvVisibility::Sensitive);
    assert!(decl.required);
}

#[test]
fn parse_mixed_format() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"
[env.declarations]
DATABASE_URL = "sensitive"
PUBLIC_API_URL = "plain"
STRIPE_SECRET_KEY = "sensitive"
ANALYTICS_ID = { visibility = "plain", required = false }
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.env.declarations.len(), 4);
    assert_eq!(
        config.env.declarations["DATABASE_URL"].visibility,
        EnvVisibility::Sensitive
    );
    assert!(config.env.declarations["DATABASE_URL"].required);
    assert_eq!(
        config.env.declarations["PUBLIC_API_URL"].visibility,
        EnvVisibility::Plain
    );
    assert!(config.env.declarations["PUBLIC_API_URL"].required);
    assert!(!config.env.declarations["ANALYTICS_ID"].required);
}

#[test]
fn parse_invalid_visibility_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nFOO = \"invalid\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("unknown variant") || err.contains("unknown visibility"),
        "got: {err}"
    );
}

#[test]
fn parse_table_missing_visibility_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nFOO = { required = false }\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("visibility"), "got: {err}");
}

#[test]
fn empty_env_section() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "[env.declarations]\n").unwrap();

    let config = load(dir.path()).unwrap();
    assert!(config.env.declarations.is_empty());
}

#[test]
fn no_env_section_defaults_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[project]\nid = \"proj_1\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert!(config.env.declarations.is_empty());
}

#[test]
fn required_env_vars_returns_only_required() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"
[env.declarations]
DB_URL = "sensitive"
PUBLIC_URL = "plain"
OPTIONAL = { visibility = "plain", required = false }
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    let mut required = config.required_env_vars();
    required.sort();
    assert_eq!(required, vec!["DB_URL", "PUBLIC_URL"]);
}

#[test]
fn env_visibility_lookup() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nDB_URL = \"sensitive\"\nPUBLIC_URL = \"plain\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(
        config.env_visibility("DB_URL"),
        Some(EnvVisibility::Sensitive)
    );
    assert_eq!(
        config.env_visibility("PUBLIC_URL"),
        Some(EnvVisibility::Plain)
    );
    assert_eq!(config.env_visibility("UNKNOWN"), None);
}

#[test]
fn env_section_does_not_break_existing_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"
[project]
id = "proj_123"

[dev]
port = 3000

[env.declarations]
DATABASE_URL = "sensitive"
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert_eq!(config.project.id.as_deref(), Some("proj_123"));
    assert_eq!(config.dev_port(), 3000);
    assert_eq!(config.env.declarations.len(), 1);
}

#[test]
fn strict_defaults_false() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nDB_URL = \"sensitive\"\n",
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert!(!config.env_strict());
}

#[test]
fn strict_set_true() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        r#"
[env]
strict = true

[env.declarations]
DB_URL = "sensitive"
"#,
    )
    .unwrap();

    let config = load(dir.path()).unwrap();
    assert!(config.env_strict());
    assert_eq!(config.env.declarations.len(), 1);
}

#[test]
fn strict_false_explicit() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("onreza.toml"), "[env]\nstrict = false\n").unwrap();

    let config = load(dir.path()).unwrap();
    assert!(!config.env_strict());
    assert!(config.env.declarations.is_empty());
}

#[test]
fn invalid_env_var_name_with_spaces_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\n\"HELLO WORLD\" = \"sensitive\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("invalid env var name"), "got: {err}");
}

#[test]
fn invalid_env_var_name_lowercase_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\nmy_var = \"sensitive\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("UPPER_SNAKE_CASE"), "got: {err}");
}

#[test]
fn invalid_env_var_name_starts_with_digit_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("onreza.toml"),
        "[env.declarations]\n\"3RD_PARTY_KEY\" = \"sensitive\"\n",
    )
    .unwrap();

    let result = load(dir.path());
    assert!(result.is_err());
}

#[test]
fn visibility_display() {
    assert_eq!(EnvVisibility::Plain.to_string(), "plain");
    assert_eq!(EnvVisibility::Sensitive.to_string(), "sensitive");
    assert_eq!(EnvVisibility::Sensitive.as_str(), "sensitive");
}
