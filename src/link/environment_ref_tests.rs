use super::environment_ref::{EnvironmentRef, EnvironmentType, load, save};

fn make_ref() -> EnvironmentRef {
    EnvironmentRef {
        environment_id: "env_abc123".to_string(),
        environment_type: EnvironmentType::Production,
    }
}

#[test]
fn save_and_load_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let eref = make_ref();

    save(tmp.path(), &eref).unwrap();

    let loaded = load(tmp.path())
        .unwrap()
        .expect("should load saved environment ref");
    assert_eq!(loaded.environment_id, eref.environment_id);
    assert_eq!(loaded.environment_type, eref.environment_type);
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
    std::fs::write(dir.join("environment.json"), "invalid").unwrap();

    let result = load(tmp.path());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("corrupt environment link file"), "got: {err}");
}

#[test]
fn save_creates_onreza_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let eref = make_ref();

    save(tmp.path(), &eref).unwrap();

    assert!(tmp.path().join(".onreza").is_dir());
    assert!(tmp.path().join(".onreza/environment.json").exists());
}

#[test]
fn save_overwrites_existing() {
    let tmp = tempfile::tempdir().unwrap();

    save(
        tmp.path(),
        &EnvironmentRef {
            environment_id: "old_id".into(),
            environment_type: EnvironmentType::Preview,
        },
    )
    .unwrap();

    let new_ref = make_ref();
    save(tmp.path(), &new_ref).unwrap();

    let loaded = load(tmp.path()).unwrap().unwrap();
    assert_eq!(loaded.environment_id, new_ref.environment_id);
    assert_eq!(loaded.environment_type, EnvironmentType::Production);
}

#[test]
fn environment_type_serializes_lowercase() {
    let eref = make_ref();
    let json = serde_json::to_string(&eref).unwrap();
    assert!(json.contains("\"production\""), "got: {json}");
}

#[test]
fn environment_type_deserializes_lowercase() {
    let json = r#"{"environment_id":"id1","environment_type":"preview"}"#;
    let eref: EnvironmentRef = serde_json::from_str(json).unwrap();
    assert_eq!(eref.environment_type, EnvironmentType::Preview);
}

#[test]
fn environment_type_rejects_invalid() {
    let json = r#"{"environment_id":"id1","environment_type":"staging"}"#;
    let result: Result<EnvironmentRef, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn environment_type_display() {
    assert_eq!(EnvironmentType::Production.to_string(), "production");
    assert_eq!(EnvironmentType::Preview.to_string(), "preview");
    assert_eq!(EnvironmentType::Development.to_string(), "development");
}
