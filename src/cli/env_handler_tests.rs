use serde_json::json;

use super::env_handler::{
    EnvVar, env_list_url, env_var_matches_targets, is_secret_by_name, parse_dotenv,
    parse_dotenv_value,
};

// ── parse_dotenv ─────────────────────────────────────────────

#[test]
fn parse_dotenv_basic_key_value() {
    let r = parse_dotenv("FOO=bar\nBAZ=qux\n");
    assert_eq!(r.vars.len(), 2);
    assert_eq!(r.vars[0].key, "FOO");
    assert_eq!(r.vars[0].value, "bar");
    assert_eq!(r.vars[1].key, "BAZ");
    assert_eq!(r.vars[1].value, "qux");
    assert_eq!(r.skipped_lines, 0);
}

#[test]
fn parse_dotenv_skips_comments() {
    let r = parse_dotenv("# comment\nFOO=bar\n# another comment\n");
    assert_eq!(r.vars.len(), 1);
    assert_eq!(r.vars[0].key, "FOO");
}

#[test]
fn parse_dotenv_skips_empty_lines() {
    let r = parse_dotenv("\n\nFOO=bar\n\n");
    assert_eq!(r.vars.len(), 1);
}

#[test]
fn parse_dotenv_export_prefix() {
    let r = parse_dotenv("export FOO=bar\nexport BAZ=qux\n");
    assert_eq!(r.vars.len(), 2);
    assert_eq!(r.vars[0].key, "FOO");
    assert_eq!(r.vars[0].value, "bar");
}

#[test]
fn parse_dotenv_export_without_space_is_not_stripped() {
    let r = parse_dotenv("exportFOO=bar\n");
    assert_eq!(r.vars.len(), 1);
    assert_eq!(r.vars[0].key, "exportFOO");
    assert_eq!(r.vars[0].value, "bar");
}

#[test]
fn parse_dotenv_double_quoted() {
    let r = parse_dotenv("FOO=\"hello world\"\n");
    assert_eq!(r.vars[0].value, "hello world");
}

#[test]
fn parse_dotenv_single_quoted() {
    let r = parse_dotenv("FOO='hello world'\n");
    assert_eq!(r.vars[0].value, "hello world");
}

#[test]
fn parse_dotenv_inline_comment_stripped() {
    let r = parse_dotenv("FOO=bar # inline comment\n");
    assert_eq!(r.vars[0].value, "bar");
}

#[test]
fn parse_dotenv_empty_value() {
    let r = parse_dotenv("FOO=\n");
    assert_eq!(r.vars.len(), 1);
    assert_eq!(r.vars[0].value, "");
}

#[test]
fn parse_dotenv_skips_no_equals() {
    let r = parse_dotenv("NOTAVAR\nFOO=bar\n");
    assert_eq!(r.vars.len(), 1);
    assert_eq!(r.vars[0].key, "FOO");
    assert_eq!(r.skipped_lines, 1);
}

#[test]
fn parse_dotenv_skips_key_with_whitespace() {
    let r = parse_dotenv("BAD KEY=value\nGOOD=ok\n");
    assert_eq!(r.vars.len(), 1);
    assert_eq!(r.vars[0].key, "GOOD");
    assert_eq!(r.skipped_lines, 1);
}

// ── parse_dotenv_value ───────────────────────────────────────

