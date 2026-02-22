use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn scan_files_flat_directory() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::write(dir.path().join("style.css"), "body{}").unwrap();

    let files = scan_files(dir.path()).unwrap();

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

    let files = scan_files(dir.path()).unwrap();

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

    let files = scan_files(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn scan_files_empty_directory() {
    let dir = tempdir().unwrap();
    let files = scan_files(dir.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn scan_files_sorted_alphabetically() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let files = scan_files(dir.path()).unwrap();

    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "m.txt");
    assert_eq!(files[2].path, "z.txt");
}

#[test]
fn scan_files_allows_double_dots_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file..backup.js"), "x").unwrap();

    let files = scan_files(dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "file..backup.js");
}

#[cfg(unix)]
#[test]
fn scan_files_skips_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let files = scan_files(dir.path()).unwrap();
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
        },
        FileEntry {
            path: "b.css".into(),
            size: 200,
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
    }];
    let files_b = vec![FileEntry {
        path: "b.js".into(),
        size: 100,
    }];

    assert_ne!(synthetic_sha(&files_a), synthetic_sha(&files_b));
}

#[test]
fn synthetic_sha_is_64_hex_chars() {
    let files = vec![FileEntry {
        path: "x.txt".into(),
        size: 1,
    }];
    let sha = synthetic_sha(&files);
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── detect_migrations tests ─────────────────────────────────

#[test]
fn detect_migrations_skip_flag() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();
    fs::write(
        dir.path().join("migrations/0001_init.sql"),
        "CREATE TABLE t;",
    )
    .unwrap();

    let result = detect_migrations(dir.path(), true, true, "migrations").unwrap();
    assert!(result.is_none());
}

#[test]
fn detect_migrations_no_migrations_dir() {
    let dir = tempdir().unwrap();

    let result = detect_migrations(dir.path(), true, false, "migrations").unwrap();
    assert!(result.is_none());
}

#[test]
fn detect_migrations_empty_migrations_dir() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();

    let result = detect_migrations(dir.path(), true, false, "migrations").unwrap();
    assert!(result.is_none());
}

#[test]
fn detect_migrations_returns_entries() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("migrations")).unwrap();
    fs::write(
        dir.path().join("migrations/0001_init.sql"),
        "CREATE TABLE t;",
    )
    .unwrap();
    fs::write(
        dir.path().join("migrations/0002_users.sql"),
        "CREATE TABLE users;",
    )
    .unwrap();

    let entries = detect_migrations(dir.path(), true, false, "migrations")
        .unwrap()
        .expect("should return migrations");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "0001_init");
    assert_eq!(entries[1].name, "0002_users");
    assert!(!entries[0].checksum.is_empty());
}

#[test]
fn detect_migrations_custom_dir() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("db/migrations")).unwrap();
    fs::write(
        dir.path().join("db/migrations/0001_init.sql"),
        "CREATE TABLE t;",
    )
    .unwrap();

    let entries = detect_migrations(dir.path(), true, false, "db/migrations")
        .unwrap()
        .expect("should find migrations in custom dir");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "0001_init");
}

// ── resolve_build_command tests ──────────────────────────────

#[test]
fn build_command_explicit_wins_over_config_and_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(Some("explicit cmd"), dir.path(), &config);
    assert_eq!(result.unwrap(), "explicit cmd");
}

#[test]
fn build_command_config_wins_over_auto() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.build.command = Some("config cmd".into());

    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "config cmd");
}

#[test]
fn build_command_auto_detect_bun_lock() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_bun_lockb() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("bun.lockb"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "bun run build");
}

#[test]
fn build_command_auto_detect_pnpm() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "pnpm run build");
}

#[test]
fn build_command_auto_detect_yarn() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "yarn run build");
}

#[test]
fn build_command_auto_detect_npm_fallback() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert_eq!(result.unwrap(), "npm run build");
}

#[test]
fn build_command_none_without_package_json() {
    let dir = tempdir().unwrap();

    let config = nrz::config::ProjectConfig::default();
    let result = resolve_build_command(None, dir.path(), &config);
    assert!(result.is_none());
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
    assert!(framework_static_hint("nextjs").contains("export"));
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
fn process_with_manifest_is_error() {
    let detection = make_detection("nextjs", None);
    let err = validate_compute_manifest_contract(ComputeType::Process, true, &detection)
        .expect_err("PROCESS with manifest should fail");
    assert!(err.to_string().contains("manifest"));
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
fn static_with_manifest_is_error() {
    let detection = make_detection("vite", None);
    let err = validate_compute_manifest_contract(ComputeType::Static, true, &detection)
        .expect_err("STATIC with manifest should fail");
    assert!(err.to_string().contains("manifest"));
}

#[test]
fn process_without_manifest_is_ok() {
    let detection = make_detection("nextjs", None);
    assert!(validate_compute_manifest_contract(ComputeType::Process, false, &detection).is_ok());
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
            ssr_adapter: None,
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
fn validate_nextjs_dot_next_with_standalone_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next");
    fs::create_dir(&output_dir).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok());
}

#[test]
fn validate_nextjs_standalone_dir_ok() {
    let dir = tempdir().unwrap();
    let output_dir = dir.path().join(".next/standalone");
    fs::create_dir_all(&output_dir).unwrap();

    let detection = make_detection("nextjs", None);
    let result = validate_process_output(&output_dir, dir.path(), &detection);
    assert!(result.is_ok()); // dir_name == "standalone", not ".next"
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
fn ensure_process_entry_uses_module_field_and_patches_main() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"app","module":"./server.mjs"}"#,
    )
    .unwrap();
    fs::write(dir.path().join("server.mjs"), "export default {}").unwrap();

    let detection = make_detection("other", None);
    ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();

    let pkg: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("package.json")).unwrap())
            .unwrap();
    assert_eq!(pkg["main"].as_str(), Some("server.mjs"));
}

#[test]
fn ensure_process_entry_ambiguous_candidates_returns_error() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("runtime")).unwrap();
    fs::write(dir.path().join("runtime/foo.mjs"), "console.log('foo')").unwrap();
    fs::write(dir.path().join("runtime/bar.mjs"), "console.log('bar')").unwrap();

    let detection = make_detection("other", None);
    let err = ensure_process_entry(dir.path(), dir.path(), None, &detection, true)
        .expect_err("expected ambiguity error");
    assert!(err.to_string().contains("multiple candidates"));
}

#[test]
fn ensure_process_entry_root_prefers_server_over_main() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("server.js"), "console.log('server')").unwrap();
    fs::write(dir.path().join("main.js"), "console.log('main')").unwrap();

    let detection = make_detection("other", None);
    ensure_process_entry(dir.path(), dir.path(), None, &detection, true).unwrap();

    let pkg: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("package.json")).unwrap())
            .unwrap();
    assert_eq!(pkg["main"].as_str(), Some("server.js"));
}

#[test]
fn ensure_process_entry_config_entry_allows_double_dot_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo..js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None);
    ensure_process_entry(dir.path(), dir.path(), Some("foo..js"), &detection, true).unwrap();

    let pkg: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("package.json")).unwrap())
            .unwrap();
    assert_eq!(pkg["main"].as_str(), Some("foo..js"));
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
