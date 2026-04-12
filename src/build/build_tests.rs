use super::{
    collect_body_files, compute_aware_output_dirs, copy_dir_recursive,
    copy_missing_prisma_packages, detect_output_dir, prepare_nextjs_standalone, run_with_hint,
    try_generate_ssr_manifest,
};
use crate::cli::BuildArgs;

#[test]
fn framework_dirs_checked_before_config_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // Both dirs exist, but framework-specific should be preferred
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[".next"], None).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
}

#[test]
fn manifest_dir_wins_over_plain_dir() {
    let dir = tempfile::tempdir().unwrap();
    // "dist" exists as plain dir, ".output" has .onreza/ inside
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".output/.onreza")).unwrap();

    let (found, has_manifest) =
        detect_output_dir(dir.path(), &["dist", ".output"], &[], None).unwrap();
    assert_eq!(found.file_name().unwrap(), ".output");
    assert!(has_manifest);
}

#[test]
fn dedup_preserves_order() {
    let dir = tempfile::tempdir().unwrap();
    // "dist" is in both framework_dirs and config_dirs
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &["dist", "build"], None).unwrap();
    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn error_lists_all_checked_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // No dirs exist
    let err = detect_output_dir(dir.path(), &["build"], &[".next", "out"], None).unwrap_err();
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

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[], None).unwrap();
    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn framework_manifest_dir_wins_over_config_manifest_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Both have .onreza/, but framework dir should be checked first
    std::fs::create_dir_all(dir.path().join("dist/.onreza")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist"], &[".next"], None).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
    assert!(has_manifest);
}

// ── detect_output_dir with server_output_dir ─────────────────

#[test]
fn server_output_dir_used_when_no_framework_or_config_match() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("server-out")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[], Some("server-out")).unwrap();
    assert_eq!(found.file_name().unwrap(), "server-out");
}

#[test]
fn framework_dir_wins_over_server_output_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".next")).unwrap();
    std::fs::create_dir(dir.path().join("server-out")).unwrap();

    let (found, _) =
        detect_output_dir(dir.path(), &["dist"], &[".next"], Some("server-out")).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
}

#[test]
fn server_output_dir_wins_over_config_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("server-out")).unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    // server-out is checked before config "dist"
    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[], Some("server-out")).unwrap();
    assert_eq!(found.file_name().unwrap(), "server-out");
}

#[test]
fn server_output_dir_appears_in_error_when_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let err = detect_output_dir(dir.path(), &["dist"], &[".next"], Some("server-out")).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("server-out/"),
        "error should list server-out: {msg}"
    );
}

// ── compute_aware_output_dirs ────────────────────────────────

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
fn nextjs_default_ssr_includes_standalone_probe() {
    let detection = make_detection("nextjs", None);
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec![".next/standalone", ".next"]);
}

#[test]
fn nextjs_standalone_returns_standalone_first() {
    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec![".next/standalone", ".next"]);
}

#[test]
fn nextjs_export_returns_out() {
    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: true,
        ssr_features: vec!["output: 'export' (static)".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["out"]);
}

#[test]
fn vite_delegates_to_presets() {
    let detection = make_detection("vite", None);
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["dist"]);
}

#[test]
fn nextjs_standalone_found_before_dot_next() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/server")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist", ".output", "build"],
        &[".next/standalone", ".next"],
        None,
    )
    .unwrap();
    assert!(
        found.ends_with(".next/standalone"),
        "should find standalone first, got: {}",
        found.display()
    );
}

// ── STATIC auto-gen in run_with_hint ─────────────────────────

fn make_static_detection(framework: &str) -> crate::detect::types::DetectionResult {
    crate::detect::types::DetectionResult {
        framework: framework.to_string(),
        name: framework.to_string(),
        version: None,
        suggested_compute: crate::detect::types::ComputeType::Static,
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
            ssr_analysis: None,

            structure: vec![],
        },
    }
}

