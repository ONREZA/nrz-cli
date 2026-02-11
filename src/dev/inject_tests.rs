//! Unit tests for JS bootstrap generation

use super::inject::generate_bootstrap;

#[test]
fn bootstrap_contains_port() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("http://127.0.0.1:4322"));
}

#[test]
fn bootstrap_contains_db_path() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("dev.db"));
}

#[test]
fn bootstrap_sets_global() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("globalThis.ONREZA"));
}

#[test]
fn bootstrap_has_kv_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("/__nrz/kv/"));
}

#[test]
fn bootstrap_has_db_methods() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("/__nrz/db/query"));
    assert!(script.contains("/__nrz/db/batch"));
    assert!(script.contains("/__nrz/db/exec"));
}

#[test]
fn bootstrap_has_context() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("deploymentId"));
    assert!(script.contains("clientIp"));
}

#[test]
fn bootstrap_different_ports() {
    let dir = tempfile::tempdir().unwrap();
    let s1 = generate_bootstrap(dir.path(), 3000).unwrap();
    let s2 = generate_bootstrap(dir.path(), 5000).unwrap();
    assert!(s1.contains("http://127.0.0.1:3000"));
    assert!(s2.contains("http://127.0.0.1:5000"));
    assert!(!s1.contains("5000"));
    assert!(!s2.contains("3000"));
}

#[test]
fn bootstrap_db_path_is_json_string() {
    let dir = tempfile::tempdir().unwrap();
    let script = generate_bootstrap(dir.path(), 4322).unwrap();
    assert!(script.contains("const DB_PATH = \""));
}
