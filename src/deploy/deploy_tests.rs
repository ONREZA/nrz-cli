use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn scan_files_flat_directory() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::write(dir.path().join("style.css"), "body{}").unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "index.html");
    assert_eq!(files[1].path, "style.css");
}

#[test]
fn scan_files_nested_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets/images")).unwrap();
    fs::write(dir.path().join("index.html"), "hi").unwrap();
    fs::write(dir.path().join("assets/app.js"), "js").unwrap();
    fs::write(dir.path().join("assets/images/logo.png"), "png").unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(files.len(), 3);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"index.html"));
    assert!(paths.contains(&"assets/app.js"));
    assert!(paths.contains(&"assets/images/logo.png"));
}

#[test]
fn scan_files_records_correct_sizes() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn scan_files_computes_sha256_from_original_content() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(files.len(), 1);
    let hash = files[0]
        .sha256
        .as_deref()
        .expect("sha256 should be present");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Known SHA-256 of "hello world"
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn scan_files_sha256_deterministic_across_calls() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "same content").unwrap();

    let (files1, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();
    let (files2, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(files1[0].sha256, files2[0].sha256);
}

#[test]
fn scan_files_sha256_differs_for_different_content() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "content A").unwrap();
    fs::write(dir.path().join("b.txt"), "content B").unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_ne!(files[0].sha256, files[1].sha256);
}

#[test]
fn scan_files_empty_directory() {
    let dir = tempdir().unwrap();
    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();
    assert!(files.is_empty());
}

#[test]
fn scan_files_sorted_alphabetically() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "m.txt");
    assert_eq!(files[2].path, "z.txt");
}

#[test]
fn scan_files_allows_double_dots_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file..backup.js"), "x").unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "file..backup.js");
}

#[cfg(unix)]
#[test]
fn scan_files_skips_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let (files, _) = scan_and_maybe_compress(dir.path(), &[]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "real.txt");
}

// ── synthetic_sha tests ─────────────────────────────────────

#[test]
fn synthetic_sha_deterministic() {
    let files = vec![
        FileEntry {
            path: "a.js".into(),
            size: 100,
            sha256: None,
        },
        FileEntry {
            path: "b.css".into(),
            size: 200,
            sha256: None,
        },
    ];

    let sha1 = synthetic_sha(&files);
    let sha2 = synthetic_sha(&files);
    assert_eq!(sha1, sha2);
}

#[test]
fn synthetic_sha_differs_for_different_files() {
    let files_a = vec![FileEntry {
        path: "a.js".into(),
        size: 100,
        sha256: None,
    }];
    let files_b = vec![FileEntry {
        path: "b.js".into(),
        size: 100,
        sha256: None,
    }];

    assert_ne!(synthetic_sha(&files_a), synthetic_sha(&files_b));
}

#[test]
fn synthetic_sha_is_64_hex_chars() {
    let files = vec![FileEntry {
        path: "x.txt".into(),
        size: 1,
        sha256: None,
    }];
    let sha = synthetic_sha(&files);
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── resolve_build_command tests ──────────────────────────────

#[test]
fn build_command_explicit_wins_over_config_and_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(Some("explicit cmd"), dir.path(), &config, None);
    assert_eq!(result.unwrap(), "explicit cmd");
}

#[test]
fn build_command_config_wins_over_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(None, dir.path(), &config, None);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_auto_detect_bun_lock() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, None);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_bun_lockb() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lockb"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, None);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_pnpm() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, None);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_auto_detect_yarn() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, None);
    assert_eq!(result.unwrap(), "yarn run build");
}

#[test]
fn build_command_auto_detect_npm_fallback() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, None);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_none_without_package_json() {
    let dir = tempdir().unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, None);
    assert!(result.is_none());
}

// ── resolve_build_command server fallback ────────────────────

#[test]
fn build_command_server_wins_over_auto_detect() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, Some("server build cmd"));
    assert_eq!(result.unwrap(), "server build cmd");
}

#[test]
fn build_command_config_wins_over_server() {
    let dir = tempdir().unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(None, dir.path(), &config, Some("server cmd"));
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_explicit_wins_over_server() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();

    let result = resolve_build_command(Some("explicit"), dir.path(), &config, Some("server cmd"));
    assert_eq!(result.unwrap(), "explicit");
}

#[test]
fn build_command_server_used_without_package_json() {
    let dir = tempdir().unwrap();
    // No package.json — auto-detect would return None, but server command should still work
    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config, Some("make build"));
    assert_eq!(result.unwrap(), "make build");
}

// ── ProjectInfo deserialization ──────────────────────────────

#[test]
fn project_info_deserializes_camel_case() {
    let json = r#"{
        "id": "proj_123",
        "installCommand": "npm ci",
        "buildCommand": "npm run build",
        "outputDirectory": "dist"
    }"#;
    let info: ProjectInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.install_command.unwrap(), "npm ci");
    assert_eq!(info.build_command.unwrap(), "npm run build");
    assert_eq!(info.output_directory.unwrap(), "dist");
}

