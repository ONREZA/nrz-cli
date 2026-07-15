use serde_json::json;

use nrz::config::EnvVisibility;

use super::env_handler::{
    EnvVar, env_var_matches_environment, normalize_stdin_value, read_set_value, resolve_category,
};

#[test]
fn exact_environment_filter_keeps_all_scope() {
    let variable: EnvVar = serde_json::from_value(json!({
        "key": "GLOBAL",
        "value": "x",
        "isSecret": false,
        "scopeType": "ALL",
        "environments": []
    }))
    .unwrap();

    assert!(env_var_matches_environment(&variable, "environment-1"));
}

#[test]
fn exact_environment_filter_matches_selected_id_only() {
    let variable: EnvVar = serde_json::from_value(json!({
        "key": "PREVIEW_ONLY",
        "value": "x",
        "isSecret": false,
        "scopeType": "SELECTED",
        "environments": [
            { "id": "environment-1", "name": "Preview", "type": "PREVIEW" }
        ]
    }))
    .unwrap();

    assert!(env_var_matches_environment(&variable, "environment-1"));
    assert!(!env_var_matches_environment(&variable, "environment-2"));
}

#[test]
fn category_uses_explicit_flag_or_project_declaration() {
    assert!(resolve_category(true, false, Some(EnvVisibility::Plain)).unwrap());
    assert!(!resolve_category(false, true, Some(EnvVisibility::Sensitive)).unwrap());
    assert!(resolve_category(false, false, Some(EnvVisibility::Sensitive)).unwrap());
    assert!(!resolve_category(false, false, Some(EnvVisibility::Plain)).unwrap());
    assert!(resolve_category(false, false, None).is_err());
    assert!(resolve_category(true, true, None).is_err());
}

#[test]
fn secret_value_rejects_command_line_source() {
    let error = read_set_value(Some("secret-value".to_string()), false, None, true).unwrap_err();
    assert!(error.to_string().contains("--stdin or --from-file"));
}

#[test]
fn file_value_is_exact_and_rejects_nul() {
    let dir = tempfile::tempdir().unwrap();
    let exact = dir.path().join("exact.txt");
    std::fs::write(&exact, "line one\nline two\n").unwrap();
    assert_eq!(
        read_set_value(None, false, exact.to_str(), true).unwrap(),
        "line one\nline two\n"
    );

    let invalid = dir.path().join("invalid.txt");
    std::fs::write(&invalid, b"before\0after").unwrap();
    assert!(read_set_value(None, false, invalid.to_str(), true).is_err());
}

#[test]
fn stdin_value_removes_exactly_one_terminal_newline() {
    assert_eq!(
        normalize_stdin_value(b"value\n\n".to_vec()).unwrap(),
        "value\n"
    );
    assert_eq!(
        normalize_stdin_value(b"value\r\n".to_vec()).unwrap(),
        "value"
    );
    assert!(normalize_stdin_value(vec![0xff]).is_err());
}
