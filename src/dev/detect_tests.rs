//! Unit tests for framework detection

use std::path::Path;

use super::detect::{FrameworkName, detect_framework};

fn write_pkg(dir: &Path, content: &str) {
    std::fs::write(dir.join("package.json"), content).unwrap();
}

#[test]
fn detect_astro() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{"dependencies":{"astro":"^4.0"}}"#);
    let fw = detect_framework(dir.path()).unwrap();
    assert!(matches!(fw.name, FrameworkName::Astro));
    assert_eq!(fw.dev_command, "astro dev");
}

#[test]
fn detect_nuxt() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{"dependencies":{"nuxt":"^3.0"}}"#);
    let fw = detect_framework(dir.path()).unwrap();
    assert!(matches!(fw.name, FrameworkName::Nuxt));
    assert_eq!(fw.dev_command, "nuxt dev");
}

#[test]
fn detect_sveltekit() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(
        dir.path(),
        r#"{"devDependencies":{"@sveltejs/kit":"^2.0"}}"#,
    );
    let fw = detect_framework(dir.path()).unwrap();
    assert!(matches!(fw.name, FrameworkName::SvelteKit));
    assert_eq!(fw.dev_command, "vite dev");
}

#[test]
fn detect_nitro() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{"dependencies":{"nitropack":"^2.0"}}"#);
    let fw = detect_framework(dir.path()).unwrap();
    assert!(matches!(fw.name, FrameworkName::Nitro));
    assert_eq!(fw.dev_command, "nitro dev");
}

#[test]
fn detect_from_dev_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{"devDependencies":{"astro":"^4.0"}}"#);
    let fw = detect_framework(dir.path()).unwrap();
    assert!(matches!(fw.name, FrameworkName::Astro));
}

#[test]
fn detect_missing_package_json() {
    let dir = tempfile::tempdir().unwrap();
    let err = detect_framework(dir.path()).unwrap_err();
    assert!(err.to_string().contains("package.json"));
}

#[test]
fn detect_unknown_framework() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), r#"{"dependencies":{"express":"^4.0"}}"#);
    let err = detect_framework(dir.path()).unwrap_err();
    assert!(err.to_string().contains("could not detect framework"));
}

#[test]
fn detect_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(dir.path(), "not json");
    let err = detect_framework(dir.path()).unwrap_err();
    assert!(err.to_string().contains("invalid package.json"));
}

#[test]
fn detect_priority_astro_over_nitro() {
    let dir = tempfile::tempdir().unwrap();
    write_pkg(
        dir.path(),
        r#"{"dependencies":{"astro":"^4.0","nitropack":"^2.0"}}"#,
    );
    let fw = detect_framework(dir.path()).unwrap();
    assert!(matches!(fw.name, FrameworkName::Astro));
}