#[tokio::test]
async fn static_project_without_adapter_auto_generates_manifest() {
    let dir = tempfile::tempdir().unwrap();
    // Create a "dist" output dir with a file but no .onreza/ subdir
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("dist/index.html"), "<h1>hi</h1>").unwrap();

    let detection = make_static_detection("vite");
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("STATIC auto-gen should produce a manifest");
    assert_eq!(manifest.layers.len(), 1);
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
    assert_eq!(manifest.routes.len(), 1);
    assert_eq!(manifest.routes[0].pattern, "^/.*$");
}

#[tokio::test]
async fn process_project_without_adapter_returns_no_manifest_from_build() {
    let dir = tempfile::tempdir().unwrap();
    // Create a "dist" output dir — no .onreza/ subdir, non-static detection
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("dist/server.js"), "console.log('ok')").unwrap();

    let detection = make_detection("other", None); // suggested_compute == Process
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    // PROCESS without adapter: build returns None, deploy step will auto-gen manifest later
    assert!(result.manifest.is_none());
}

// ── prepare_nextjs_standalone ────────────────────────────────

#[test]
fn nextjs_standalone_prepares_static_with_correct_nesting() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // Create .next/static/chunks/main.js in project dir
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    // Create server.js in output dir
    std::fs::write(output.path().join("server.js"), "// server").unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    // CDN static: _static/_next/static/chunks/main.js
    assert!(
        output
            .path()
            .join("_static/_next/static/chunks/main.js")
            .is_file()
    );
}

#[test]
fn nextjs_standalone_copies_static_for_server() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".next/static/css")).unwrap();
    std::fs::write(project.path().join(".next/static/css/style.css"), "body{}").unwrap();
    std::fs::write(output.path().join("server.js"), "// server").unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    // Server-side: .next/static/css/style.css
    assert!(output.path().join(".next/static/css/style.css").is_file());
}

#[test]
fn nextjs_standalone_copies_public() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join("public")).unwrap();
    std::fs::write(project.path().join("public/favicon.ico"), "icon").unwrap();
    std::fs::write(output.path().join("server.js"), "// server").unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    assert!(output.path().join("public/favicon.ico").is_file());
}

#[test]
fn nextjs_standalone_without_public_dir() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // No public/ dir in project
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();
    std::fs::write(output.path().join("server.js"), "// server").unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    // public/ should not exist in output
    assert!(!output.path().join("public").is_dir());
    // _static/ should still be created
    assert!(
        output
            .path()
            .join("_static/_next/static/chunks/main.js")
            .is_file()
    );
}

#[test]
fn nextjs_standalone_does_not_overwrite_existing() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// original",
    )
    .unwrap();

    // Pre-create destination with different content — simulates a previous build
    // or an adapter that already prepared the output. prepare_nextjs_standalone
    // must not overwrite to avoid clobbering adapter-generated files.
    std::fs::create_dir_all(output.path().join("_static/_next/static/chunks")).unwrap();
    std::fs::write(
        output.path().join("_static/_next/static/chunks/main.js"),
        "// existing",
    )
    .unwrap();
    std::fs::create_dir_all(output.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        output.path().join(".next/static/chunks/main.js"),
        "// existing-server",
    )
    .unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    // Existing content should not be overwritten
    let cdn_content =
        std::fs::read_to_string(output.path().join("_static/_next/static/chunks/main.js")).unwrap();
    assert_eq!(cdn_content, "// existing");
    let server_content =
        std::fs::read_to_string(output.path().join(".next/static/chunks/main.js")).unwrap();
    assert_eq!(server_content, "// existing-server");
}

