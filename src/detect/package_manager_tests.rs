use super::package_json::PackageJson;
use super::package_manager::*;
use super::types::PackageManagerType;

#[test]
fn detect_from_package_manager_field_pnpm() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager": "pnpm@9.1.0"}"#,
    )
    .unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Pnpm);
    assert_eq!(pm.version.as_deref(), Some("9.1.0"));
}

#[test]
fn detect_from_package_manager_field_yarn() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager": "yarn@4.0.0"}"#,
    )
    .unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Yarn);
    assert_eq!(pm.version.as_deref(), Some("4.0.0"));
}

#[test]
fn detect_from_package_manager_field_no_version() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager": "bun@"}"#,
    )
    .unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Bun);
    assert!(pm.version.is_none());
}

#[test]
fn detect_from_bun_lock() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();
    std::fs::write(dir.path().join("bun.lock"), "").unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Bun);
    assert_eq!(pm.lockfile.as_deref(), Some("bun.lock"));
}

#[test]
fn detect_from_bun_lockb() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();
    std::fs::write(dir.path().join("bun.lockb"), "").unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Bun);
    assert_eq!(pm.lockfile.as_deref(), Some("bun.lockb"));
}

#[test]
fn detect_from_pnpm_lock() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Pnpm);
}

#[test]
fn detect_from_yarn_lock() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Yarn);
}

#[test]
fn detect_from_package_lock_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Npm);
    assert_eq!(pm.lockfile.as_deref(), Some("package-lock.json"));
}

#[test]
fn default_npm_with_package_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{"name":"t"}"#).unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Npm);
    assert!(pm.lockfile.is_none());
}

#[test]
fn no_package_json_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(detect_package_manager(dir.path(), None).is_none());
}

#[test]
fn package_manager_field_takes_priority_over_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager": "pnpm@9.0.0"}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    let pm = detect_package_manager(dir.path(), Some(&pkg)).unwrap();
    assert_eq!(pm.pm_type, PackageManagerType::Pnpm);
}

#[test]
fn install_command_variants() {
    assert_eq!(install_command(PackageManagerType::Npm), "npm install");
    assert_eq!(install_command(PackageManagerType::Yarn), "yarn install");
    assert_eq!(install_command(PackageManagerType::Pnpm), "pnpm install");
    assert_eq!(install_command(PackageManagerType::Bun), "bun install");
}

#[test]
fn build_command_variants() {
    assert_eq!(
        build_command(PackageManagerType::Npm, "build"),
        "npm run build"
    );
    assert_eq!(
        build_command(PackageManagerType::Yarn, "build"),
        "yarn build"
    );
    assert_eq!(
        build_command(PackageManagerType::Pnpm, "build"),
        "pnpm build"
    );
    assert_eq!(
        build_command(PackageManagerType::Bun, "build"),
        "bun run build"
    );
}

#[test]
fn build_command_empty_script() {
    assert_eq!(build_command(PackageManagerType::Npm, ""), "");
}
