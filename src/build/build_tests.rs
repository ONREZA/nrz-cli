use super::{compute_aware_output_dirs, detect_output_dir, run_with_hint};
use crate::cli::BuildArgs;

#[test]
fn framework_dirs_checked_before_config_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // Both dirs exist, but framework-specific should be preferred
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[".next"]).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
}

#[test]
fn manifest_dir_wins_over_plain_dir() {
    let dir = tempfile::tempdir().unwrap();
    // "dist" exists as plain dir, ".output" has .onreza/ inside
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".output/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist", ".output"], &[]).unwrap();
    assert_eq!(found.file_name().unwrap(), ".output");
    assert!(has_manifest);
}

#[test]
fn dedup_preserves_order() {
    let dir = tempfile::tempdir().unwrap();
    // "dist" is in both framework_dirs and config_dirs
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &["dist", "build"]).unwrap();
    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn error_lists_all_checked_dirs() {
    let dir = tempfile::tempdir().unwrap();
    // No dirs exist
    let err = detect_output_dir(dir.path(), &["build"], &[".next", "out"]).unwrap_err();
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

    let (found, _) = detect_output_dir(dir.path(), &["dist"], &[]).unwrap();
    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn framework_manifest_dir_wins_over_config_manifest_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Both have .onreza/, but framework dir should be checked first
    std::fs::create_dir_all(dir.path().join("dist/.onreza")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist"], &[".next"]).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
    assert!(has_manifest);
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
            ssr_adapter: None,
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
            ssr_adapter: None,
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

    let result = run_with_hint(args, true, &config, Some(&detection))
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

    let result = run_with_hint(args, true, &config, Some(&detection))
        .await
        .unwrap();

    // PROCESS without adapter: build returns None, deploy step will auto-gen manifest later
    assert!(result.manifest.is_none());
}