#[test]
fn project_info_optional_fields_default_to_none() {
    let json = r#"{"id": "proj_123"}"#;
    let info: ProjectInfo = serde_json::from_str(json).unwrap();
    assert!(info.install_command.is_none());
    assert!(info.build_command.is_none());
    assert!(info.output_directory.is_none());
}

// ── guess_content_type tests ────────────────────────────────

#[test]
fn content_type_html() {
    assert_eq!(guess_content_type("index.html"), "text/html");
    assert_eq!(guess_content_type("page.htm"), "text/html");
}

#[test]
fn content_type_js() {
    assert_eq!(guess_content_type("app.js"), "application/javascript");
    assert_eq!(guess_content_type("entry.mjs"), "application/javascript");
    assert_eq!(guess_content_type("lib.cjs"), "application/javascript");
}

#[test]
fn content_type_css() {
    assert_eq!(guess_content_type("style.css"), "text/css");
}

#[test]
fn content_type_images() {
    assert_eq!(guess_content_type("logo.png"), "image/png");
    assert_eq!(guess_content_type("photo.jpg"), "image/jpeg");
    assert_eq!(guess_content_type("photo.jpeg"), "image/jpeg");
    assert_eq!(guess_content_type("hero.webp"), "image/webp");
    assert_eq!(guess_content_type("icon.svg"), "image/svg+xml");
    assert_eq!(guess_content_type("icon.ico"), "image/x-icon");
}

#[test]
fn content_type_fonts() {
    assert_eq!(guess_content_type("font.woff2"), "font/woff2");
    assert_eq!(guess_content_type("font.woff"), "font/woff");
    assert_eq!(guess_content_type("font.ttf"), "font/ttf");
}

#[test]
fn content_type_data() {
    assert_eq!(guess_content_type("data.json"), "application/json");
    assert_eq!(guess_content_type("app.d4e5f6.js.map"), "application/json");
    assert_eq!(guess_content_type("app.wasm"), "application/wasm");
}

#[test]
fn content_type_nested_path() {
    assert_eq!(
        guess_content_type("_astro/app.d4e5f6.js"),
        "application/javascript"
    );
    assert_eq!(
        guess_content_type("server/entry.mjs"),
        "application/javascript"
    );
}

#[test]
fn content_type_unknown_fallback() {
    assert_eq!(guess_content_type("file.xyz"), "application/octet-stream");
    assert_eq!(guess_content_type("noext"), "application/octet-stream");
}

// ── framework_static_hint tests ──────────────────────────────

#[test]
fn static_hint_known_frameworks_non_empty() {
    assert!(!framework_static_hint("nextjs").is_empty());
    assert!(!framework_static_hint("nuxt").is_empty());
    assert!(!framework_static_hint("sveltekit").is_empty());
    assert!(!framework_static_hint("astro").is_empty());
    assert!(!framework_static_hint("react-router").is_empty());
    assert!(!framework_static_hint("remix").is_empty());
    assert!(!framework_static_hint("solidstart").is_empty());
    assert!(!framework_static_hint("qwik").is_empty());
    assert!(!framework_static_hint("analog").is_empty());
    assert!(framework_static_hint("nextjs").contains("export"));
    assert!(framework_static_hint("react-router").contains("ssr: false"));
    assert!(framework_static_hint("remix").contains("ssr: false"));
    assert!(framework_static_hint("solidstart").contains("ssr: false"));
    assert!(framework_static_hint("analog").contains("ssr: false"));
}

#[test]
fn static_hint_unknown_returns_empty() {
    assert!(framework_static_hint("vite").is_empty());
    assert!(framework_static_hint("unknown").is_empty());
}

// ── compute/manifest contract tests ─────────────────────────

#[test]
fn isolate_without_manifest_is_error() {
    let detection = make_detection("nextjs", None);
    let err = validate_compute_manifest_contract(ComputeType::Isolate, false, &detection)
        .expect_err("ISOLATE without manifest should fail");
    assert!(err.to_string().contains("ISOLATE"));
}

#[test]
fn process_with_manifest_is_ok() {
    // Manifest can declare COMPUTE layers — PROCESS + manifest is valid.
    let detection = make_detection("nextjs", None);
    assert!(validate_compute_manifest_contract(ComputeType::Process, true, &detection).is_ok());
}

#[test]
fn isolate_with_manifest_is_ok() {
    let detection = make_detection("nextjs", None);
    assert!(validate_compute_manifest_contract(ComputeType::Isolate, true, &detection).is_ok());
}

#[test]
fn static_without_manifest_is_ok() {
    let detection = make_detection("vite", None);
    assert!(validate_compute_manifest_contract(ComputeType::Static, false, &detection).is_ok());
}