#[tokio::test]
async fn nextjs_standalone_run_with_hint_generates_manifest() {
    let project = tempfile::tempdir().unwrap();

    // Create Next.js standalone output structure
    std::fs::create_dir_all(project.path().join(".next/standalone")).unwrap();
    std::fs::write(
        project.path().join(".next/standalone/server.js"),
        "// server",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join("public")).unwrap();
    std::fs::write(project.path().join("public/favicon.ico"), "icon").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Next.js standalone should produce a manifest");
    assert_eq!(manifest.layers.len(), 3);
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
    assert_eq!(manifest.layers[0].directory, "_static");
    assert_eq!(
        manifest.layers[1].target,
        super::manifest::LayerTarget::Static
    );
    assert_eq!(manifest.layers[1].directory, "public");
    assert_eq!(
        manifest.layers[2].target,
        super::manifest::LayerTarget::Compute
    );
    assert_eq!(manifest.layers[2].entry.as_deref(), Some("server.js"));
    assert_eq!(manifest.routes.len(), 3);

    // Verify files were copied correctly
    let output = &result.output_dir;
    assert!(output.join("_static/_next/static/chunks/main.js").is_file());
    assert!(output.join(".next/static/chunks/main.js").is_file());
    assert!(output.join("public/favicon.ico").is_file());
}

#[tokio::test]
async fn nextjs_standalone_run_with_hint_without_public_generates_2_layer_manifest() {
    let project = tempfile::tempdir().unwrap();

    // Create Next.js standalone without public/
    std::fs::create_dir_all(project.path().join(".next/standalone")).unwrap();
    std::fs::write(
        project.path().join(".next/standalone/server.js"),
        "// server",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Next.js standalone should produce a manifest");
    assert_eq!(manifest.layers.len(), 2, "no public/ → 2 layers");
    assert_eq!(manifest.routes.len(), 2, "no public/ → 2 routes");
    assert_eq!(manifest.layers[0].directory, "_static");
    assert_eq!(manifest.layers[1].directory, ".");
}

#[test]
fn nextjs_standalone_without_next_static_dir() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    // No .next/static/ at all — prepare should succeed but not create _static/
    std::fs::write(output.path().join("server.js"), "// server").unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    assert!(!output.path().join("_static").exists());
    assert!(!output.path().join(".next/static").exists());
}

#[tokio::test]
async fn nextjs_standalone_missing_server_js_is_error() {
    let project = tempfile::tempdir().unwrap();

    // Create standalone dir WITHOUT server.js
    std::fs::create_dir_all(project.path().join(".next/standalone")).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'standalone'".into()],
    };
    let detection = make_detection("nextjs", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let err = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("server.js not found"),
        "expected server.js error, got: {err}"
    );
}

#[test]
fn copy_dir_recursive_skips_symlinks() {
    let src = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("real.txt"), "real content").unwrap();
    std::os::unix::fs::symlink(src.path().join("real.txt"), src.path().join("link.txt")).unwrap();

    let dst = tempfile::tempdir().unwrap();
    let dst_sub = dst.path().join("out");
    copy_dir_recursive(src.path(), &dst_sub).unwrap();

    assert!(dst_sub.join("real.txt").is_file());
    assert!(
        !dst_sub.join("link.txt").exists(),
        "symlinks should be skipped"
    );
}

#[test]
fn copy_dir_recursive_empty_src_creates_dst() {
    let src = tempfile::tempdir().unwrap();
    // src is empty — no files at all
    let dst = tempfile::tempdir().unwrap();
    let dst_sub = dst.path().join("out");

    copy_dir_recursive(src.path(), &dst_sub).unwrap();

    assert!(dst_sub.is_dir(), "empty src should still create dst");
    assert_eq!(
        std::fs::read_dir(&dst_sub).unwrap().count(),
        0,
        "dst should be empty"
    );
}

#[test]
fn copy_dir_recursive_nested_directories() {
    let src = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("a/b/c")).unwrap();
    std::fs::write(src.path().join("root.txt"), "root").unwrap();
    std::fs::write(src.path().join("a/level1.txt"), "l1").unwrap();
    std::fs::write(src.path().join("a/b/level2.txt"), "l2").unwrap();
    std::fs::write(src.path().join("a/b/c/level3.txt"), "l3").unwrap();

    let dst = tempfile::tempdir().unwrap();
    let dst_sub = dst.path().join("out");
    copy_dir_recursive(src.path(), &dst_sub).unwrap();

    assert_eq!(
        std::fs::read_to_string(dst_sub.join("root.txt")).unwrap(),
        "root"
    );
    assert_eq!(
        std::fs::read_to_string(dst_sub.join("a/level1.txt")).unwrap(),
        "l1"
    );
    assert_eq!(
        std::fs::read_to_string(dst_sub.join("a/b/level2.txt")).unwrap(),
        "l2"
    );
    assert_eq!(
        std::fs::read_to_string(dst_sub.join("a/b/c/level3.txt")).unwrap(),
        "l3"
    );
}

