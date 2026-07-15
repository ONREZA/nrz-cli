use std::path::Path;

use super::consume_process_token;

#[test]
fn consumes_and_removes_ephemeral_token_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runner-token");
    std::fs::write(&path, "nrz_runner_token\n").unwrap();

    let token = consume_process_token(None, Some(&path)).unwrap();

    assert_eq!(token.as_deref(), Some("nrz_runner_token"));
    assert!(!path.exists());
}

#[test]
fn removes_token_file_before_rejecting_conflicting_sources() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runner-token");
    std::fs::write(&path, "nrz_runner_token").unwrap();

    let error = consume_process_token(Some("nrz_explicit".to_string()), Some(&path)).unwrap_err();

    assert!(error.to_string().contains("conflicts"));
    assert!(!path.exists());
}

#[test]
fn leaves_explicit_token_unchanged_without_file() {
    assert_eq!(
        consume_process_token(Some("nrz_explicit".to_string()), None).unwrap(),
        Some("nrz_explicit".to_string())
    );
    assert_eq!(consume_process_token(None, None).unwrap(), None);
}

#[test]
fn rejects_invalid_token_file_after_removal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runner-token");
    std::fs::write(&path, "line-one\nline-two").unwrap();

    assert!(consume_process_token(None, Some(Path::new(&path))).is_err());
    assert!(!path.exists());
}