#[test]
fn static_with_manifest_is_ok() {
    // Manifest can declare only STATIC layers — STATIC + manifest is valid.
    let detection = make_detection("vite", None);
    assert!(validate_compute_manifest_contract(ComputeType::Static, true, &detection).is_ok());
}

#[test]
fn process_without_manifest_is_error() {
    // Safety net: PROCESS auto-generation should always produce a manifest before
    // validate_compute_manifest_contract is called, so reaching here with has_manifest=false
    // is an unexpected state.
    let detection = make_detection("nextjs", None);
    let err = validate_compute_manifest_contract(ComputeType::Process, false, &detection)
        .expect_err("PROCESS without manifest should fail");
    assert!(
        err.to_string().contains("Internal error"),
        "unexpected error: {err}"
    );
}

#[test]
fn create_deployment_body_with_manifest_serializes_correctly() {
    let body = CreateDeploymentBody {
        manifest: Some(serde_json::json!({ "version": 1 })),
        files: vec![],
        production: false,
        branch: None,
        commit_sha: None,

        bundle_sha256: None,
    };

    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("manifest").is_some());
    assert!(value.get("computeType").is_none());
    assert!(value.get("processEntry").is_none());
}

#[test]
fn create_deployment_body_without_manifest_omits_manifest_field() {
    let body = CreateDeploymentBody {
        manifest: None,
        files: vec![],
        production: false,
        branch: None,
        commit_sha: None,

        bundle_sha256: None,
    };

    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("manifest").is_none());
    assert!(value.get("computeType").is_none());
    assert!(value.get("processEntry").is_none());
}

#[test]
fn prepare_upload_body_serializes_without_deprecated_fields() {
    let body = PrepareUploadBody {
        manifest: None,
        files: vec![],
        bundle_sha256: Some("abc123".to_string()),
    };

    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("manifest").is_none());
    assert!(value.get("computeType").is_none());
    assert!(value.get("processEntry").is_none());
    assert_eq!(
        value.get("bundleSha256").and_then(|v| v.as_str()),
        Some("abc123")
    );
}

#[test]
fn file_entry_serializes_sha256_when_present() {
    let entry = FileEntry {
        path: "a.js".into(),
        size: 42,
        sha256: Some("abc123".into()),
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["sha256"], "abc123");
    assert_eq!(json["path"], "a.js");
    assert_eq!(json["size"], 42);
}

#[test]
fn file_entry_omits_sha256_when_none() {
    let entry = FileEntry {
        path: "b.js".into(),
        size: 10,
        sha256: None,
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert!(json.get("sha256").is_none());
}

// ── manifest → compute type mapping tests ────────────────────
//
// Verifies the contract: primary_compute_target(manifest) → LayerTarget,
// which deploy maps as: Compute→Process, Isolate→Isolate, Static→Static.

#[test]
fn manifest_compute_layer_maps_to_process() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            {"name": "assets", "target": "STATIC", "directory": "static"},
            {"name": "server", "target": "COMPUTE", "directory": "standalone", "entry": "server.js"}
        ],
        "routes": [{"pattern": "^/.*$", "layer": "server"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Isolate => ComputeType::Isolate,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Process);
}

#[test]
fn manifest_isolate_layer_maps_to_isolate() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            {"name": "assets", "target": "STATIC", "directory": "client"},
            {"name": "server", "target": "ISOLATE", "directory": "server",
             "entry": "entry.mjs", "export": "fetch"}
        ],
        "routes": [{"pattern": "^/.*$", "layer": "server"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Isolate => ComputeType::Isolate,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Isolate);
}

#[test]
fn manifest_static_only_maps_to_static() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{"name": "site", "target": "STATIC", "directory": "."}],
        "routes": [{"pattern": "^/.*$", "layer": "site"}]
    }"#,
    )
    .unwrap();

    let target = crate::build::manifest::primary_compute_target(&manifest);
    let compute = match target {
        crate::build::manifest::LayerTarget::Compute => ComputeType::Process,
        crate::build::manifest::LayerTarget::Isolate => ComputeType::Isolate,
        crate::build::manifest::LayerTarget::Static => ComputeType::Static,
    };
    assert_eq!(compute, ComputeType::Static);
}

// ── validate_process_output tests ────────────────────────────

fn make_detection(
    framework: &str,
    ssr: Option<crate::detect::types::SsrAnalysis>,
) -> crate::detect::types::DetectionResult {
    crate::detect::types::DetectionResult {
        framework: framework.to_string(),
        name: framework.to_string(),
        version: None,
        suggested_compute: crate::detect::types::ComputeType::Process,
        reason: String::new(),
        metadata: crate::detect::types::DetectionMetadata {
            uses_typescript: None,
            config_files: vec![],
            runtime: crate::detect::types::RuntimeInfo {
                runtime_type: crate::detect::types::RuntimeType::Node,
                version: None,
            },
            package_manager: None,
            build_info: None,
            monorepo: None,
            ssr_analysis: ssr,

            structure: vec![],
        },
    }
}

