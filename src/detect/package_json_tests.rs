use super::package_json::{PackageJson, Workspaces};

#[test]
fn parse_minimal_package_json() {
    let json = r#"{"name": "my-app"}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert_eq!(pkg.name.as_deref(), Some("my-app"));
    assert!(pkg.dependencies.is_empty());
    assert!(pkg.dev_dependencies.is_empty());
}

#[test]
fn parse_with_dependencies() {
    let json = r#"{
        "name": "test",
        "dependencies": {"next": "14.0.0", "react": "18.0.0"},
        "devDependencies": {"typescript": "5.0.0"}
    }"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(pkg.has_dependency("next"));
    assert!(pkg.has_dependency("react"));
    assert!(pkg.has_dependency("typescript"));
    assert!(!pkg.has_dependency("vue"));
}

#[test]
fn dependency_version_lookup() {
    let json = r#"{
        "dependencies": {"next": "^14.0.0"},
        "devDependencies": {"eslint": "^8.0.0"}
    }"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert_eq!(pkg.dependency_version("next"), Some("^14.0.0"));
    assert_eq!(pkg.dependency_version("eslint"), Some("^8.0.0"));
    assert_eq!(pkg.dependency_version("missing"), None);
}

#[test]
fn parse_package_manager_field() {
    let json = r#"{"packageManager": "pnpm@9.0.0"}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert_eq!(pkg.package_manager.as_deref(), Some("pnpm@9.0.0"));
}

#[test]
fn parse_workspaces_array() {
    let json = r#"{"workspaces": ["packages/*", "apps/*"]}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(pkg.is_monorepo());
    let ws = pkg.workspaces.to_vec();
    assert_eq!(ws.len(), 2);
    assert_eq!(ws[0], "packages/*");
}

#[test]
fn parse_workspaces_object() {
    let json = r#"{"workspaces": {"packages": ["packages/*"]}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(pkg.is_monorepo());
    let ws = pkg.workspaces.to_vec();
    assert_eq!(ws.len(), 1);
}

#[test]
fn no_workspaces_means_not_monorepo() {
    let json = r#"{"name": "simple"}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(!pkg.is_monorepo());
}

#[test]
fn empty_workspaces_array() {
    let json = r#"{"workspaces": []}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(!pkg.is_monorepo());
}

#[test]
fn load_from_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name":"test","dependencies":{"astro":"4.0.0"}}"#,
    )
    .unwrap();
    let pkg = PackageJson::load(dir.path()).unwrap();
    assert_eq!(pkg.name.as_deref(), Some("test"));
    assert!(pkg.has_dependency("astro"));
}

#[test]
fn load_missing_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(PackageJson::load(dir.path()).is_none());
}

#[test]
fn load_invalid_json_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("package.json"), "not json{").unwrap();
    assert!(PackageJson::load(dir.path()).is_none());
}

#[test]
fn workspaces_none_variant() {
    let ws = Workspaces::None;
    assert!(ws.is_empty());
    assert!(ws.to_vec().is_empty());
}

#[test]
fn parse_scripts() {
    let json = r#"{"scripts": {"build": "next build", "dev": "next dev"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert_eq!(
        pkg.scripts.get("build").map(|s| s.as_str()),
        Some("next build")
    );
}