// ── collect_body_files (metadata routes) ────────────────────

#[test]
fn metadata_routes_favicon_copied_to_public() {
    let output = tempfile::tempdir().unwrap();
    let app_dir = output.path().join(".next/server/app");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(app_dir.join("favicon.ico.body"), b"\x00\x00\x01\x00").unwrap();
    std::fs::write(app_dir.join("favicon.ico.meta"), r#"{"status":200}"#).unwrap();

    let public_dst = output.path().join("public");
    std::fs::create_dir_all(&public_dst).unwrap();

    let mut copied = 0usize;
    collect_body_files(&app_dir, &app_dir, &public_dst, &mut copied).unwrap();

    assert_eq!(copied, 1);
    assert!(public_dst.join("favicon.ico").is_file());
    assert_eq!(
        std::fs::read(public_dst.join("favicon.ico")).unwrap(),
        b"\x00\x00\x01\x00"
    );
}

#[test]
fn metadata_routes_nested_copied_to_public() {
    let output = tempfile::tempdir().unwrap();
    let app_dir = output.path().join(".next/server/app");
    let og_dir = app_dir.join("og");
    std::fs::create_dir_all(&og_dir).unwrap();
    std::fs::write(og_dir.join("opengraph-image.png.body"), b"PNG").unwrap();

    let public_dst = output.path().join("public");
    std::fs::create_dir_all(&public_dst).unwrap();

    let mut copied = 0usize;
    collect_body_files(&app_dir, &app_dir, &public_dst, &mut copied).unwrap();

    assert_eq!(copied, 1);
    assert!(public_dst.join("og/opengraph-image.png").is_file());
}

#[test]
fn metadata_routes_skips_existing() {
    let output = tempfile::tempdir().unwrap();
    let app_dir = output.path().join(".next/server/app");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(app_dir.join("favicon.ico.body"), b"new").unwrap();

    let public_dst = output.path().join("public");
    std::fs::create_dir_all(&public_dst).unwrap();
    // Pre-existing file — should not be overwritten
    std::fs::write(public_dst.join("favicon.ico"), b"existing").unwrap();

    let mut copied = 0usize;
    collect_body_files(&app_dir, &app_dir, &public_dst, &mut copied).unwrap();

    assert_eq!(copied, 0, "should skip existing file");
    assert_eq!(
        std::fs::read(public_dst.join("favicon.ico")).unwrap(),
        b"existing"
    );
}

#[test]
fn metadata_routes_ignores_meta_files() {
    let output = tempfile::tempdir().unwrap();
    let app_dir = output.path().join(".next/server/app");
    std::fs::create_dir_all(&app_dir).unwrap();
    // Only .meta, no .body — nothing should be copied
    std::fs::write(app_dir.join("favicon.ico.meta"), r#"{"status":200}"#).unwrap();

    let public_dst = output.path().join("public");
    std::fs::create_dir_all(&public_dst).unwrap();

    let mut copied = 0usize;
    collect_body_files(&app_dir, &app_dir, &public_dst, &mut copied).unwrap();

    assert_eq!(copied, 0);
    assert!(!public_dst.join("favicon.ico").exists());
}

// ── SSR auto-manifest: Nuxt ─────────────────────────────────

#[tokio::test]
async fn nuxt_ssr_generates_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Create Nuxt .output/ structure
    std::fs::create_dir_all(dir.path().join(".output/public/_nuxt")).unwrap();
    std::fs::write(
        dir.path().join(".output/public/_nuxt/entry.abc123.js"),
        "// app",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join(".output/server")).unwrap();
    std::fs::write(dir.path().join(".output/server/index.mjs"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["server/api/ routes".into()],
    };
    let detection = make_detection("nuxt", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result.manifest.expect("Nuxt SSR should produce a manifest");
    assert_eq!(manifest.layers.len(), 2);
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
    assert_eq!(manifest.layers[0].directory, "public");
    assert_eq!(
        manifest.layers[1].target,
        super::manifest::LayerTarget::Compute
    );
    assert_eq!(manifest.layers[1].directory, "server");
    assert_eq!(manifest.layers[1].entry.as_deref(), Some("index.mjs"));
    assert_eq!(manifest.routes.len(), 2);
    assert_eq!(manifest.routes[0].pattern, "^/_nuxt/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
    assert_eq!(manifest.routes[1].pattern, "^/.*$");
    assert_eq!(manifest.routes[1].priority, Some(0));
}

#[tokio::test]
async fn nuxt_static_falls_through_to_static_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Nuxt static: .output/public/ has the static site
    std::fs::create_dir_all(dir.path().join(".output/public")).unwrap();
    std::fs::write(dir.path().join(".output/public/index.html"), "<h1>hi</h1>").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: true,
        ssr_features: vec!["ssr: false (static)".into()],
    };
    let mut detection = make_detection("nuxt", Some(ssr));
    detection.suggested_compute = crate::detect::types::ComputeType::Static;

    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("static Nuxt should produce a STATIC manifest");
    assert_eq!(manifest.layers.len(), 1);
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
}

#[test]
fn nuxt_ssr_output_dirs_prefer_dot_output() {
    let detection = make_detection(
        "nuxt",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: false,
            ssr_features: vec!["server/api/ routes".into()],
        }),
    );
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec![".output"]);
}