#[test]
fn validate_nextjs_dot_next_without_standalone_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("output: 'standalone'"),
        "should mention standalone: {msg}"
    );
}

#[test]
fn validate_nextjs_dot_next_with_standalone_but_missing_server_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Missing file"),
        "should mention missing file: {msg}"
    );
}

#[test]
fn validate_nextjs_standalone_dir_without_server_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("server.js is missing"),
        "should mention missing server.js: {msg}"
    );
}

#[test]
fn validate_nextjs_standalone_dir_with_server_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(output_dir.join("server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_nuxt_without_server_entry_bails() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("nuxt", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("nuxi build"),
        "should mention nuxi build: {msg}"
    );
}

#[test]
fn validate_nuxt_with_server_entry_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".output");
    fs::create_dir_all(output_dir.join("server")).unwrap();
    fs::write(output_dir.join("server/index.mjs"), "export default {}").unwrap();

    let detection = make_detection("nuxt", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_unknown_framework_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join("dist");
    fs::create_dir(&output_dir).unwrap();

    let detection = make_detection("vite", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

// ── ensure_process_entry tests ───────────────────────────────

#[test]
fn ensure_process_entry_resolves_module_field() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"app","module":"./server.mjs"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("server.mjs"), "export default {}").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert_eq!(entry, Some("server.mjs".to_string()));
    assert!(warning.is_none());
}

#[test]
fn ensure_process_entry_ambiguous_candidates_falls_back_for_non_strict_framework() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("runtime")).unwrap();
    fs::write(dir.path().join("runtime/foo.mjs"), "console.log('foo')").unwrap();
    fs::write(dir.path().join("runtime/bar.mjs"), "console.log('bar')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert!(entry.is_none());
    let warning = warning.expect("expected fallback warning");
    assert!(warning.contains("ambiguous"));
    assert!(warning.contains("Falling back to runtime default"));
}

#[test]
fn ensure_process_entry_root_prefers_server_over_main() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('server')").unwrap();
    fs::write(dir.path().join("main.js"), "console.log('main')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert_eq!(entry, Some("server.js".to_string()));
    assert!(warning.is_none());
}

#[test]
fn ensure_process_entry_config_entry_allows_double_dot_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo..js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), Some("foo..js"), &detection, true).unwrap();
    assert_eq!(entry, Some("foo..js".to_string()));
    assert!(warning.is_none());
    assert!(!dir.path().join("package.json").exists());
}

#[test]
fn ensure_process_entry_config_entry_rejects_parent_traversal() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    let err = ensure_process_entry(
        dir.path(),
        dir.path(),
        Some("../server.js"),
        &detection,
        true,
    )
    .expect_err("parent traversal should fail");
    assert!(
        err.to_string()
            .contains("relative path within the output directory")
    );
}

#[test]
fn ensure_process_entry_not_found_falls_back_for_non_strict_framework() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.css"), "body{}").unwrap();

    let detection = make_detection("other", None);
    let (entry, warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert!(entry.is_none());
    let warning = warning.expect("expected fallback warning");
    assert!(warning.contains("did not find a runnable file"));
    assert!(warning.contains("bun"));
}

#[test]
fn ensure_process_entry_not_found_is_error_for_strict_framework() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let err =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).expect_err("error");
    assert!(
        err.to_string().contains("Nuxt PROCESS deployment expects"),
        "unexpected error: {err:#}"
    );
}

// ── COMPUTE auto-gen bail: entry not found ────────────────────

#[test]
fn ensure_process_entry_none_is_the_bail_precondition() {
    // Verify that ensure_process_entry returns None for a project with no runnable files —
    // this is the exact state that triggers "Cannot auto-generate COMPUTE manifest" in
    // the deploy run() flow when is_process && !has_manifest.
    let dir = tempdir().unwrap();
    // Create a "dist" output dir with no .js/.mjs/.cjs files
    fs::create_dir(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/style.css"), "body{}").unwrap();

    let detection = make_detection("other", None);
    let (entry, _warning) =
        ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();
    assert!(
        entry.is_none(),
        "entry should be None when no runnable file exists (triggering COMPUTE bail)"
    );
}

// ── framework_process_diagnostic tests ───────────────────────

#[test]
fn diagnostic_nextjs_no_standalone_suggests_config() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nextjs", None);
    let msg = framework_process_diagnostic("nextjs", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("output: 'standalone'"));
}

#[test]
fn diagnostic_nextjs_standalone_mentions_server_js() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let msg = framework_process_diagnostic("nextjs", &detection, &output_dir);
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server.js"));
}

