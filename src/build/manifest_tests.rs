//! Unit tests for manifest parsing and validation (BUILD_OUTPUT_SPEC v1)

use std::path::Path;

use super::manifest::{
    LayerTarget, generate_compute_manifest, generate_nextjs_standalone_manifest,
    generate_static_manifest, load_and_validate, primary_compute_target, validate, verify_files,
};

// ── Fixtures ─────────────────────────────────────────────────

const ASTRO_MANIFEST: &str = r#"{
    "version": 1,
    "layers": [
        { "name": "assets", "target": "STATIC", "directory": "client" },
        { "name": "server", "target": "ISOLATE", "directory": "server",
          "entry": "entry.mjs", "export": "fetch" }
    ],
    "routes": [
        { "pattern": "^/_astro/.*$", "layer": "assets", "priority": 100 },
        { "pattern": "^/.*$", "layer": "server", "priority": 0 }
    ]
}"#;

const NEXTJS_MANIFEST: &str = r#"{
    "version": 1,
    "layers": [
        { "name": "static", "target": "STATIC", "directory": "static" },
        { "name": "server", "target": "COMPUTE", "directory": "standalone",
          "entry": "server.js", "runtime": { "memoryMb": 512 } }
    ],
    "routes": [
        { "pattern": "^/_next/static/.*$", "layer": "static", "priority": 100 },
        { "pattern": "^/.*$", "layer": "server", "priority": 0 }
    ],
    "meta": {
        "adapter": { "name": "@onreza/adapter-nextjs", "version": "1.0.0" },
        "framework": { "name": "nextjs", "version": "15.1.0" }
    }
}"#;

const STATIC_MANIFEST: &str = r#"{
    "version": 1,
    "layers": [
        { "name": "site", "target": "STATIC", "directory": "." }
    ],
    "routes": [
        { "pattern": "^/.*$", "layer": "site" }
    ]
}"#;

fn write_manifest(dir: &Path, content: &str) -> std::path::PathBuf {
    let path = dir.join("manifest.json");
    std::fs::write(&path, content).unwrap();
    path
}

// ── Parsing ───────────────────────────────────────────────────

#[test]
fn valid_astro_manifest_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), ASTRO_MANIFEST);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.version, 1);
    assert_eq!(m.layers.len(), 2);
    assert_eq!(m.layers[0].name, "assets");
    assert_eq!(m.layers[0].target, LayerTarget::Static);
    assert_eq!(m.layers[1].name, "server");
    assert_eq!(m.layers[1].target, LayerTarget::Isolate);
    assert_eq!(m.layers[1].entry.as_deref(), Some("entry.mjs"));
    assert_eq!(m.layers[1].export_format.as_deref(), Some("fetch"));
    assert_eq!(m.routes.len(), 2);
    assert_eq!(m.routes[0].priority, Some(100));
}

#[test]
fn valid_nextjs_manifest_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), NEXTJS_MANIFEST);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.layers.len(), 2);
    assert_eq!(m.layers[1].target, LayerTarget::Compute);
    assert_eq!(m.layers[1].entry.as_deref(), Some("server.js"));
    assert!(m.layers[1].runtime.is_some());
    assert_eq!(m.layers[1].runtime.as_ref().unwrap().memory_mb, Some(512));
    assert!(m.meta.is_some());
}

#[test]
fn valid_pure_static_manifest_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), STATIC_MANIFEST);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.layers.len(), 1);
    assert_eq!(m.layers[0].target, LayerTarget::Static);
    assert!(m.layers[0].entry.is_none());
}

// ── Version ───────────────────────────────────────────────────

#[test]
fn wrong_version() {
    let dir = tempfile::tempdir().unwrap();
    let json = ASTRO_MANIFEST.replace(r#""version": 1"#, r#""version": 2"#);
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("unsupported manifest version: 2"),
        "unexpected error: {err}"
    );
}

// ── Layers ────────────────────────────────────────────────────

#[test]
fn empty_layers() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{"version":1,"layers":[],"routes":[{"pattern":"^/.*$","layer":"x"}]}"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("at least one layer"), "{err}");
}

#[test]
fn duplicate_layer_names() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "server", "target": "STATIC", "directory": "a" },
            { "name": "server", "target": "STATIC", "directory": "b" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "server" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("duplicate layer name: 'server'"),
        "{err}"
    );
}