#[test]
fn value_double_quoted_escape_newline() {
    assert_eq!(parse_dotenv_value(r#""line1\nline2""#), "line1\nline2");
}

#[test]
fn value_double_quoted_escape_tab() {
    assert_eq!(parse_dotenv_value(r#""col1\tcol2""#), "col1\tcol2");
}

#[test]
fn value_double_quoted_escaped_quote() {
    assert_eq!(parse_dotenv_value(r#""say \"hi\"""#), r#"say "hi""#);
}

#[test]
fn value_double_quoted_escaped_backslash() {
    assert_eq!(parse_dotenv_value(r#""path\\nfile""#), r"path\nfile");
}

#[test]
fn value_double_quoted_double_backslash() {
    assert_eq!(parse_dotenv_value(r#""a\\b""#), r"a\b");
}

#[test]
fn value_single_quoted_no_escapes() {
    assert_eq!(
        parse_dotenv_value(r"'\n is not a newline'"),
        r"\n is not a newline"
    );
}

#[test]
fn value_unquoted_trims_inline_comment() {
    assert_eq!(parse_dotenv_value("value # comment"), "value");
}

#[test]
fn value_unquoted_no_comment() {
    assert_eq!(parse_dotenv_value("simple"), "simple");
}

// ── is_secret_by_name ────────────────────────────────────────

#[test]
fn secret_detection_token() {
    assert!(is_secret_by_name("API_TOKEN"));
    assert!(is_secret_by_name("AUTH_TOKEN"));
}

#[test]
fn secret_detection_secret() {
    assert!(is_secret_by_name("CLIENT_SECRET"));
    assert!(is_secret_by_name("MY_SECRET_VALUE"));
}

#[test]
fn secret_detection_password() {
    assert!(is_secret_by_name("DB_PASSWORD"));
}

#[test]
fn secret_detection_key() {
    assert!(is_secret_by_name("PRIVATE_KEY"));
    assert!(is_secret_by_name("API_KEY"));
}

#[test]
fn secret_detection_non_secret() {
    assert!(!is_secret_by_name("DATABASE_URL"));
    assert!(!is_secret_by_name("NODE_ENV"));
    assert!(!is_secret_by_name("PORT"));
    assert!(!is_secret_by_name("APP_NAME"));
}

#[test]
fn secret_detection_case_insensitive() {
    assert!(is_secret_by_name("api_key"));
    assert!(is_secret_by_name("db_password"));
    assert!(is_secret_by_name("auth_token"));
    assert!(is_secret_by_name("client_secret"));
}

// ── server env endpoint contract ─────────────────────────────

#[test]
fn env_list_url_sends_single_target_only() {
    assert_eq!(
        env_list_url("proj", &["PRODUCTION".to_string()], None),
        "/v1/projects/proj/env?target=PRODUCTION"
    );
    assert_eq!(
        env_list_url(
            "proj",
            &["PRODUCTION".to_string(), "PREVIEW".to_string()],
            None
        ),
        "/v1/projects/proj/env"
    );
}

#[test]
fn env_list_url_preserves_keys_filter() {
    assert_eq!(
        env_list_url("proj", &[], Some("API_KEY,DB_URL")),
        "/v1/projects/proj/env?keys=API_KEY,DB_URL"
    );
    assert_eq!(
        env_list_url("proj", &["PREVIEW".to_string()], Some("API_KEY")),
        "/v1/projects/proj/env?target=PREVIEW&keys=API_KEY"
    );
}

#[test]
fn multi_target_filter_matches_server_single_target_semantics() {
    let all_var: EnvVar = serde_json::from_value(json!({
        "key": "GLOBAL",
        "value": "x",
        "isSecret": false,
        "scopeType": "ALL",
        "environments": []
    }))
    .unwrap();
    let preview_var: EnvVar = serde_json::from_value(json!({
        "key": "PREVIEW_ONLY",
        "value": "x",
        "isSecret": false,
        "scopeType": "SELECTED",
        "environments": [{ "id": "env1", "name": "Preview", "type": "PREVIEW" }]
    }))
    .unwrap();
    let prod_var: EnvVar = serde_json::from_value(json!({
        "key": "PROD_ONLY",
        "value": "x",
        "isSecret": false,
        "scopeType": "SELECTED",
        "environments": [{ "id": "env2", "name": "Production", "type": "PRODUCTION" }]
    }))
    .unwrap();

    let targets = vec!["PREVIEW".to_string(), "DEVELOPMENT".to_string()];
    assert!(env_var_matches_targets(&all_var, &targets));
    assert!(env_var_matches_targets(&preview_var, &targets));
    assert!(!env_var_matches_targets(&prod_var, &targets));
}
