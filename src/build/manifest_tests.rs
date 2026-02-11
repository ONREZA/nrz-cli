//! Unit tests for manifest parsing and validation

use std::path::Path;

use super::manifest::{RouteType, load_and_validate, verify_files};

const VALID_MANIFEST: &str = r#"{
    "version": 1,
    "adapter": { "name": "@onreza/adapter-astro", "version": "0.1.0" },
    "framework": { "name": "astro", "version": "4.0.0" },
    "server": { "entry": "server/entry.mjs", "export": "fetch" },
    "assets": { "directory": "client", "prefix": "/_astro/" },
    "routes": [
        { "pattern": "/*", "type": "ssr" }
    ]
}"#;

fn write_manifest(dir: &Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("manifest.json");
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn valid_manifest_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), VALID_MANIFEST);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.version, 1);
    assert_eq!(m.adapter.name, "@onreza/adapter-astro");
    assert_eq!(m.framework.name, "astro");
    assert_eq!(m.server.entry, "server/entry.mjs");
    assert_eq!(m.server.export, "fetch");
    assert_eq!(m.routes.len(), 1);
    assert_eq!(m.routes[0].route_type, RouteType::Ssr);
}

#[test]
fn wrong_version() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace(r#""version": 1"#, r#""version": 2"#);
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("unsupported manifest version: 2"));
}

#[test]
fn unknown_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace("@onreza/adapter-astro", "some-other-adapter");
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("unknown adapter"));
}

#[test]
fn wrong_export() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace(r#""export": "fetch""#, r#""export": "default""#);
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("unsupported export format"));
}

#[test]
fn no_routes() {
    let dir = tempfile::tempdir().unwrap();
    // Заменяем весь массив routes на пустой
    let json = VALID_MANIFEST.replace(
        r#""routes": [
        { "pattern": "/*", "type": "ssr" }
    ]"#,
        r#""routes": []"#,
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("no routes defined"));
}

#[test]
fn isr_without_revalidate() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace(
        r#"{ "pattern": "/*", "type": "ssr" }"#,
        r#"{ "pattern": "/blog/*", "type": "isr" }"#,
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("must have revalidate > 0"));
}

#[test]
fn isr_with_revalidate_zero() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace(
        r#"{ "pattern": "/*", "type": "ssr" }"#,
        r#"{ "pattern": "/blog/*", "type": "isr", "revalidate": 0 }"#,
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("must have revalidate > 0"));
}

#[test]
fn isr_with_valid_revalidate() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace(
        r#"{ "pattern": "/*", "type": "ssr" }"#,
        r#"{ "pattern": "/blog/*", "type": "isr", "revalidate": 60 }"#,
    );
    let path = write_manifest(dir.path(), &json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.routes[0].route_type, RouteType::Isr);
    assert_eq!(m.routes[0].revalidate, Some(60));
}

#[test]
fn invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), "not json");
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("failed to parse"));
}

#[test]
fn missing_file() {
    let err = load_and_validate(Path::new("/nonexistent/manifest.json")).unwrap_err();
    assert!(err.to_string().contains("failed to read"));
}

#[test]
fn all_route_types_parse() {
    let dir = tempfile::tempdir().unwrap();
    let json = VALID_MANIFEST.replace(
        r#""routes": [
        { "pattern": "/*", "type": "ssr" }
    ]"#,
        r#""routes": [
        { "pattern": "/", "type": "static" },
        { "pattern": "/app/*", "type": "ssr" },
        { "pattern": "/api/*", "type": "api" },
        { "pattern": "/blog/*", "type": "isr", "revalidate": 60 },
        { "pattern": "/edge/*", "type": "edge_function" }
    ]"#,
    );
    let path = write_manifest(dir.path(), &json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.routes.len(), 5);
    assert_eq!(m.routes[0].route_type, RouteType::Static);
    assert_eq!(m.routes[4].route_type, RouteType::EdgeFunction);
}

#[test]
fn verify_files_ok() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), VALID_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/entry.mjs"), "").unwrap();
    std::fs::create_dir_all(dir.path().join("client")).unwrap();

    verify_files(dir.path(), &m).unwrap();
}

#[test]
fn verify_files_missing_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), VALID_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("client")).unwrap();

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(err.to_string().contains("server entry not found"));
}

#[test]
fn verify_files_missing_assets() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), VALID_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/entry.mjs"), "").unwrap();

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(err.to_string().contains("assets directory not found"));
}

#[test]
fn manifest_with_prerender() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "adapter": { "name": "@onreza/adapter-astro", "version": "0.1.0" },
        "framework": { "name": "astro", "version": "4.0.0" },
        "server": { "entry": "server/entry.mjs", "export": "fetch" },
        "assets": { "directory": "client", "prefix": "/_astro/" },
        "routes": [{ "pattern": "/*", "type": "ssr" }],
        "prerender": {
            "directory": "prerendered",
            "pages": { "/about": { "html": "about.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    assert!(m.prerender.is_some());
    assert_eq!(m.prerender.as_ref().unwrap().directory, "prerendered");
}

#[test]
fn verify_files_missing_prerender_dir() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "adapter": { "name": "@onreza/adapter-astro", "version": "0.1.0" },
        "framework": { "name": "astro", "version": "4.0.0" },
        "server": { "entry": "server/entry.mjs", "export": "fetch" },
        "assets": { "directory": "client", "prefix": "/_astro/" },
        "routes": [{ "pattern": "/*", "type": "ssr" }],
        "prerender": {
            "directory": "prerendered",
            "pages": {}
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/entry.mjs"), "").unwrap();
    std::fs::create_dir_all(dir.path().join("client")).unwrap();

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(err.to_string().contains("prerender directory not found"));
}