#[test]
fn directory_path_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "../secret" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn directory_absolute_path_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "/etc/passwd" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn directory_path_traversal_url_encoded_dots() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "%2e%2e/secret" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn directory_path_traversal_double_encoded_dots() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "%252e%252e/secret" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn directory_path_traversal_encoded_slash_not_blocked() {
    let dir = tempfile::tempdir().unwrap();
    // %2f (encoded slash) alone is not flagged — only %2e (encoded dot) triggers the check.
    // A literal ".." segment is still required to form traversal, and that is caught separately.
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "foo%2fbar" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    load_and_validate(&path).unwrap();
}

#[test]
fn directory_path_traversal_null_byte() {
    let dir = tempfile::tempdir().unwrap();
    // JSON \u0000 is decoded by serde_json into a Rust string containing '\0'
    let json = r#"{"version":1,"layers":[{"name":"x","target":"STATIC","directory":"server\u0000.js"}],"routes":[{"pattern":"^/.*$","layer":"x"}]}"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn entry_path_traversal_backslash() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "standalone",
                     "entry": "..\\outside\\server.js" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn static_with_entry_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist", "entry": "index.html" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("STATIC layer 's' must not have 'entry'"),
        "{err}"
    );
}

#[test]
fn static_with_export_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist", "export": "fetch" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("STATIC layer 's' must not have 'export'"),
        "{err}"
    );
}

#[test]
fn isolate_without_entry_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "ISOLATE", "directory": "server", "export": "fetch" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("(target=ISOLATE) requires 'entry'"),
        "{err}"
    );
}

#[test]
fn isolate_without_export_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "ISOLATE", "directory": "server", "entry": "e.mjs" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("requires export: \"fetch\""),
        "{err}"
    );
}

#[test]
fn isolate_with_wrong_export_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "ISOLATE", "directory": "server",
                     "entry": "e.mjs", "export": "default" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("got: \"default\""), "{err}");
}

#[test]
fn compute_without_entry_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "standalone" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("(target=COMPUTE) requires 'entry'"),
        "{err}"
    );
}

#[test]
fn compute_with_export_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "standalone",
                     "entry": "server.js", "export": "fetch" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("COMPUTE layer 's' must not have 'export'"),
        "{err}"
    );
}

#[test]
fn entry_path_traversal_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "standalone",
                     "entry": "../outside/server.js" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

// ── Routes ────────────────────────────────────────────────────

#[test]
fn empty_routes() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": []
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("at least one route"), "{err}");
}

#[test]
fn route_pattern_without_anchor_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "/api/.*", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("must start with '^/'"), "{err}");
}

#[test]
fn route_unknown_layer_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "assets", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "ghost" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("route references unknown layer: 'ghost'"),
        "{err}"
    );
}

#[test]
fn revalidate_on_static_layer_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s", "revalidate": 60 }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("ISR revalidate not applicable to STATIC layer"),
        "{err}"
    );
}

#[test]
fn revalidate_zero_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "e.mjs", "export": "fetch" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "server", "revalidate": 0 }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("ISR revalidate must be positive"),
        "{err}"
    );
}

#[test]
fn revalidate_valid_parses() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "e.mjs", "export": "fetch" }
        ],
        "routes": [{ "pattern": "^/blog/.*$", "layer": "server", "revalidate": 60 }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.routes[0].revalidate, Some(60));
}

// ── JSON / file errors ────────────────────────────────────────

#[test]
fn invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), "not json");
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("failed to parse"), "{err}");
}

#[test]
fn missing_file() {
    let err = load_and_validate(Path::new("/nonexistent/manifest.json")).unwrap_err();
    assert!(err.to_string().contains("failed to read"), "{err}");
}

// ── Optional sections ─────────────────────────────────────────

#[test]
fn manifest_with_middleware_parses() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [
            {
                "name": "auth",
                "bundlePath": "middleware/auth.mjs",
                "codeHash": "sha256-abc",
                "matchers": ["^/dashboard/.*$"]
            }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    let mw = m.middleware.unwrap();
    assert_eq!(mw.len(), 1);
    assert_eq!(mw[0].name, "auth");
}

