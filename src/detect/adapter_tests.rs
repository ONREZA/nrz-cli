use super::adapter::*;
use super::package_json::PackageJson;

#[test]
fn detect_adapter_astro() {
    let json = r#"{"dependencies": {"astro": "4.0.0", "@onreza/adapter-astro": "^1.0.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    let adapter = detect_adapter(&pkg).unwrap();
    assert_eq!(adapter.adapter_package, "@onreza/adapter-astro");
    assert_eq!(adapter.adapter_version.as_deref(), Some("^1.0.0"));
}

#[test]
fn detect_adapter_nitro() {
    let json = r#"{"dependencies": {"@onreza/adapter-nitro": "1.2.3"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    let adapter = detect_adapter(&pkg).unwrap();
    assert_eq!(adapter.adapter_package, "@onreza/adapter-nitro");
}

#[test]
fn detect_adapter_in_dev_dependencies() {
    let json = r#"{"devDependencies": {"@onreza/adapter-nextjs": "0.5.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    let adapter = detect_adapter(&pkg).unwrap();
    assert_eq!(adapter.adapter_package, "@onreza/adapter-nextjs");
}

#[test]
fn detect_adapter_sveltekit() {
    let json = r#"{"dependencies": {"@onreza/adapter-sveltekit": "2.0.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    let adapter = detect_adapter(&pkg).unwrap();
    assert_eq!(adapter.adapter_package, "@onreza/adapter-sveltekit");
}

#[test]
fn no_adapter_returns_none() {
    let json = r#"{"dependencies": {"next": "14.0.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(detect_adapter(&pkg).is_none());
}

#[test]
fn runtime_package_not_detected_as_adapter() {
    // @onreza/runtime is NOT an adapter — it's the runtime bindings library
    let json = r#"{"dependencies": {"@onreza/runtime": "1.0.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(detect_adapter(&pkg).is_none());
}

#[test]
fn unknown_onreza_package_not_detected() {
    let json = r#"{"dependencies": {"@onreza/cli": "1.0.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    assert!(detect_adapter(&pkg).is_none());
}

#[test]
fn future_adapter_auto_detected() {
    // Any new adapter following the @onreza/adapter-* pattern is auto-detected
    let json = r#"{"dependencies": {"@onreza/adapter-remix": "0.1.0"}}"#;
    let pkg: PackageJson = serde_json::from_str(json).unwrap();
    let adapter = detect_adapter(&pkg).unwrap();
    assert_eq!(adapter.adapter_package, "@onreza/adapter-remix");
}