#[test]
fn nuxt_static_output_dirs_prefer_public() {
    let detection = make_detection(
        "nuxt",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: true,
            ssr_features: vec!["ssr: false (static)".into()],
        }),
    );
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec![".output/public", ".output"]);
}

#[tokio::test]
async fn nuxt_ssr_without_public_generates_compute_only() {
    let dir = tempfile::tempdir().unwrap();

    // Nuxt .output/ with server but no public/
    std::fs::create_dir_all(dir.path().join(".output/server")).unwrap();
    std::fs::write(dir.path().join(".output/server/index.mjs"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["server/api/ routes".into()],
    };
    let detection = make_detection("nuxt", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Nuxt SSR without public should produce a manifest");
    assert_eq!(manifest.layers.len(), 1, "no public/ → compute only");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Compute
    );
    assert_eq!(manifest.layers[0].entry.as_deref(), Some("index.mjs"));
}

// ── SSR auto-manifest: SvelteKit ────────────────────────────

#[tokio::test]
async fn sveltekit_ssr_generates_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Create SvelteKit build/ structure (adapter-node)
    std::fs::create_dir_all(dir.path().join("build/client/_app")).unwrap();
    std::fs::write(dir.path().join("build/client/_app/immutable.js"), "// app").unwrap();
    std::fs::write(dir.path().join("build/index.js"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["adapter-node (runtime)".into()],
    };
    let detection = make_detection("sveltekit", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("SvelteKit SSR should produce a manifest");
    assert_eq!(manifest.layers.len(), 2);
    assert_eq!(manifest.layers[0].directory, "client");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
    assert_eq!(manifest.layers[1].directory, ".");
    assert_eq!(manifest.layers[1].entry.as_deref(), Some("index.js"));
    assert_eq!(manifest.routes[0].pattern, "^/_app/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
}

#[tokio::test]
async fn sveltekit_ssr_without_client_generates_compute_only() {
    let dir = tempfile::tempdir().unwrap();

    // SvelteKit build with no client/ dir
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/index.js"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["adapter-node (runtime)".into()],
    };
    let detection = make_detection("sveltekit", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("SvelteKit SSR without client should produce a manifest");
    assert_eq!(manifest.layers.len(), 1, "no client/ → compute only");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Compute
    );
    assert_eq!(manifest.routes.len(), 1);
}

// ── SSR auto-manifest: Remix ────────────────────────────────

#[tokio::test]
async fn remix_ssr_generates_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Create Remix build/ structure
    std::fs::create_dir_all(dir.path().join("build/client/assets")).unwrap();
    std::fs::write(
        dir.path().join("build/client/assets/root-abc123.js"),
        "// app",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("build/server")).unwrap();
    std::fs::write(dir.path().join("build/server/index.js"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["route loaders".into()],
    };
    let detection = make_detection("remix", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Remix SSR should produce a manifest");
    assert_eq!(manifest.layers.len(), 2);
    assert_eq!(manifest.layers[0].directory, "client");
    assert_eq!(manifest.layers[1].directory, "server");
    assert_eq!(manifest.layers[1].entry.as_deref(), Some("index.js"));
    assert_eq!(manifest.routes[0].pattern, "^/assets/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
}

#[tokio::test]
async fn react_router_ssr_generates_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // React Router v7 uses the same build/ structure as Remix
    std::fs::create_dir_all(dir.path().join("build/client/assets")).unwrap();
    std::fs::write(
        dir.path().join("build/client/assets/root-abc123.js"),
        "// app",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("build/server")).unwrap();
    std::fs::write(dir.path().join("build/server/index.js"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["route loaders".into()],
    };
    let detection = make_detection("react-router", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("React Router SSR should produce a manifest");
    assert_eq!(manifest.layers.len(), 2);
    assert_eq!(manifest.layers[0].directory, "client");
    assert_eq!(manifest.layers[1].directory, "server");
    assert_eq!(manifest.layers[1].entry.as_deref(), Some("index.js"));
}

#[test]
fn remix_ssr_output_dirs_prefer_build_root() {
    let detection = make_detection(
        "remix",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: false,
            ssr_features: vec!["route loaders".into()],
        }),
    );
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["build"]);
}

#[test]
fn remix_static_output_dirs_prefer_build_client() {
    let detection = make_detection(
        "remix",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: true,
            ssr_features: vec!["ssr: false (SPA mode)".into()],
        }),
    );
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["build/client", "build"]);
}