#[test]
fn diagnostic_nuxt_mentions_nuxi_build() {
    let dir = tempdir().unwrap();
    let detection = make_detection("nuxt", None);
    let msg = framework_process_diagnostic("nuxt", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("nuxi build"));
}

#[test]
fn diagnostic_sveltekit_mentions_adapter_node() {
    let dir = tempdir().unwrap();
    let detection = make_detection("sveltekit", None);
    let msg = framework_process_diagnostic("sveltekit", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("adapter-node"));
}

#[test]
fn diagnostic_unknown_framework_returns_none() {
    let dir = tempdir().unwrap();
    let detection = make_detection("vite", None);
    let msg = framework_process_diagnostic("vite", &detection, dir.path());
    assert!(msg.is_none());
}

#[test]
fn diagnostic_react_router_mentions_server_index() {
    let dir = tempdir().unwrap();
    let detection = make_detection("react-router", None);
    let msg = framework_process_diagnostic("react-router", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server/index.js"));
}

#[test]
fn diagnostic_remix_mentions_server_index() {
    let dir = tempdir().unwrap();
    let detection = make_detection("remix", None);
    let msg = framework_process_diagnostic("remix", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("server/index.js"));
}

#[test]
fn diagnostic_hono_mentions_entry_point() {
    let dir = tempdir().unwrap();
    let detection = make_detection("hono", None);
    let msg = framework_process_diagnostic("hono", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("dist/"));
}

#[test]
fn diagnostic_elysia_mentions_bun() {
    let dir = tempdir().unwrap();
    let detection = make_detection("elysia", None);
    let msg = framework_process_diagnostic("elysia", &detection, dir.path());
    assert!(msg.is_some());
    assert!(msg.as_ref().unwrap().contains("Bun"));
}

// ── resolve_bundle_upload tests ──────────────────────────────

#[test]
fn resolve_bundle_upload_with_url() {
    let data = Some((vec![1, 2, 3], "abc123".to_string()));
    let result = resolve_bundle_upload(data, Some("https://s3.example.com/bundle")).unwrap();
    assert!(result.is_some());
    let (bytes, url) = result.unwrap();
    assert_eq!(bytes, vec![1, 2, 3]);
    assert_eq!(url, "https://s3.example.com/bundle");
}

#[test]
fn resolve_bundle_upload_no_bundle_data() {
    let result = resolve_bundle_upload(None, Some("https://s3.example.com/bundle")).unwrap();
    assert!(result.is_none());
}

#[test]
fn resolve_bundle_upload_no_url_bails() {
    let data = Some((vec![1, 2, 3], "abc123".to_string()));
    let result = resolve_bundle_upload(data, None);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("bundle upload URL"), "unexpected error: {msg}");
}

#[test]
fn resolve_bundle_upload_both_none() {
    let result = resolve_bundle_upload(None, None).unwrap();
    assert!(result.is_none());
}

// ── resolve_health_check ─────────────────────────────────────

#[test]
fn health_check_flag_wins_over_config_and_autodetect() {
    let dir = tempdir().unwrap();
    // Create a detectable endpoint that should be ignored
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Http(
        "/from-config".to_string(),
    ));

    let detection = make_detection("nextjs", None);
    let result = resolve_health_check(
        Some("/from-flag"),
        &config,
        dir.path(),
        &detection,
        dir.path(),
        true, // json mode suppresses output
    )
    .unwrap();

    assert_eq!(result.path, Some("/from-flag".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Flag));
}

#[test]
fn health_check_config_wins_over_autodetect() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Http(
        "/from-config".to_string(),
    ));

    let detection = make_detection("nextjs", None);
    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert_eq!(result.path, Some("/from-config".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Config));
}

#[test]
fn health_check_autodetect_when_no_flag_or_config() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("nextjs", None);

    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert_eq!(result.path, Some("/api/health".to_string()));
    assert!(matches!(result.source, HealthCheckSource::Detected));
}

#[test]
fn health_check_default_tcp_when_nothing_found() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("other", None);

    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert!(result.path.is_none());
    assert!(matches!(result.source, HealthCheckSource::Default));
}

#[test]
fn health_check_flag_none_gives_tcp() {
    let dir = tempdir().unwrap();
    let config = nrz::config::ProjectConfig::default();
    let detection = make_detection("other", None);

    for alias in &["none", "NONE", "false", "tcp", "TCP", "None"] {
        let result = resolve_health_check(
            Some(alias),
            &config,
            dir.path(),
            &detection,
            dir.path(),
            true,
        )
        .unwrap();

        assert!(
            result.path.is_none(),
            "expected TCP for alias \"{alias}\", got: {:?}",
            result.path
        );
        assert!(matches!(result.source, HealthCheckSource::Flag));
    }
}