#[test]
fn manifest_with_prerender_parses() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "prerendered", "target": "STATIC", "directory": "prerender" },
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "entry.mjs", "export": "fetch" }
        ],
        "routes": [
            { "pattern": "^/about/?$", "layer": "prerendered", "priority": 50 },
            { "pattern": "^/.*$", "layer": "server", "priority": 0 }
        ],
        "prerender": {
            "layer": "prerendered",
            "pages": { "/about": { "html": "about/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    let pr = m.prerender.unwrap();
    assert_eq!(pr.layer, "prerendered");
    assert!(pr.pages.contains_key("/about"));
}

// ── verify_files ──────────────────────────────────────────────

#[test]
fn verify_files_ok_isolate() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), ASTRO_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("client")).unwrap();
    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/entry.mjs"), "").unwrap();

    verify_files(dir.path(), &m).unwrap();
}

#[test]
fn verify_files_ok_compute() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), NEXTJS_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("static")).unwrap();
    std::fs::create_dir_all(dir.path().join("standalone")).unwrap();
    std::fs::write(dir.path().join("standalone/server.js"), "").unwrap();

    verify_files(dir.path(), &m).unwrap();
}

#[test]
fn verify_files_missing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), ASTRO_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    // Only create one of the two required dirs
    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/entry.mjs"), "").unwrap();

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(
        err.to_string()
            .contains("layer directory not found: 'client'"),
        "{err}"
    );
}

#[test]
fn verify_files_missing_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), ASTRO_MANIFEST);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("client")).unwrap();
    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    // entry.mjs deliberately not created

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(
        err.to_string()
            .contains("entry not found: 'server/entry.mjs'"),
        "{err}"
    );
}

#[test]
fn validate_prerender_unknown_layer_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "prerender": { "layer": "ghost", "pages": {} }
    }"#;
    let path = write_manifest(dir.path(), json);
    // validate() now catches prerender→layer cross-reference
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("prerender references unknown layer: 'ghost'"),
        "{err}"
    );
}

#[test]
fn verify_files_prerender_pages_exist() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "prerendered", "target": "STATIC", "directory": "prerender" },
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "entry.mjs", "export": "fetch" }
        ],
        "routes": [
            { "pattern": "^/about/?$", "layer": "prerendered", "priority": 50 },
            { "pattern": "^/.*$", "layer": "server", "priority": 0 }
        ],
        "prerender": {
            "layer": "prerendered",
            "pages": { "/about": { "html": "about/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("prerender/about")).unwrap();
    std::fs::write(dir.path().join("prerender/about/index.html"), "").unwrap();
    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/entry.mjs"), "").unwrap();

    verify_files(dir.path(), &m).unwrap();
}

#[test]
fn verify_files_prerender_html_missing() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "prerendered", "target": "STATIC", "directory": "prerender" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "prerendered" }],
        "prerender": {
            "layer": "prerendered",
            "pages": { "/about": { "html": "about/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();

    // Create layer dir but not the html file
    std::fs::create_dir_all(dir.path().join("prerender")).unwrap();

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(
        err.to_string()
            .contains("prerender page '/about' html not found"),
        "{err}"
    );
}

// ── primary_compute_target ────────────────────────────────────

fn make_manifest_with_targets(targets: &[(&str, &str)]) -> crate::build::manifest::Manifest {
    use serde_json::json;
    let layers_json: Vec<serde_json::Value> = targets
        .iter()
        .map(|(name, target)| {
            let mut obj = json!({ "name": name, "target": target, "directory": "." });
            if *target == "ISOLATE" {
                obj["entry"] = json!("e.mjs");
                obj["export"] = json!("fetch");
            } else if *target == "COMPUTE" {
                obj["entry"] = json!("server.js");
            }
            obj
        })
        .collect();
    let manifest_json = json!({
        "version": 1,
        "layers": layers_json,
        "routes": [{ "pattern": "^/.*$", "layer": targets[0].0 }]
    });
    serde_json::from_value(manifest_json).unwrap()
}

#[test]
fn primary_compute_target_returns_compute_when_present() {
    let m = make_manifest_with_targets(&[("assets", "STATIC"), ("server", "COMPUTE")]);
    assert_eq!(primary_compute_target(&m), LayerTarget::Compute);
}

#[test]
fn primary_compute_target_returns_isolate_when_no_compute() {
    let m = make_manifest_with_targets(&[("assets", "STATIC"), ("server", "ISOLATE")]);
    assert_eq!(primary_compute_target(&m), LayerTarget::Isolate);
}

#[test]
fn primary_compute_target_returns_static_when_only_static() {
    let m = make_manifest_with_targets(&[("site", "STATIC")]);
    assert_eq!(primary_compute_target(&m), LayerTarget::Static);
}

#[test]
fn primary_compute_target_prefers_compute_over_isolate() {
    let m = make_manifest_with_targets(&[
        ("assets", "STATIC"),
        ("edge", "ISOLATE"),
        ("api", "COMPUTE"),
    ]);
    assert_eq!(primary_compute_target(&m), LayerTarget::Compute);
}

// ── Spec limits: layer count ───────────────────────────────────

#[test]
fn too_many_layers_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // Build 11 STATIC layers (MAX_LAYERS = 10)
    let layers: String = (0..=10)
        .map(|i| format!(r#"{{"name":"l{i}","target":"STATIC","directory":"d{i}"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"version":1,"layers":[{layers}],"routes":[{{"pattern":"^/.*$","layer":"l0"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("too many layers"), "{err}");
}

// ── Spec limits: route count ───────────────────────────────────

#[test]
fn too_many_routes_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // Build 201 routes (MAX_ROUTES = 200)
    let routes: String = (0..=200)
        .map(|i| format!(r#"{{"pattern":"^/p{i}$","layer":"s"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{routes}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("too many routes"), "{err}");
}

// ── Field length limits ────────────────────────────────────────

#[test]
fn layer_name_too_long_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let long_name = "a".repeat(65); // MAX_NAME_LEN = 64
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"{long_name}","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"{long_name}"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("layer name exceeds"), "{err}");
}

#[test]
fn layer_directory_too_long_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let long_dir = "a".repeat(257); // MAX_DIRECTORY_LEN = 256
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"{long_dir}"}}],"routes":[{{"pattern":"^/.*$","layer":"s"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("directory path exceeds"), "{err}");
}

#[test]
fn layer_entry_too_long_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let long_entry = "a".repeat(513); // MAX_ENTRY_LEN = 512
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"COMPUTE","directory":"dist","entry":"{long_entry}"}}],"routes":[{{"pattern":"^/.*$","layer":"s"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("entry path exceeds"), "{err}");
}

// ── Routes: invalid regex ──────────────────────────────────────

#[test]
fn route_invalid_regex_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // `[invalid` is an unclosed character class — invalid regex
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/[invalid$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("invalid regex in route pattern"),
        "{err}"
    );
}