#[tokio::test]
async fn remix_ssr_without_client_generates_compute_only() {
    let dir = tempfile::tempdir().unwrap();

    // Remix build with server but no client/
    std::fs::create_dir_all(dir.path().join("build/server")).unwrap();
    std::fs::write(dir.path().join("build/server/index.js"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["route loaders".into()],
    };
    let detection = make_detection("remix", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Remix SSR without client should produce a manifest");
    assert_eq!(manifest.layers.len(), 1, "no client/ → compute only");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Compute
    );
    assert_eq!(manifest.routes.len(), 1);
}

#[test]
fn react_router_ssr_output_dirs_prefer_build_root() {
    let detection = make_detection(
        "react-router",
        Some(crate::detect::types::SsrAnalysis {
            is_static_compatible: false,
            ssr_features: vec!["route loaders".into()],
        }),
    );
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["build"]);
}

// ── SSR auto-manifest: Astro ────────────────────────────────

#[tokio::test]
async fn astro_ssr_generates_manifest() {
    let dir = tempfile::tempdir().unwrap();

    // Create Astro SSR dist/ structure
    std::fs::create_dir_all(dir.path().join("dist/client/_astro")).unwrap();
    std::fs::write(
        dir.path().join("dist/client/_astro/index.abc123.js"),
        "// app",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("dist/server")).unwrap();
    std::fs::write(dir.path().join("dist/server/entry.mjs"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'server' (SSR)".into()],
    };
    let detection = make_detection("astro", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Astro SSR should produce a manifest");
    assert_eq!(manifest.layers.len(), 2);
    assert_eq!(manifest.layers[0].directory, "client");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
    assert_eq!(manifest.layers[1].directory, "server");
    assert_eq!(manifest.layers[1].entry.as_deref(), Some("entry.mjs"));
    assert_eq!(manifest.routes[0].pattern, "^/_astro/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
}

#[tokio::test]
async fn astro_ssr_without_client_generates_compute_only() {
    let dir = tempfile::tempdir().unwrap();

    // Astro SSR with no client/ dir
    std::fs::create_dir_all(dir.path().join("dist/server")).unwrap();
    std::fs::write(dir.path().join("dist/server/entry.mjs"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["output: 'server' (SSR)".into()],
    };
    let detection = make_detection("astro", Some(ssr));
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(args, true, &config, Some(&detection), None)
        .await
        .unwrap();

    let manifest = result
        .manifest
        .expect("Astro SSR without client should produce a manifest");
    assert_eq!(manifest.layers.len(), 1, "no client/ → compute only");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Compute
    );
    assert_eq!(manifest.layers[0].entry.as_deref(), Some("entry.mjs"));
}

// ── SSR auto-manifest: edge cases ───────────────────────────

#[test]
fn try_generate_ssr_returns_none_for_static_compatible() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/index.mjs"), "// server").unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: true,
        ssr_features: vec!["ssr: false (static)".into()],
    };
    let detection = make_detection("nuxt", Some(ssr));

    assert!(try_generate_ssr_manifest(&detection, dir.path()).is_none());
}

#[test]
fn try_generate_ssr_returns_none_for_missing_entry() {
    let dir = tempfile::tempdir().unwrap();
    // Create output dir but NOT the entry file
    std::fs::create_dir_all(dir.path().join("server")).unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["server/api/ routes".into()],
    };
    let detection = make_detection("nuxt", Some(ssr));

    assert!(try_generate_ssr_manifest(&detection, dir.path()).is_none());
}