#[test]
fn health_check_config_tcp_overrides_autodetect() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("app/api/health")).unwrap();
    fs::write(dir.path().join("app/api/health/route.ts"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.deploy.health_check_path = Some(nrz::config::HealthCheckPathConfig::Tcp);

    let detection = make_detection("nextjs", None);
    let result =
        resolve_health_check(None, &config, dir.path(), &detection, dir.path(), true).unwrap();

    assert!(result.path.is_none());
    assert!(matches!(result.source, HealthCheckSource::Config));
}

// ── validate_health_path ─────────────────────────────────────

#[test]
fn validate_health_path_rejects_no_slash() {
    let result = validate_health_path("health", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must start with '/'")
    );
}

#[test]
fn validate_health_path_rejects_parent_traversal() {
    let result = validate_health_path("/../../etc", "--health-check-path");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("'..'"));
}

#[test]
fn validate_health_path_rejects_query() {
    let result = validate_health_path("/health?v=1", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("query or fragment")
    );
}

#[test]
fn validate_health_path_rejects_fragment() {
    let result = validate_health_path("/health#section", "--health-check-path");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("query or fragment")
    );
}

#[test]
fn validate_health_path_accepts_valid_path() {
    assert!(validate_health_path("/health", "--health-check-path").is_ok());
    assert!(validate_health_path("/api/health", "--health-check-path").is_ok());
    assert!(validate_health_path("/v1/healthz", "--health-check-path").is_ok());
}

// ── ComputeConfigBody serialization ─────────────────────────

#[test]
fn compute_config_body_with_health_check_path_serializes_camel_case() {
    let body = ComputeConfigBody {
        health_check_path: Some("/api/health".to_string()),
    };
    let value = serde_json::to_value(&body).unwrap();
    assert_eq!(
        value.get("healthCheckPath").and_then(|v| v.as_str()),
        Some("/api/health")
    );
    assert!(value.get("health_check_path").is_none());
}

#[test]
fn compute_config_body_without_path_omits_field() {
    let body = ComputeConfigBody {
        health_check_path: None,
    };
    let value = serde_json::to_value(&body).unwrap();
    assert!(value.get("healthCheckPath").is_none());
}

// ── precompressed_dirs / scan_and_maybe_compress ─────────────

#[test]
fn precompressed_dirs_empty_when_no_manifest() {
    let dirs = precompressed_dirs(None);
    assert!(dirs.is_empty());
}

#[test]
fn precompressed_dirs_empty_when_no_static_precompressed() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist" }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#,
    )
    .unwrap();
    let dirs = precompressed_dirs(Some(&manifest));
    assert!(dirs.is_empty());
}

#[test]
fn precompressed_dirs_includes_precompressed_static() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            { "name": "assets", "target": "STATIC", "directory": "client",
              "isPrecompressed": true },
            { "name": "server", "target": "ISOLATE", "directory": "server",
              "entry": "e.mjs", "export": "fetch" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "server" }]
    }"#,
    )
    .unwrap();
    let dirs = precompressed_dirs(Some(&manifest));
    assert_eq!(dirs, vec!["client/"]);
}

#[test]
fn precompressed_dirs_root_dot_gives_dot() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": ".",
                     "isPrecompressed": true }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#,
    )
    .unwrap();
    let dirs = precompressed_dirs(Some(&manifest));
    assert_eq!(dirs, vec!["."]);
}

#[test]
fn scan_and_compress_compresses_precompressed_files() {
    let dir = tempfile::tempdir().unwrap();
    let content = "Hello, world! This is a test file with some content to compress.";
    fs::write(dir.path().join("index.html"), content).unwrap();

    let pc_dirs = vec![".".to_string()];
    let (entries, compressed) = scan_and_maybe_compress(dir.path(), &pc_dirs).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(compressed.contains_key("index.html"));

    let br_bytes = compressed.get("index.html").unwrap();
    // Compressed size is reported in the entry
    assert_eq!(entries[0].size, br_bytes.len() as u64);
    // Compressed bytes differ from original
    assert_ne!(br_bytes.as_slice(), content.as_bytes());
    // Verify output differs from raw input (brotli has no magic bytes, but the stream is structurally different)
    assert_ne!(&br_bytes[..4], &content.as_bytes()[..4]);
    // SHA-256 is computed from the original content, not from compressed
    let expected_hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));
    assert_eq!(entries[0].sha256.as_deref(), Some(expected_hash.as_str()));
}