// ── Routes: path %5c (encoded backslash) ──────────────────────

#[test]
fn directory_path_traversal_encoded_backslash_not_blocked() {
    let dir = tempfile::tempdir().unwrap();
    // %5c (encoded backslash) alone is not flagged — matches server behaviour.
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "foo%5cbar" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    load_and_validate(&path).unwrap();
}

// ── Middleware validation ──────────────────────────────────────

#[test]
fn too_many_middleware_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let mws: String = (0..=10)
        .map(|i| {
            format!(
                r#"{{"name":"mw{i}","bundlePath":"mw{i}.mjs","codeHash":"h","matchers":["^/.*$"]}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"s"}}],"middleware":[{mws}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("too many middleware"), "{err}");
}

#[test]
fn duplicate_middleware_names_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [
            { "name": "auth", "bundlePath": "a.mjs", "codeHash": "h", "matchers": ["^/.*$"] },
            { "name": "auth", "bundlePath": "b.mjs", "codeHash": "h", "matchers": ["^/.*$"] }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("duplicate middleware name: 'auth'"),
        "{err}"
    );
}

#[test]
fn middleware_bundle_path_traversal_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [
            { "name": "evil", "bundlePath": "../outside/evil.mjs", "codeHash": "h",
              "matchers": ["^/.*$"] }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn middleware_bundle_path_too_long_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let long_path = "a".repeat(513); // MAX_BUNDLE_PATH_LEN = 512
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"s"}}],"middleware":[{{"name":"m","bundlePath":"{long_path}","codeHash":"h","matchers":[]}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("bundlePath exceeds"), "{err}");
}

#[test]
fn middleware_invalid_matcher_regex_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [
            { "name": "bad", "bundlePath": "bad.mjs", "codeHash": "h",
              "matchers": ["^/[broken"] }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("invalid regex in middleware"),
        "{err}"
    );
}

