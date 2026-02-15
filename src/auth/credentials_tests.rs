use super::credentials::{Credentials, load_from, remove_at, save_to};

fn make_creds() -> Credentials {
    Credentials {
        access_token: "nrz_test_token_123".to_string(),
        workspace_slug: "my-workspace".to_string(),
        workspace_name: "My Workspace".to_string(),
    }
}

fn creds_path(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    tmp.path().join("nrz").join("credentials.json")
}

#[test]
fn save_and_load_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = creds_path(&tmp);
    let creds = make_creds();

    save_to(&path, &creds).unwrap();

    let loaded = load_from(&path).expect("should load saved credentials");
    assert_eq!(loaded.access_token, creds.access_token);
    assert_eq!(loaded.workspace_slug, creds.workspace_slug);
    assert_eq!(loaded.workspace_name, creds.workspace_name);
}

#[test]
fn load_returns_none_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let path = creds_path(&tmp);
    assert!(load_from(&path).is_none());
}

#[test]
fn remove_deletes_credentials() {
    let tmp = tempfile::tempdir().unwrap();
    let path = creds_path(&tmp);

    save_to(&path, &make_creds()).unwrap();
    assert!(load_from(&path).is_some());

    remove_at(&path).unwrap();
    assert!(load_from(&path).is_none());
}

#[test]
fn remove_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = creds_path(&tmp);
    remove_at(&path).unwrap();
    remove_at(&path).unwrap();
}

#[test]
fn load_returns_none_on_corrupt_json() {
    let tmp = tempfile::tempdir().unwrap();
    let path = creds_path(&tmp);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "not json").unwrap();

    assert!(load_from(&path).is_none());
}

#[cfg(unix)]
#[test]
fn save_sets_permissions_600() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let path = creds_path(&tmp);

    save_to(&path, &make_creds()).unwrap();

    let perms = std::fs::metadata(&path).unwrap().permissions();
    assert_eq!(perms.mode() & 0o777, 0o600);
}

#[test]
fn save_creates_parent_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("deep").join("nested").join("creds.json");

    save_to(&path, &make_creds()).unwrap();
    assert!(path.exists());
}