#[test]
fn scan_and_compress_leaves_other_dirs_uncompressed() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("client")).unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    // Use repetitive content that brotli will reliably compress
    let content = "function hello(){return 42;} ".repeat(20);
    let content = content.as_str();
    fs::write(dir.path().join("client/app.js"), content).unwrap();
    fs::write(dir.path().join("server/entry.mjs"), content).unwrap();

    // Only "client/" is precompressed
    let pc_dirs = vec!["client/".to_string()];
    let (entries, compressed) = scan_and_maybe_compress(dir.path(), &pc_dirs).unwrap();

    assert_eq!(entries.len(), 2);
    assert!(
        compressed.contains_key("client/app.js"),
        "client/app.js should be compressed"
    );
    assert!(
        !compressed.contains_key("server/entry.mjs"),
        "server/entry.mjs should not be compressed"
    );

    // server file size matches original raw bytes (not compressed)
    let server_entry = entries
        .iter()
        .find(|e| e.path == "server/entry.mjs")
        .unwrap();
    assert_eq!(
        server_entry.size,
        content.len() as u64,
        "server file should report raw size"
    );
}

#[test]
fn scan_and_compress_empty_precompressed_list_behaves_like_scan_files() {
    let dir = tempfile::tempdir().unwrap();
    let content = "body { color: red; }";
    fs::write(dir.path().join("style.css"), content).unwrap();

    let (entries, compressed) = scan_and_maybe_compress(dir.path(), &[]).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(compressed.is_empty());
    assert_eq!(entries[0].size, content.len() as u64);
}

#[test]
fn is_precompressed_path_root_dot_matches_all() {
    assert!(is_precompressed_path("index.html", &[".".to_string()]));
    assert!(is_precompressed_path("assets/app.js", &[".".to_string()]));
    assert!(is_precompressed_path(
        "deep/nested/file.css",
        &[".".to_string()]
    ));
}

#[test]
fn is_precompressed_path_prefix_matches_only_subtree() {
    let dirs = vec!["client/".to_string()];
    assert!(is_precompressed_path("client/app.js", &dirs));
    assert!(is_precompressed_path("client/sub/deep.css", &dirs));
    assert!(!is_precompressed_path("server/entry.mjs", &dirs));
    assert!(!is_precompressed_path("index.html", &dirs));
}

#[test]
fn is_precompressed_path_false_prefix_not_matched() {
    let dirs = vec!["client/".to_string()];
    // "clientsecrets/" shares the "client" prefix but must NOT match "client/"
    assert!(!is_precompressed_path("clientsecrets/key.pem", &dirs));
    assert!(!is_precompressed_path("client-extra/file.js", &dirs));
    assert!(!is_precompressed_path("clientx", &dirs));
}

#[test]
fn precompressed_dirs_only_precompressed_layers_included() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            { "name": "assets", "target": "STATIC", "directory": "client",
              "isPrecompressed": true },
            { "name": "public", "target": "STATIC", "directory": "public" }
        ],
        "routes": [{ "pattern": "^/.*$", "layer": "assets" }]
    }"#,
    )
    .unwrap();
    let dirs = precompressed_dirs(Some(&manifest));
    assert_eq!(dirs, vec!["client/"]);
    assert!(
        !dirs.iter().any(|d| d == "public/"),
        "public/ should not appear"
    );
}

#[test]
fn precompressed_dirs_excludes_explicit_false() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "dist",
                     "isPrecompressed": false }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#,
    )
    .unwrap();
    let dirs = precompressed_dirs(Some(&manifest));
    assert!(dirs.is_empty());
}

#[test]
fn precompressed_dirs_trailing_slash_not_doubled() {
    // directory value already has trailing slash in manifest — must not produce "client//"
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [{ "name": "s", "target": "STATIC", "directory": "client/",
                     "isPrecompressed": true }],
        "routes": [{ "pattern": "^/.*$", "layer": "s" }]
    }"#,
    )
    .unwrap();
    let dirs = precompressed_dirs(Some(&manifest));
    assert_eq!(dirs, vec!["client/"]);
}

fn decompress_brotli(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    brotli::BrotliDecompress(&mut std::io::Cursor::new(data), &mut out)
        .expect("brotli decompression failed");
    out
}

#[test]
fn brotli_compress_roundtrip() {
    let original = b"Hello, world! This is test data for brotli roundtrip verification.";
    let compressed = brotli_compress(original).unwrap();
    let decompressed = decompress_brotli(&compressed);
    assert_eq!(decompressed, original);
}

#[test]
fn brotli_compress_empty_roundtrip() {
    let original: &[u8] = &[];
    let compressed = brotli_compress(original).unwrap();
    let decompressed = decompress_brotli(&compressed);
    assert_eq!(decompressed, original);
}

#[test]
fn scan_and_compress_skips_brotli_when_expansion() {
    let dir = tempfile::tempdir().unwrap();
    // 1 byte: brotli stream overhead always exceeds 1 byte, so expansion is guaranteed
    let content = b"x";
    fs::write(dir.path().join("tiny.js"), content).unwrap();

    let pc_dirs = vec![".".to_string()];
    let (entries, compressed) = scan_and_maybe_compress(dir.path(), &pc_dirs).unwrap();

    assert_eq!(entries.len(), 1);
    assert!(
        !compressed.contains_key("tiny.js"),
        "tiny file must not be pre-compressed when brotli expands it"
    );
    assert_eq!(
        entries[0].size,
        content.len() as u64,
        "raw size must be reported"
    );
    // SHA-256 must be computed from the original content even when brotli expands
    let expected_hash = format!("{:x}", sha2::Sha256::digest(content));
    assert_eq!(entries[0].sha256.as_deref(), Some(expected_hash.as_str()));
}