// ── Prerender: layer must be STATIC ───────────────────────────

#[test]
fn prerender_layer_must_be_static() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "e.mjs", "export": "fetch" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "server" }],
        "prerender": {
            "layer": "server",
            "pages": { "/about": { "html": "about/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("prerender layer 'server' must be STATIC"),
        "{err}"
    );
}

// ── Prerender: path traversal in page html/data ────────────────

#[test]
fn prerender_html_traversal_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "prerender": {
            "layer": "s",
            "pages": { "/x": { "html": "../secret/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

#[test]
fn prerender_data_traversal_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "prerender": {
            "layer": "s",
            "pages": { "/x": { "html": "x/index.html", "data": "../../etc/passwd" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

// ── verify_files: prerender data file missing ──────────────────

#[test]
fn verify_files_prerender_data_missing() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "pre" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "prerender": {
            "layer": "s",
            "pages": { "/blog": { "html": "blog/index.html", "data": "blog/data.json" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();

    std::fs::create_dir_all(dir.path().join("pre/blog")).unwrap();
    std::fs::write(dir.path().join("pre/blog/index.html"), "").unwrap();
    // data.json deliberately missing

    let err = verify_files(dir.path(), &m).unwrap_err();
    assert!(
        err.to_string()
            .contains("prerender page '/blog' data not found"),
        "{err}"
    );
}

// ── Runtime config validation ──────────────────────────────────

#[test]
fn runtime_timeout_ms_zero_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist",
                     "entry": "server.js", "runtime": { "timeoutMs": 0 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("runtime.timeoutMs must be positive"),
        "{err}"
    );
}

#[test]
fn runtime_memory_mb_zero_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist",
                     "entry": "server.js", "runtime": { "memoryMb": 0 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("runtime.memoryMb must be positive"),
        "{err}"
    );
}

#[test]
fn runtime_max_concurrency_zero_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "ISOLATE", "directory": "server",
                     "entry": "e.mjs", "export": "fetch",
                     "runtime": { "maxConcurrency": 0 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("runtime.maxConcurrency must be positive"),
        "{err}"
    );
}

#[test]
fn runtime_max_concurrency_on_compute_is_allowed() {
    // Server schema permits maxConcurrency on any layer target — CLI matches.
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist",
                     "entry": "server.js", "runtime": { "maxConcurrency": 4 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    load_and_validate(&path).unwrap();
}

#[test]
fn runtime_valid_values_parse() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "ISOLATE", "directory": "server",
                     "entry": "e.mjs", "export": "fetch",
                     "runtime": { "timeoutMs": 5000, "memoryMb": 256, "maxConcurrency": 10 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    let rt = m.layers[0].runtime.as_ref().unwrap();
    assert_eq!(rt.timeout_ms, Some(5000));
    assert_eq!(rt.memory_mb, Some(256));
    assert_eq!(rt.max_concurrency, Some(10));
}

// ── URL-encoded dot (%2e) ──────────────────────────────────────

#[test]
fn directory_path_traversal_single_encoded_dot() {
    let dir = tempfile::tempdir().unwrap();
    // Single %2e (encoded dot) is now flagged — matches server URL_ENCODED_TRAVERSAL_REGEX.
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "x", "target": "STATIC", "directory": "foo%2ebar" }],
        "routes": [{ "pattern": "^/.*$", "layer": "x" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("path traversal"), "{err}");
}

// ── Duplicate route patterns ───────────────────────────────────

#[test]
fn duplicate_route_patterns_same_priority_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [
            { "pattern": "^/.*$", "layer": "s" },
            { "pattern": "^/.*$", "layer": "s" }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("duplicate route pattern with same priority"),
        "{msg}"
    );
    assert!(
        msg.contains("implicit default"),
        "error should mention implicit default priority: {msg}"
    );
}

#[test]
fn duplicate_route_patterns_different_priority_is_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "pub", "target": "STATIC", "directory": "public" },
            { "name": "srv", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
        ],
        "routes": [
            { "pattern": "^/.*$", "layer": "pub", "priority": 50 },
            { "pattern": "^/.*$", "layer": "srv", "priority": 0 }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    load_and_validate(&path).unwrap();
}

#[test]
fn duplicate_pattern_same_layer_different_priority_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "srv", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
        ],
        "routes": [
            { "pattern": "^/.*$", "layer": "srv", "priority": 50 },
            { "pattern": "^/.*$", "layer": "srv", "priority": 0 }
        ]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("unreachable"),
        "expected unreachable route error, got: {err}"
    );
}

// ── Meta size limit ────────────────────────────────────────────

#[test]
fn meta_too_large_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // Build a meta value that serialises to > 16 384 bytes
    let big_value = "x".repeat(20_000);
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"s"}}],"meta":{{"big":"{big_value}"}}}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("meta exceeds"), "{err}");
}

