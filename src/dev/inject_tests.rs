//! Unit tests for JS bootstrap generation

use super::{inject::generate_bootstrap, prepare_data_dir, write_bootstrap};

#[test]
fn bootstrap_contains_port() {
    let script = generate_bootstrap("http://127.0.0.1:4322", "test-token").unwrap();
    assert!(script.contains("http://127.0.0.1:4322"));
}

#[test]
fn bootstrap_sets_global() {
    let script = generate_bootstrap("http://127.0.0.1:4322", "test-token").unwrap();
    assert!(script.contains("globalThis.ONREZA"));
}

#[test]
fn bootstrap_has_kv_proxy() {
    let script = generate_bootstrap("http://127.0.0.1:4322", "test-token").unwrap();
    assert!(script.contains("/__nrz/kv/"));
    assert!(script.contains("\"x-nrz-emulator-token\": NRZ_EMULATOR_TOKEN"));
}

#[test]
fn bootstrap_no_db_references() {
    let script = generate_bootstrap("http://127.0.0.1:4322", "test-token").unwrap();
    assert!(!script.contains("/__nrz/db/"));
    assert!(!script.contains("DB_PATH"));
}

#[test]
fn bootstrap_has_context() {
    let script = generate_bootstrap("http://127.0.0.1:4322", "test-token").unwrap();
    assert!(script.contains("deploymentId"));
    assert!(script.contains("clientIp"));
}

#[test]
fn bootstrap_different_ports() {
    let s1 = generate_bootstrap("http://127.0.0.1:3000", "token-one").unwrap();
    let s2 = generate_bootstrap("http://127.0.0.1:5000", "token-two").unwrap();
    assert!(s1.contains("http://127.0.0.1:3000"));
    assert!(s2.contains("http://127.0.0.1:5000"));
    assert!(!s1.contains("5000"));
    assert!(!s2.contains("3000"));
}

#[cfg(unix)]
#[test]
fn bootstrap_file_is_private() {
    use std::os::unix::fs::PermissionsExt;

    let bootstrap = write_bootstrap("secret-token").unwrap();
    let path = bootstrap.path();

    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[test]
fn data_dir_stays_inside_project() {
    let project = tempfile::tempdir().unwrap();
    let data_dir = prepare_data_dir(project.path(), ".onreza/data").unwrap();

    assert_eq!(data_dir, project.path().join(".onreza/data"));
    assert!(data_dir.is_dir());
    assert!(prepare_data_dir(project.path(), "../outside").is_err());
    assert!(prepare_data_dir(project.path(), "/tmp/nrz-data").is_err());
}

#[cfg(unix)]
#[test]
fn data_dir_rejects_symbolic_link_components() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join(".onreza")).unwrap();

    let result = prepare_data_dir(project.path(), ".onreza/data");

    assert!(result.is_err());
    assert!(!outside.path().join("data").exists());
}

#[cfg(unix)]
#[test]
fn existing_data_dir_permissions_are_preserved() {
    use std::os::unix::fs::PermissionsExt;

    let project = tempfile::tempdir().unwrap();
    let data_dir = project.path().join("custom-data");
    std::fs::create_dir(&data_dir).unwrap();
    std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    prepare_data_dir(project.path(), "custom-data").unwrap();

    assert_eq!(
        std::fs::metadata(data_dir).unwrap().permissions().mode() & 0o777,
        0o755
    );
}