#[test]
fn scan_and_compress_empty_file_not_compressed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("empty.html"), b"").unwrap();

    let pc_dirs = vec![".".to_string()];
    let (entries, compressed) = scan_and_maybe_compress(dir.path(), &pc_dirs).unwrap();

    assert_eq!(entries.len(), 1);
    // Empty file: any brotli stream is larger than 0 bytes, so falls back to raw
    assert!(
        !compressed.contains_key("empty.html"),
        "empty file should not be in compressed_map"
    );
    assert_eq!(entries[0].size, 0, "empty file size must be 0");
    // SHA-256 of empty content is a well-known constant
    assert_eq!(
        entries[0].sha256.as_deref(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
}

// ── static_layer_dirs / is_in_layer_dirs ────────────────────

#[test]
fn static_layer_dirs_returns_only_static_layers() {
    let manifest: crate::build::manifest::Manifest = serde_json::from_str(
        r#"{
        "version": 1,
        "layers": [
            { "name": "cdn", "target": "STATIC", "directory": "_static" },
            { "name": "pub", "target": "STATIC", "directory": "public" },
            { "name": "srv", "target": "COMPUTE", "directory": ".", "entry": "server.js" }
        ],
        "routes": [
            { "pattern": "^/_next/.*$", "layer": "cdn", "priority": 100 },
            { "pattern": "^/.*$", "layer": "pub", "priority": 50 },
            { "pattern": "^/.*$", "layer": "srv", "priority": 0 }
        ]
    }"#,
    )
    .unwrap();
    let dirs = static_layer_dirs(Some(&manifest));
    assert_eq!(dirs, vec!["_static/", "public/"]);
}

#[test]
fn static_layer_dirs_none_manifest_returns_empty() {
    assert!(static_layer_dirs(None).is_empty());
}

#[test]
fn is_in_layer_dirs_matches_prefix() {
    let dirs = vec!["_static/".to_string(), "public/".to_string()];
    assert!(is_in_layer_dirs(
        "_static/_next/static/chunks/main.js",
        &dirs
    ));
    assert!(is_in_layer_dirs("public/favicon.ico", &dirs));
    assert!(!is_in_layer_dirs("node_modules/react/index.js", &dirs));
    assert!(!is_in_layer_dirs("server.js", &dirs));
    assert!(!is_in_layer_dirs(".next/server/app/page.js", &dirs));
}

#[test]
fn is_in_layer_dirs_root_dir_matches_all() {
    let dirs = vec![".".to_string()];
    assert!(is_in_layer_dirs("anything.js", &dirs));
    assert!(is_in_layer_dirs("deep/nested/file.txt", &dirs));
}

#[test]
fn file_list_filtered_for_process_with_manifest() {
    // Simulate what happens in run(): scan produces all files, then STATIC filter
    // keeps only files belonging to STATIC layer directories.
    let all_files = vec![
        FileEntry {
            path: "_static/_next/static/chunks/main.js".into(),
            size: 100,
            sha256: None,
        },
        FileEntry {
            path: "public/favicon.ico".into(),
            size: 50,
            sha256: None,
        },
        FileEntry {
            path: "server.js".into(),
            size: 200,
            sha256: None,
        },
        FileEntry {
            path: "node_modules/react/index.js".into(),
            size: 300,
            sha256: None,
        },
        FileEntry {
            path: ".next/server/app/page.js".into(),
            size: 400,
            sha256: None,
        },
    ];
    let sd = vec!["_static/".to_string(), "public/".to_string()];
    let filtered: Vec<_> = all_files
        .into_iter()
        .filter(|f| is_in_layer_dirs(&f.path, &sd))
        .collect();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].path, "_static/_next/static/chunks/main.js");
    assert_eq!(filtered[1].path, "public/favicon.ico");
}

// ── is_nextjs_project ───────────────────────────────────────

#[test]
fn is_nextjs_detects_next_in_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"next":"^16.2.0","react":"^19.0.0"}}"#,
    )
    .unwrap();
    assert!(is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_detects_next_in_dev_dependencies() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"next":"16.2.0"}}"#,
    )
    .unwrap();
    assert!(is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_false_for_non_next_project() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"react":"^19.0.0","vite":"^6.0.0"}}"#,
    )
    .unwrap();
    assert!(!is_nextjs_project(dir.path()));
}

#[test]
fn is_nextjs_false_without_package_json() {
    let dir = tempdir().unwrap();
    assert!(!is_nextjs_project(dir.path()));
}