#[test]
fn meta_within_limit_parses() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "meta": { "framework": "astro", "version": "5.0.0" }
    }"#;
    let path = write_manifest(dir.path(), json);
    load_and_validate(&path).unwrap();
}

// ── Empty string checks ────────────────────────────────────────

#[test]
fn empty_layer_name_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("layer name must not be empty"),
        "{err}"
    );
}

#[test]
fn empty_layer_directory_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("directory must not be empty"),
        "{err}"
    );
}

#[test]
fn empty_layer_entry_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist", "entry": "" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("entry must not be empty"), "{err}");
}

#[test]
fn empty_middleware_name_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [{ "name": "", "bundlePath": "mw.mjs", "codeHash": "h", "matchers": [] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("middleware name must not be empty"),
        "{err}"
    );
}

#[test]
fn middleware_name_too_long_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let long_name = "m".repeat(65); // MAX_NAME_LEN = 64
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"s"}}],"middleware":[{{"name":"{long_name}","bundlePath":"mw.mjs","codeHash":"h","matchers":[]}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("middleware name exceeds"), "{err}");
}

#[test]
fn empty_middleware_code_hash_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [{ "name": "m", "bundlePath": "mw.mjs", "codeHash": "", "matchers": [] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("codeHash must not be empty"),
        "{err}"
    );
}

#[test]
fn empty_middleware_bundle_path_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [{ "name": "m", "bundlePath": "", "codeHash": "h", "matchers": [] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("bundlePath must not be empty"),
        "{err}"
    );
}

#[test]
fn empty_middleware_matcher_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [{ "name": "m", "bundlePath": "mw.mjs", "codeHash": "h",
                         "matchers": ["^/api/.*$", ""] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("matcher must not be empty"),
        "{err}"
    );
}

// ── Prerender page key must start with '/' ─────────────────────

#[test]
fn prerender_page_key_without_leading_slash_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "prerender": {
            "layer": "s",
            "pages": { "about": { "html": "about/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("page key must start with '/'"),
        "{err}"
    );
}

#[test]
fn prerender_page_key_with_leading_slash_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "prerender": {
            "layer": "s",
            "pages": { "/about": { "html": "about/index.html" } }
        }
    }"#;
    let path = write_manifest(dir.path(), json);
    load_and_validate(&path).unwrap();
}

// ── Route pattern length limit ─────────────────────────────────

#[test]
fn route_pattern_exceeds_500_chars_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // "^/" + 499 'a' chars = 501 bytes total
    let long_pattern = format!("^/{}", "a".repeat(499));
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"{long_pattern}","layer":"s"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("route pattern exceeds 500 chars"),
        "{err}"
    );
}

#[test]
fn route_pattern_exactly_500_chars_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    // "^/" + 498 'a' chars = 500 bytes — exactly at the limit
    let ok_pattern = format!("^/{}", "a".repeat(498));
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"s","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"{ok_pattern}","layer":"s"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    load_and_validate(&path).unwrap();
}

// ── #1: STATIC layer must not have `runtime` ──────────────────

#[test]
fn static_with_runtime_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist",
                     "runtime": { "timeoutMs": 5000 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("STATIC layer 's' must not have 'runtime'"),
        "{err}"
    );
}

// ── #2: revalidate max 31_536_000 ─────────────────────────────

#[test]
fn revalidate_exceeds_max_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "e.mjs", "export": "fetch" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "server", "revalidate": 31536001 }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("ISR revalidate exceeds maximum"),
        "{err}"
    );
}

#[test]
fn revalidate_exactly_max_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "e.mjs", "export": "fetch" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "server", "revalidate": 31536000 }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.routes[0].revalidate, Some(31_536_000));
}

// ── #3: routes[].methods enum validation ──────────────────────

