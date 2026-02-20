use super::detect_output_dir;

#[test]
fn framework_dirs_checked_before_config_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // Both dirs exist, but framework-specific should be preferred
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[".next"]).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
}

#[test]
fn manifest_dir_wins_over_plain_dir() {
    let dir = tempfile::tempdir().unwrap();
    // "dist" exists as plain dir, ".output" has .onreza/ inside
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".output/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist", ".output"], &[]).unwrap();
    assert_eq!(found.file_name().unwrap(), ".output");
    assert!(has_manifest);
}

#[test]
fn dedup_preserves_order() {
    let dir = tempfile::tempdir().unwrap();
    // "dist" is in both framework_dirs and config_dirs
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &["dist", "build"]).unwrap();
    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn error_lists_all_checked_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // No dirs exist
    let err = detect_output_dir(dir.path(), &["build"], &[".next", "out"]).unwrap_err();
    let msg = err.to_string();
    // framework dirs + config dirs should all appear in error
    assert!(msg.contains(".next/"), "error should list .next: {msg}");
    assert!(msg.contains("out/"), "error should list out: {msg}");
    assert!(msg.contains("build/"), "error should list build: {msg}");
}

#[test]
fn empty_framework_dirs_falls_back_to_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[]).unwrap();
    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn framework_manifest_dir_wins_over_config_manifest_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Both have .onreza/, but framework dir should be checked first
    std::fs::create_dir_all(dir.path().join("dist/.onreza")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist"], &[".next"]).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
    assert!(has_manifest);
}