#[test]
fn try_generate_ssr_returns_none_for_unknown_framework() {
    let dir = tempfile::tempdir().unwrap();

    let ssr = crate::detect::types::SsrAnalysis {
        is_static_compatible: false,
        ssr_features: vec!["some feature".into()],
    };
    let detection = make_detection("other", Some(ssr));

    assert!(try_generate_ssr_manifest(&detection, dir.path()).is_none());
}

#[test]
fn sveltekit_output_dirs_delegate_to_presets() {
    let detection = make_detection("sveltekit", None);
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["build"]);
}

#[test]
fn astro_output_dirs_delegate_to_presets() {
    let detection = make_detection("astro", None);
    let dirs = compute_aware_output_dirs(&detection);
    assert_eq!(dirs, vec!["dist"]);
}

#[test]
fn try_generate_ssr_returns_none_without_ssr_analysis() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("server")).unwrap();
    std::fs::write(dir.path().join("server/index.mjs"), "// server").unwrap();

    let detection = make_detection("nuxt", None);

    assert!(try_generate_ssr_manifest(&detection, dir.path()).is_none());
}

// ── copy_missing_prisma_packages ────────────────────────────

#[test]
fn prisma_client_hash_packages_copied_to_standalone() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // Create @prisma/client-<hash> in project node_modules
    let src = project
        .path()
        .join("node_modules/@prisma/client-2c3a283f134fdcb6");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("index.js"), "// prisma client").unwrap();
    std::fs::write(
        src.join("package.json"),
        r#"{"name":"@prisma/client-2c3a283f134fdcb6"}"#,
    )
    .unwrap();

    // Create standalone output with node_modules/@prisma/ but WITHOUT the hash package
    std::fs::create_dir_all(output.path().join("node_modules/@prisma/client")).unwrap();
    std::fs::write(
        output.path().join("node_modules/@prisma/client/index.js"),
        "// base client",
    )
    .unwrap();

    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    // Hash package should now exist in output
    let dst = output
        .path()
        .join("node_modules/@prisma/client-2c3a283f134fdcb6");
    assert!(dst.is_dir());
    assert!(dst.join("index.js").is_file());
    assert!(dst.join("package.json").is_file());
}

#[test]
fn prisma_packages_not_copied_when_already_present() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    let hash = "client-abc123";
    let src = project.path().join(format!("node_modules/@prisma/{hash}"));
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("index.js"), "// src version").unwrap();

    // Already present in output
    let dst = output.path().join(format!("node_modules/@prisma/{hash}"));
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(dst.join("index.js"), "// dst version").unwrap();

    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    // Should NOT overwrite — dst version should remain
    let content = std::fs::read_to_string(dst.join("index.js")).unwrap();
    assert_eq!(content, "// dst version");
}