#[test]
fn route_lowercase_method_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s", "methods": ["get"] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("invalid HTTP method 'get'"),
        "{err}"
    );
}

#[test]
fn route_unknown_method_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s", "methods": ["INVALID"] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("invalid HTTP method 'INVALID'"),
        "{err}"
    );
}

#[test]
fn route_valid_methods_parse() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s",
                     "methods": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.routes[0].methods.as_deref().unwrap().len(), 7);
}

// ── #6: middleware must have at least one matcher ──────────────

#[test]
fn middleware_empty_matchers_array_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [{ "name": "m", "bundlePath": "mw.mjs", "codeHash": "h", "matchers": [] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("must have at least one matcher"),
        "{err}"
    );
}

// ── #7: JS-only regex features rejected ───────────────────────

#[test]
fn route_pattern_lookahead_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/(?=login).*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("JS-only regex features"), "{err}");
}

#[test]
fn route_pattern_lookbehind_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/(?<=api/).*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("JS-only regex features"), "{err}");
}

#[test]
fn route_pattern_backreference_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/(\\w+)/\\1$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("JS-only regex features"), "{err}");
}

#[test]
fn middleware_matcher_lookahead_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "." }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }],
        "middleware": [{ "name": "m", "bundlePath": "mw.mjs", "codeHash": "h",
                         "matchers": ["^/(?!public/).*$"] }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("JS-only regex features"), "{err}");
}

// ── #9: length limits count Unicode chars, not bytes ──────────

#[test]
fn layer_name_unicode_within_char_limit_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    // 64 Cyrillic chars — each is 2 UTF-8 bytes (128 bytes total),
    // but only 64 Unicode chars → within the 64-char limit.
    let name = "а".repeat(64);
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"{name}","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"{name}"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    load_and_validate(&path).unwrap();
}

#[test]
fn layer_name_unicode_exceeds_char_limit_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // 65 Cyrillic chars — exceeds the 64-char limit.
    let name = "а".repeat(65);
    let json = format!(
        r#"{{"version":1,"layers":[{{"name":"{name}","target":"STATIC","directory":"."}}],"routes":[{{"pattern":"^/.*$","layer":"{name}"}}]}}"#
    );
    let path = write_manifest(dir.path(), &json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(err.to_string().contains("layer name exceeds"), "{err}");
}

// ── #10: fractional runtime values give a clear error ─────────

#[test]
fn runtime_fractional_timeout_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist",
                     "entry": "server.js", "runtime": { "timeoutMs": 5000.5 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("non-negative integer")
            || err.to_string().contains("failed to parse"),
        "{err}"
    );
}

#[test]
fn runtime_negative_memory_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist",
                     "entry": "server.js", "runtime": { "memoryMb": -256 } }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string().contains("non-negative integer")
            || err.to_string().contains("failed to parse"),
        "{err}"
    );
}

// ── Auto-generation ────────────────────────────────────────────

#[test]
fn generate_static_manifest_is_valid() {
    let m = generate_static_manifest();
    validate(&m).unwrap();
    assert_eq!(m.layers.len(), 1);
    assert_eq!(m.layers[0].target, LayerTarget::Static);
    assert_eq!(m.layers[0].directory, ".");
    assert!(m.layers[0].entry.is_none());
    assert_eq!(m.routes.len(), 1);
    assert_eq!(m.routes[0].pattern, "^/.*$");
    assert_eq!(m.routes[0].layer, "site");
}

#[test]
fn generate_compute_manifest_is_valid() {
    let m = generate_compute_manifest("server.js");
    validate(&m).unwrap();
    assert_eq!(m.layers.len(), 1);
    assert_eq!(m.layers[0].target, LayerTarget::Compute);
    assert_eq!(m.layers[0].entry.as_deref(), Some("server.js"));
    assert_eq!(m.layers[0].directory, ".");
    assert_eq!(m.routes.len(), 1);
    assert_eq!(m.routes[0].pattern, "^/.*$");
    assert_eq!(m.routes[0].layer, "server");
}

#[test]
fn generate_compute_manifest_empty_entry_fails_validation() {
    // Auto-gen with empty entry string must fail validate() — callers must pass a non-empty entry.
    let m = generate_compute_manifest("");
    let err = validate(&m).expect_err("empty entry should fail validation");
    assert!(
        err.to_string().contains("entry must not be empty"),
        "unexpected error: {err}"
    );
}

