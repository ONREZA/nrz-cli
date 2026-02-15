use super::project_ref::{ProjectRef, load, save};

fn make_ref() -> ProjectRef {
    ProjectRef {
        project_id: "proj_abc123".to_string(),
        project_name: "My Cool App".to_string(),
    }
}

#[test]
fn save_and_load_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let pref = make_ref();

    save(tmp.path(), &pref).unwrap();

    let loaded = load(tmp.path())
        .unwrap()
        .expect("should load saved project ref");
    assert_eq!(loaded.project_id, pref.project_id);
    assert_eq!(loaded.project_name, pref.project_name);
}

#[test]
fn load_returns_none_when_missing() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(load(tmp.path()).unwrap().is_none());
}

#[test]
fn load_returns_error_on_corrupt_json() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(".onreza");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("project.json"), "invalid").unwrap();

    let result = load(tmp.path());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("corrupt project link file"), "got: {err}");
}

#[test]
fn save_creates_onreza_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let pref = make_ref();

    save(tmp.path(), &pref).unwrap();

    assert!(tmp.path().join(".onreza").is_dir());
    assert!(tmp.path().join(".onreza/project.json").exists());
}

#[test]
fn save_overwrites_existing() {
    let tmp = tempfile::tempdir().unwrap();

    save(
        tmp.path(),
        &ProjectRef {
            project_id: "old_id".into(),
            project_name: "Old".into(),
        },
    )
    .unwrap();

    let new_ref = make_ref();
    save(tmp.path(), &new_ref).unwrap();

    let loaded = load(tmp.path()).unwrap().unwrap();
    assert_eq!(loaded.project_id, new_ref.project_id);
}