#[test]
fn prisma_noop_when_no_prisma_in_project() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // No node_modules/@prisma/ at all
    std::fs::create_dir_all(project.path().join("node_modules")).unwrap();

    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    // Should not create anything in output
    assert!(!output.path().join("node_modules/@prisma").is_dir());
}

#[test]
fn prisma_skips_non_client_hash_packages() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // Create @prisma/engines and @prisma/client (not client-<hash>)
    std::fs::create_dir_all(project.path().join("node_modules/@prisma/engines")).unwrap();
    std::fs::write(
        project
            .path()
            .join("node_modules/@prisma/engines/schema.js"),
        "// engine",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join("node_modules/@prisma/client")).unwrap();
    std::fs::write(
        project.path().join("node_modules/@prisma/client/index.js"),
        "// client",
    )
    .unwrap();

    std::fs::create_dir_all(output.path().join("node_modules/@prisma")).unwrap();

    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    // Neither engines nor client should be copied (only client-<hash> pattern)
    assert!(!output.path().join("node_modules/@prisma/engines").exists());
    assert!(!output.path().join("node_modules/@prisma/client").exists());
}

#[test]
fn prisma_copies_multiple_hash_packages() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    for hash in ["client-aaa111", "client-bbb222"] {
        let src = project.path().join(format!("node_modules/@prisma/{hash}"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("index.js"), hash).unwrap();
    }

    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    for hash in ["client-aaa111", "client-bbb222"] {
        let dst = output.path().join(format!("node_modules/@prisma/{hash}"));
        assert!(dst.is_dir(), "missing {hash}");
        assert_eq!(std::fs::read_to_string(dst.join("index.js")).unwrap(), hash);
    }
}

#[test]
fn prisma_standalone_integration_via_prepare() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // Set up minimal standalone structure
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(project.path().join(".next/static/chunks/main.js"), "// js").unwrap();
    std::fs::write(output.path().join("server.js"), "// server").unwrap();

    // Add Prisma hash package
    let src = project.path().join("node_modules/@prisma/client-deadbeef");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("index.js"), "// prisma").unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    // Static should be copied
    assert!(
        output
            .path()
            .join("_static/_next/static/chunks/main.js")
            .is_file()
    );
    // Prisma package should also be copied
    assert!(
        output
            .path()
            .join("node_modules/@prisma/client-deadbeef/index.js")
            .is_file()
    );
}

#[cfg(unix)]
#[test]
fn prisma_copies_through_symlink() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    // Simulate pnpm: real package lives elsewhere, symlink in node_modules
    let store = tempfile::tempdir().unwrap();
    let real_pkg = store.path().join("@prisma/client-sym123");
    std::fs::create_dir_all(&real_pkg).unwrap();
    std::fs::write(real_pkg.join("index.js"), "// from store").unwrap();

    let prisma_dir = project.path().join("node_modules/@prisma");
    std::fs::create_dir_all(&prisma_dir).unwrap();
    std::os::unix::fs::symlink(&real_pkg, prisma_dir.join("client-sym123")).unwrap();

    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    let dst = output.path().join("node_modules/@prisma/client-sym123");
    assert!(dst.is_dir());
    assert_eq!(
        std::fs::read_to_string(dst.join("index.js")).unwrap(),
        "// from store"
    );
}

#[cfg(unix)]
#[test]
fn prisma_skips_dangling_symlink() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    let prisma_dir = project.path().join("node_modules/@prisma");
    std::fs::create_dir_all(&prisma_dir).unwrap();
    // Symlink pointing to nonexistent path
    std::os::unix::fs::symlink("/nonexistent/path", prisma_dir.join("client-dangling")).unwrap();

    // Should not panic or error — just skip with a warning
    copy_missing_prisma_packages(project.path(), output.path(), true).unwrap();

    assert!(
        !output
            .path()
            .join("node_modules/@prisma/client-dangling")
            .exists()
    );
}
