use super::detect_package_manager;

#[test]
fn test_detect_package_manager_bun_lockb() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bun.lockb"), "").unwrap();
    assert_eq!(detect_package_manager(dir.path()), "bun");
}

#[test]
fn test_detect_package_manager_bun_lock() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bun.lock"), "").unwrap();
    assert_eq!(detect_package_manager(dir.path()), "bun");
}

#[test]
fn test_detect_package_manager_pnpm() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
    assert_eq!(detect_package_manager(dir.path()), "pnpm");
}

#[test]
fn test_detect_package_manager_yarn() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
    assert_eq!(detect_package_manager(dir.path()), "yarn");
}

#[test]
fn test_detect_package_manager_npm_default() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(detect_package_manager(dir.path()), "npm");
}

#[test]
fn test_detect_package_manager_npm_with_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
    assert_eq!(detect_package_manager(dir.path()), "npm");
}
