//! Unit tests for JS bootstrap generation

use super::{inject::generate_bootstrap, write_bootstrap};

#[test]
fn bootstrap_contains_port() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322, "test-token").unwrap();
    assert!(script.contains("http://127.0.0.1:4322"));
}

#[test]
fn bootstrap_sets_global() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322, "test-token").unwrap();
    assert!(script.contains("globalThis.ONREZA"));
}

#[test]
fn bootstrap_has_kv_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322, "test-token").unwrap();
    assert!(script.contains("/__nrz/kv/"));
    assert!(script.contains("\"x-nrz-emulator-token\": NRZ_EMULATOR_TOKEN"));
}

#[test]
fn bootstrap_no_db_references() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322, "test-token").unwrap();
    assert!(!script.contains("/__nrz/db/"));
    assert!(!script.contains("DB_PATH"));
}

#[test]
fn bootstrap_has_context() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322, "test-token").unwrap();
    assert!(script.contains("deploymentId"));
    assert!(script.contains("clientIp"));
}

#[test]
fn bootstrap_different_ports() {
    let dir = tempfile::tempdir().unwrap();
    let s1 = generate_bootstrap(dir.path(), 3000, "token-one").unwrap();
    let s2 = generate_bootstrap(dir.path(), 5000, "token-two").unwrap();
    assert!(s1.contains("http://127.0.0.1:3000"));
    assert!(s2.contains("http://127.0.0.1:5000"));
    assert!(!s1.contains("5000"));
    assert!(!s2.contains("3000"));
}

#[cfg(unix)]
#[test]
fn bootstrap_file_is_private() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = write_bootstrap(dir.path(), "secret-token").unwrap();

    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