// ── isPrecompressed ────────────────────────────────────────────

#[test]
fn static_layer_is_precompressed_true_parses() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist",
                     "isPrecompressed": true }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.layers[0].is_precompressed, Some(true));
}

#[test]
fn static_layer_is_precompressed_false_parses() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist",
                     "isPrecompressed": false }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    assert_eq!(m.layers[0].is_precompressed, Some(false));
}

#[test]
fn static_layer_is_precompressed_absent_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_manifest(dir.path(), STATIC_MANIFEST);
    let m = load_and_validate(&path).unwrap();
    assert!(m.layers[0].is_precompressed.is_none());
}

#[test]
fn isolate_layer_is_precompressed_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "ISOLATE", "directory": "server",
                     "entry": "e.mjs", "export": "fetch", "isPrecompressed": true }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("ISOLATE layer 's' must not have 'isPrecompressed'"),
        "{err}"
    );
}

#[test]
fn compute_layer_is_precompressed_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "COMPUTE", "directory": "dist",
                     "entry": "server.js", "isPrecompressed": true }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let err = load_and_validate(&path).unwrap_err();
    assert!(
        err.to_string()
            .contains("COMPUTE layer 's' must not have 'isPrecompressed'"),
        "{err}"
    );
}

#[test]
fn is_precompressed_serializes_as_camel_case() {
    let dir = tempfile::tempdir().unwrap();
    let json = r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist",
                     "isPrecompressed": true }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#;
    let path = write_manifest(dir.path(), json);
    let m = load_and_validate(&path).unwrap();
    let serialized = serde_json::to_value(&m).unwrap();
    let layer = &serialized["layers"][0];
    assert_eq!(layer["isPrecompressed"], serde_json::json!(true));
    assert!(layer.get("is_precompressed").is_none());
}

#[test]
fn is_precompressed_none_omitted_from_serialization() {
    let m = generate_static_manifest();
    let serialized = serde_json::to_value(&m).unwrap();
    let layer = &serialized["layers"][0];
    assert!(
        layer.get("isPrecompressed").is_none(),
        "isPrecompressed should be omitted when None"
    );
}

// ── Next.js standalone manifest generation ─────────────────────

#[test]
fn nextjs_standalone_manifest_with_public_is_valid() {
    let m = generate_nextjs_standalone_manifest(true);
    validate(&m).unwrap();
    assert_eq!(m.layers.len(), 3);
    assert_eq!(m.routes.len(), 3);
    assert_eq!(m.layers[0].name, "static-assets");
    assert_eq!(m.layers[0].target, LayerTarget::Static);
    assert_eq!(m.layers[1].name, "public-assets");
    assert_eq!(m.layers[1].target, LayerTarget::Static);
    assert_eq!(m.layers[2].name, "server");
    assert_eq!(m.layers[2].target, LayerTarget::Compute);
}

#[test]
fn nextjs_standalone_manifest_without_public_is_valid() {
    let m = generate_nextjs_standalone_manifest(false);
    validate(&m).unwrap();
    assert_eq!(m.layers.len(), 2);
    assert_eq!(m.routes.len(), 2);
    assert_eq!(m.layers[0].name, "static-assets");
    assert_eq!(m.layers[1].name, "server");
}

#[test]
fn nextjs_standalone_manifest_directories() {
    let m = generate_nextjs_standalone_manifest(true);
    assert_eq!(m.layers[0].directory, "_static");
    assert_eq!(m.layers[1].directory, "public");
    assert_eq!(m.layers[2].directory, ".");
    assert_eq!(m.layers[2].entry.as_deref(), Some("server.js"));
    // Generated layers must not set is_precompressed (deploy handles compression separately)
    assert!(m.layers.iter().all(|l| l.is_precompressed.is_none()));
}

#[test]
fn nextjs_standalone_manifest_route_priorities() {
    let m = generate_nextjs_standalone_manifest(true);
    assert_eq!(m.routes[0].priority, Some(100));
    assert_eq!(m.routes[0].pattern, "^/_next/static/.*$");
    assert_eq!(m.routes[1].priority, Some(50));
    assert_eq!(m.routes[1].pattern, "^/.*$");
    assert_eq!(m.routes[2].priority, Some(0));
    assert_eq!(m.routes[2].pattern, "^/.*$");
}
