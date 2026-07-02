use super::{
    BuildSettingSource, OutputDirectoryHint, collect_body_files, copy_dir_recursive,
    copy_missing_prisma_packages, detect_output_dir, detect_output_dir_for_framework,
    prepare_nextjs_standalone, run_with_hint, try_generate_ssr_manifest,
};
use crate::cli::BuildArgs;
use crate::frameworks::compute_aware_output_dirs;

fn output_hint(path: &str, source: BuildSettingSource) -> OutputDirectoryHint<'_> {
    OutputDirectoryHint { path, source }
}

#[test]
fn build_output_serializes_nextjs_compatibility_report() {
    let output = super::BuildOutput {
        layers: vec![super::LayerInfo {
            name: "server".to_string(),
            target: "COMPUTE".to_string(),
            directory: ".".to_string(),
            entry: Some("server.js".to_string()),
        }],
        routes: 1,
        output_dir: ".next/standalone".to_string(),
        framework: Some("nextjs".to_string()),
        framework_version: Some("16.2.9".to_string()),
        compatibility: Some(serde_json::json!({
            "platform": {
                "prerenders": { "status": "partial_static_split" }
            }
        })),
    };

    let value = serde_json::to_value(output).unwrap();
    assert_eq!(
        value["compatibility"]["platform"]["prerenders"]["status"],
        "partial_static_split"
    );
}

fn write_manifest(output_dir: &std::path::Path) {
    std::fs::create_dir_all(output_dir.join(".onreza")).unwrap();
    std::fs::write(output_dir.join(".onreza/manifest.json"), "{}").unwrap();
}

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
    // "dist" exists as plain dir, ".output" has .onreza/manifest.json inside.
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    write_manifest(&dir.path().join(".output"));

    let (found, has_manifest) =
        detect_output_dir(dir.path(), &["dist", ".output"], &[], None).unwrap();
    assert_eq!(found.file_name().unwrap(), ".output");
    assert!(has_manifest);
}

#[test]
fn bare_onreza_dir_does_not_count_as_manifest() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir_all(dir.path().join(".output/.onreza")).unwrap();

    let (found, has_manifest) =
        detect_output_dir(dir.path(), &["dist", ".output"], &[], None).unwrap();

    assert_eq!(found.file_name().unwrap(), "dist");
    assert!(!has_manifest);
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
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .expect("missing-output error must carry a CodedError for Builder classification");
    assert_eq!(coded.code, "MISSING_BUILD_OUTPUT");
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
    // Both have .onreza/manifest.json, but framework dir should be checked first.
    write_manifest(&dir.path().join("dist"));
    write_manifest(&dir.path().join(".next"));

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist"], &[".next"], None).unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
    assert!(has_manifest);
}

#[test]
fn framework_existing_dir_wins_over_config_manifest_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".next")).unwrap();
    write_manifest(&dir.path().join("dist"));

    let (found, has_manifest) = detect_output_dir(dir.path(), &["dist"], &[".next"], None).unwrap();

    assert_eq!(found.file_name().unwrap(), ".next");
    assert!(!has_manifest);
}

// ── detect_output_dir with output_directory_hint ─────────────────

#[test]
fn output_directory_hint_used_when_no_framework_or_config_match() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("hint-out")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[],
        Some(output_hint("hint-out", BuildSettingSource::Preset)),
    )
    .unwrap();
    assert_eq!(found.file_name().unwrap(), "hint-out");
}

#[test]
fn framework_dir_wins_over_output_directory_hint() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".next")).unwrap();
    std::fs::create_dir(dir.path().join("hint-out")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next"],
        Some(output_hint("hint-out", BuildSettingSource::Preset)),
    )
    .unwrap();
    assert_eq!(found.file_name().unwrap(), ".next");
}

#[test]
fn output_directory_hint_wins_over_config_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("hint-out")).unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    // hint-out is checked before config "dist"
    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[],
        Some(output_hint("hint-out", BuildSettingSource::Preset)),
    )
    .unwrap();
    assert_eq!(found.file_name().unwrap(), "hint-out");
}

#[test]
fn output_directory_hint_appears_in_error_when_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let err = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next"],
        Some(output_hint("hint-out", BuildSettingSource::Preset)),
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("hint-out/"),
        "error should list hint-out: {msg}"
    );
}

#[test]
fn explicit_output_dir_missing_fails_without_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone")).unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let err = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint("custom-out", BuildSettingSource::User)),
    )
    .unwrap_err();
    let msg = err.to_string();

    assert!(msg.contains("explicit outputDirectory"), "{msg}");
    assert!(msg.contains("custom-out"), "{msg}");
    assert!(msg.contains("no fallback"), "{msg}");
}

#[test]
fn explicit_output_dir_wins_over_framework_and_config_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone/.onreza")).unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::create_dir(dir.path().join("hint-out")).unwrap();

    let (found, has_manifest) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint("hint-out", BuildSettingSource::User)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "hint-out");
    assert!(!has_manifest);
}

#[test]
fn user_output_dir_does_not_refine_to_nested_framework_output() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("build/client/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["build/client", "build"],
        Some(output_hint("build", BuildSettingSource::User)),
    )
    .unwrap();

    assert!(found.ends_with("build"));
    assert!(!has_manifest);
}

#[test]
fn user_root_output_dir_does_not_refine_to_framework_output() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("dist/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["dist"],
        Some(output_hint(".", BuildSettingSource::User)),
    )
    .unwrap();

    assert_eq!(found, dir.path().join("."));
    assert!(!has_manifest);
}

#[test]
fn nextjs_user_root_output_dir_allows_standalone_refinement() {
    let dir = tempfile::tempdir().unwrap();
    write_manifest(&dir.path().join(".next/standalone"));

    let (found, has_manifest) = detect_output_dir_for_framework(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint(".", BuildSettingSource::User)),
        "nextjs",
    )
    .unwrap();

    assert!(found.ends_with(".next/standalone"));
    assert!(has_manifest);
}

#[test]
fn nextjs_user_root_output_dir_preserves_root_manifest() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".onreza")).unwrap();
    std::fs::write(dir.path().join(".onreza/manifest.json"), "{}").unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir_for_framework(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint(".", BuildSettingSource::User)),
        "nextjs",
    )
    .unwrap();

    assert_eq!(found, dir.path().join("."));
    assert!(has_manifest);
}

#[test]
fn non_nextjs_user_root_output_dir_stays_exact() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("out/.onreza")).unwrap();

    let (found, has_manifest) = detect_output_dir_for_framework(
        dir.path(),
        &["dist"],
        &["out"],
        Some(output_hint(".", BuildSettingSource::User)),
        "vite",
    )
    .unwrap();

    assert_eq!(found, dir.path().join("."));
    assert!(!has_manifest);
}

#[test]
fn detected_output_dir_wins_over_framework_dir_without_being_strict() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("custom-dist")).unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["build"],
        &["dist"],
        Some(output_hint("custom-dist", BuildSettingSource::Detected)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "custom-dist");

    let (fallback, _) = detect_output_dir(
        dir.path(),
        &["build"],
        &["dist"],
        Some(output_hint(
            "missing-custom-dist",
            BuildSettingSource::Detected,
        )),
    )
    .unwrap();

    assert_eq!(fallback.file_name().unwrap(), "dist");
}

#[test]
fn detected_output_dir_wins_over_lower_priority_manifest_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("custom-dist")).unwrap();
    write_manifest(&dir.path().join("dist"));

    let (found, has_manifest) = detect_output_dir(
        dir.path(),
        &["build"],
        &["dist"],
        Some(output_hint("custom-dist", BuildSettingSource::Detected)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "custom-dist");
    assert!(!has_manifest);
}

#[test]
fn detected_root_allows_framework_refinement() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["fallback"],
        &["dist"],
        Some(output_hint(".", BuildSettingSource::Detected)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "dist");
}

#[test]
fn detected_root_stays_root_when_framework_root_is_current_output() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();

    let (found, has_manifest) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["."],
        Some(output_hint(".", BuildSettingSource::Detected)),
    )
    .unwrap();

    assert_eq!(found, dir.path().join("."));
    assert!(!has_manifest);
}

#[test]
fn detected_parent_allows_framework_refinement_matrix() {
    let cases = [
        (".next", ".next/standalone"),
        (".output", ".output/public"),
        ("build", "build/client"),
        ("dist", "dist/client"),
    ];

    for (parent, child) in cases {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(child)).unwrap();

        let (found, _) = detect_output_dir(
            dir.path(),
            &["fallback"],
            &[child, parent],
            Some(output_hint(parent, BuildSettingSource::Detected)),
        )
        .unwrap();

        assert!(
            found.ends_with(child),
            "detected {parent} should refine to {child}, got {}",
            found.display()
        );
    }
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

#[test]
fn nextjs_preset_output_dir_allows_standalone_refinement() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/server")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint(".next", BuildSettingSource::Preset)),
    )
    .unwrap();

    assert!(found.ends_with(".next/standalone"));
}

#[test]
fn nextjs_detected_dot_next_allows_standalone_refinement() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/server")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .unwrap();

    assert!(found.ends_with(".next/standalone"));
}

#[test]
fn nextjs_user_dot_next_allows_standalone_refinement() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next/server")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &[".next/standalone", ".next"],
        Some(output_hint(".next", BuildSettingSource::User)),
    )
    .unwrap();

    assert!(found.ends_with(".next/standalone"));
}

#[test]
fn user_project_root_refinement_is_limited_to_next_like_frameworks() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".next/standalone")).unwrap();
    std::fs::create_dir(dir.path().join("out")).unwrap();

    let (found, _) = detect_output_dir_for_framework(
        dir.path(),
        &["dist"],
        &[".next/standalone", "out"],
        Some(output_hint(".", BuildSettingSource::User)),
        "nextjs",
    )
    .unwrap();
    assert!(found.ends_with(".next/standalone"));

    let (found, _) = detect_output_dir_for_framework(
        dir.path(),
        &["dist"],
        &[".next/standalone", "out"],
        Some(output_hint(".", BuildSettingSource::User)),
        "vite",
    )
    .unwrap();
    assert_eq!(found, dir.path().join("."));
}

#[test]
fn nextjs_user_dot_next_allows_static_export_refinement() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("out")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["out"],
        Some(output_hint(".next", BuildSettingSource::User)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "out");
}

#[test]
fn detected_nextjs_dot_next_allows_static_export_refinement() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("out")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["out"],
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "out");
}

#[test]
fn nextjs_static_export_prefers_out_over_preset_dot_next() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("out")).unwrap();
    std::fs::create_dir_all(dir.path().join(".next")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["out"],
        Some(output_hint(".next", BuildSettingSource::Preset)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "out");
}

#[test]
fn generic_process_user_output_dir_wins_over_root_framework_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("hint-out")).unwrap();

    let (found, _) = detect_output_dir(
        dir.path(),
        &["dist"],
        &["."],
        Some(output_hint("hint-out", BuildSettingSource::User)),
    )
    .unwrap();

    assert_eq!(found.file_name().unwrap(), "hint-out");
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
async fn package_backed_static_html_prefers_build_artifact_over_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<h1>source</h1>").unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"vite build"}}"#,
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("dist")).unwrap();
    std::fs::write(dir.path().join("dist/index.html"), "<h1>built</h1>").unwrap();

    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let result = run_with_hint(args, true, &config, None, None)
        .await
        .unwrap();

    assert_eq!(result.output_dir, dir.path().join("dist"));
    let manifest = result
        .manifest
        .expect("STATIC auto-gen should produce a manifest");
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
}

#[tokio::test]
async fn package_backed_static_html_without_artifact_does_not_deploy_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<h1>source</h1>").unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"build":"echo no output"}}"#,
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("node_modules")).unwrap();

    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let err = run_with_hint(args, true, &config, None, None)
        .await
        .expect_err("package-backed static HTML without an artifact dir must fail");

    let msg = err.to_string();
    assert!(
        msg.contains("no output directory found"),
        "unexpected error: {msg}"
    );
    let coded = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("missing-output error must carry a CodedError");
    assert_eq!(coded.code, "MISSING_BUILD_OUTPUT");
}

#[tokio::test]
async fn configured_vite_static_output_generates_static_manifest_even_with_server_dep() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{
          "scripts": {"build": "vite build"},
          "dependencies": {"express": "^4.19.0", "react": "^18.3.0"},
          "devDependencies": {"vite": "^5.0.0", "@vitejs/plugin-react": "^4.0.0"}
        }"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("vite.config.js"), "x".repeat(600)).unwrap();
    std::fs::create_dir_all(dir.path().join("dist/assets")).unwrap();
    std::fs::write(
        dir.path().join("dist/index.html"),
        "<div id=\"root\"></div>",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("dist/assets/index.js"),
        "console.log('app')",
    )
    .unwrap();

    let mut config = nrz::config::ProjectConfig::default();
    config.project.framework = Some("vite".into());
    let args = BuildArgs {
        dir: dir.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        None,
        Some(output_hint("dist", BuildSettingSource::Preset)),
    )
    .await
    .unwrap();

    assert_eq!(result.output_dir, dir.path().join("dist"));
    let manifest = result
        .manifest
        .expect("Vite static output should auto-generate STATIC manifest");
    assert_eq!(manifest.layers.len(), 1);
    assert_eq!(
        manifest.layers[0].target,
        super::manifest::LayerTarget::Static
    );
    assert!(manifest.layers[0].entry.is_none());
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

#[cfg(unix)]
#[test]
fn nextjs_standalone_prunes_broken_pnpm_hoist_symlinks() {
    let project = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();

    std::fs::write(output.path().join("server.js"), "// server").unwrap();
    let pnpm_hoist = output.path().join("node_modules/.pnpm/node_modules");
    std::fs::create_dir_all(&pnpm_hoist).unwrap();
    std::fs::create_dir_all(
        output
            .path()
            .join("node_modules/.pnpm/left-pad@1.3.0/node_modules/left-pad"),
    )
    .unwrap();
    std::fs::create_dir_all(pnpm_hoist.join("@scope")).unwrap();
    std::fs::create_dir_all(
        output
            .path()
            .join("node_modules/.pnpm/@scope+valid@1.0.0/node_modules/@scope/valid"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../missing-semver@6.3.1/node_modules/semver",
        pnpm_hoist.join("semver"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../left-pad@1.3.0/node_modules/left-pad",
        pnpm_hoist.join("left-pad"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../../missing-scope@1.0.0/node_modules/@scope/pkg",
        pnpm_hoist.join("@scope/pkg"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        "../../@scope+valid@1.0.0/node_modules/@scope/valid",
        pnpm_hoist.join("@scope/valid"),
    )
    .unwrap();

    prepare_nextjs_standalone(project.path(), output.path(), true).unwrap();

    assert!(!pnpm_hoist.join("semver").exists());
    assert!(pnpm_hoist.join("left-pad").exists());
    assert!(!pnpm_hoist.join("@scope/pkg").exists());
    assert!(pnpm_hoist.join("@scope/valid").exists());
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
async fn nextjs_adapter_descriptor_generates_manifest_before_legacy_standalone() {
    let project = tempfile::tempdir().unwrap();

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
    std::fs::create_dir_all(project.path().join(".next/server/app")).unwrap();
    std::fs::write(
        project.path().join(".next/server/app/index.html"),
        "<main>Home</main>",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join("public")).unwrap();
    std::fs::write(project.path().join("public/robots.txt"), "User-agent: *").unwrap();
    std::fs::create_dir_all(project.path().join(".onreza")).unwrap();
    let static_file_path = project.path().join(".next/static/chunks/main.js");
    std::fs::write(
        project.path().join(".onreza/next-adapter-output.json"),
        format!(
            r#"{{
          "version": 1,
          "adapter": {{ "name": "@onreza/nrz-next-adapter", "version": "0.34.1" }},
          "nextVersion": "16.2.9",
          "buildId": "build-123",
          "outputs": {{
            "staticFiles": [{{
              "type": "STATIC_FILE",
              "pathname": "/_next/static/chunks/main.js",
              "filePath": "{}"
            }}],
            "prerenders": [{{
              "type": "PRERENDER",
              "pathname": "/",
              "fallback": {{
                "filePath": "{}",
                "initialHeaders": {{ "content-type": "text/html; charset=utf-8" }},
                "initialRevalidate": false
              }}
            }}]
          }}
        }}"#,
            static_file_path.display(),
            project.path().join(".next/server/app/index.html").display()
        ),
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
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
        .expect("Next.js adapter descriptor should produce a manifest");
    assert_eq!(manifest.layers.len(), 4);
    assert_eq!(manifest.layers[0].directory, "_static");
    assert_eq!(manifest.routes[0].pattern, "^/.*$");
    assert_eq!(manifest.routes[0].fallthrough, Some(true));
    assert_eq!(manifest.layers[1].name, "prerendered");
    assert_eq!(manifest.layers[1].directory, "_prerender");
    assert_eq!(manifest.routes[1].fallthrough, Some(true));
    assert_eq!(manifest.layers[2].name, "public-assets");
    assert_eq!(manifest.layers[2].directory, "public");
    assert_eq!(manifest.routes[2].fallthrough, Some(true));
    assert_eq!(manifest.layers[3].entry.as_deref(), Some("server.js"));
    let prerender = manifest.prerender.as_ref().expect("prerender config");
    assert_eq!(prerender.layer, "prerendered");
    assert_eq!(prerender.pages["/"].html, "index.html");
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/adapter/name"))
            .and_then(|value| value.as_str()),
        Some("@onreza/nrz-next-adapter")
    );
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/framework/version"))
            .and_then(|value| value.as_str()),
        Some("16.2.9")
    );
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/next/adapterCompatibility/outputs/staticFiles"))
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| {
                meta.pointer("/next/adapterCompatibility/platform/prerenders/staticLayerCount")
            })
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert!(
        result
            .output_dir
            .join("_static/_next/static/chunks/main.js")
            .is_file()
    );
    assert!(result.output_dir.join("_prerender/index.html").is_file());
    assert!(result.output_dir.join("public/robots.txt").is_file());
}

#[tokio::test]
async fn nextjs_adapter_manifest_meta_stays_compact_for_many_isr_routes() {
    let project = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".next/standalone")).unwrap();
    std::fs::write(
        project.path().join(".next/standalone/server.js"),
        "// server",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".onreza")).unwrap();
    let prerenders = (0..300)
        .map(|index| {
            serde_json::json!({
                "type": "PRERENDER",
                "id": format!("/blog/post-{index}"),
                "pathname": format!("/blog/post-{index}"),
                "fallback": {
                    "filePath": format!("/tmp/next/.next/server/app/blog/post-{index}.html"),
                    "initialHeaders": { "content-type": "text/html; charset=utf-8" },
                    "initialRevalidate": 60,
                    "initialExpiration": 31_536_000
                }
            })
        })
        .collect::<Vec<_>>();
    let descriptor = serde_json::json!({
        "version": 1,
        "adapter": { "name": "@onreza/nrz-next-adapter", "version": "0.34.1" },
        "nextVersion": "16.2.9",
        "buildId": "build-123",
        "outputs": {
            "prerenders": prerenders
        }
    });
    std::fs::write(
        project.path().join(".onreza/next-adapter-output.json"),
        serde_json::to_string(&descriptor).unwrap(),
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
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
        .expect("Next.js adapter descriptor should produce a manifest");
    let meta = manifest.meta.as_ref().expect("adapter metadata");
    let meta_size = serde_json::to_string(meta).unwrap().len();
    assert!(
        meta_size <= 16_384,
        "adapter metadata must stay within platform manifest limit, got {meta_size} bytes"
    );
    assert_eq!(
        meta.pointer("/next/adapterCompatibility/platform/prerenders/isrCount")
            .and_then(|value| value.as_u64()),
        Some(300)
    );
    assert!(
        meta.pointer("/next/adapterCompatibility/platform/prerenders/routes")
            .is_none()
    );
    assert!(
        meta.pointer("/next/adapterCompatibility/platform/nextCache/routes")
            .is_none()
    );
}

#[tokio::test]
async fn nextjs_adapter_descriptor_with_middleware_uses_compute_fallback() {
    let project = tempfile::tempdir().unwrap();

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
    std::fs::write(project.path().join("public/robots.txt"), "User-agent: *").unwrap();
    std::fs::create_dir_all(project.path().join(".onreza")).unwrap();
    let static_file_path = project.path().join(".next/static/chunks/main.js");
    std::fs::write(
        project.path().join(".onreza/next-adapter-output.json"),
        format!(
            r#"{{
          "version": 1,
          "adapter": {{ "name": "@onreza/nrz-next-adapter", "version": "0.34.1" }},
          "nextVersion": "16.2.9",
          "buildId": "build-123",
          "outputs": {{
            "staticFiles": [{{
              "type": "STATIC_FILE",
              "pathname": "/_next/static/chunks/main.js",
              "filePath": "{}"
            }}],
            "middleware": {{
              "type": "MIDDLEWARE",
              "pathname": "/_middleware",
              "runtime": "edge",
              "edgeRuntime": {{ "entryKey": "middleware" }}
            }}
          }}
        }}"#,
            static_file_path.display(),
        ),
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
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
        .expect("Next.js adapter descriptor should produce a manifest");
    assert_eq!(manifest.layers.len(), 1);
    assert_eq!(manifest.layers[0].name, "server");
    assert_eq!(manifest.layers[0].entry.as_deref(), Some("server.js"));
    assert_eq!(manifest.routes.len(), 1);
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/next/adapterCompatibility/platform/staticFiles/status"))
            .and_then(|value| value.as_str()),
        Some("compute_fallback")
    );
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/next/adapterCompatibility/platform/middleware/status"))
            .and_then(|value| value.as_str()),
        Some("compute_fallback_edge_runtime")
    );
    assert!(
        result
            .output_dir
            .join(".next/static/chunks/main.js")
            .is_file()
    );
    assert!(result.output_dir.join("public/robots.txt").is_file());
    assert!(
        !result
            .output_dir
            .join("_static/_next/static/chunks/main.js")
            .exists()
    );
}

#[tokio::test]
async fn nextjs_adapter_descriptor_with_disjoint_middleware_keeps_static_layers() {
    let project = tempfile::tempdir().unwrap();

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
    std::fs::write(project.path().join("public/robots.txt"), "User-agent: *").unwrap();
    std::fs::create_dir_all(project.path().join(".onreza")).unwrap();
    let static_file_path = project.path().join(".next/static/chunks/main.js");
    std::fs::write(
        project.path().join(".onreza/next-adapter-output.json"),
        format!(
            r#"{{
          "version": 1,
          "adapter": {{ "name": "@onreza/nrz-next-adapter", "version": "0.34.1" }},
          "nextVersion": "16.2.9",
          "buildId": "build-123",
          "outputs": {{
            "staticFiles": [{{
              "type": "STATIC_FILE",
              "pathname": "/_next/static/chunks/main.js",
              "filePath": "{}"
            }}],
            "middleware": {{
              "type": "MIDDLEWARE",
              "pathname": "/_middleware",
              "runtime": "edge",
              "config": {{
                "matchers": [{{
                  "source": "/private/:path*",
                  "sourceRegex": "^(?:\\\\/(_next\\\\/data\\\\/[^/]{{1,}}))?\\\\/private(?:\\\\/((?:[^\\\\/#\\\\?]+?)(?:\\\\/(?:[^\\\\/#\\\\?]+?))*))?(\\\\.json|\\\\.rsc)?[\\\\/#\\\\?]?$"
                }}]
              }},
              "edgeRuntime": {{ "entryKey": "middleware" }}
            }}
          }}
        }}"#,
            static_file_path.display(),
        ),
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
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
        .expect("Next.js adapter descriptor should produce a manifest");
    assert_eq!(manifest.layers.len(), 3);
    assert_eq!(manifest.layers[0].name, "static-assets");
    assert_eq!(manifest.layers[1].name, "public-assets");
    assert_eq!(manifest.layers[2].name, "server");
    assert_eq!(
        manifest
            .meta
            .as_ref()
            .and_then(|meta| meta.pointer("/next/adapterCompatibility/platform/staticFiles/status"))
            .and_then(|value| value.as_str()),
        Some("guarded_static_split")
    );
    assert!(
        result
            .output_dir
            .join("_static/_next/static/chunks/main.js")
            .is_file()
    );
    assert!(result.output_dir.join("public/robots.txt").is_file());
}

#[tokio::test]
async fn nextjs_detected_monorepo_standalone_keeps_bundle_root_and_nested_entry() {
    let project = tempfile::tempdir().unwrap();

    let root = project.path().join(".next/standalone");
    let app = root.join("peerpulse");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("server.js"), "// server").unwrap();
    std::fs::create_dir_all(root.join("node_modules/shared")).unwrap();
    std::fs::write(root.join("node_modules/shared/index.js"), "// shared").unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .await
    .unwrap();

    assert!(result.output_dir.ends_with(".next/standalone"));
    assert!(
        result
            .output_dir
            .join("node_modules/shared/index.js")
            .is_file()
    );
    let manifest = result
        .manifest
        .expect("nested Next.js standalone output should produce a manifest");
    assert_eq!(
        manifest.layers.last().unwrap().entry.as_deref(),
        Some("peerpulse/server.js")
    );
    assert!(
        result
            .output_dir
            .join("peerpulse/_static/_next/static/chunks/main.js")
            .is_file()
    );
}

#[tokio::test]
async fn nextjs_nested_standalone_copies_prisma_to_bundle_root() {
    let project = tempfile::tempdir().unwrap();

    let root = project.path().join(".next/standalone");
    let app = root.join("peerpulse");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::write(app.join("server.js"), "// server").unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let prisma = project.path().join("node_modules/@prisma/client-root");
    std::fs::create_dir_all(&prisma).unwrap();
    std::fs::write(prisma.join("index.js"), "// prisma").unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .await
    .unwrap();

    assert!(result.output_dir.ends_with(".next/standalone"));
    assert!(
        root.join("node_modules/@prisma/client-root/index.js")
            .is_file()
    );
    assert!(
        !app.join("node_modules/@prisma/client-root/index.js")
            .exists()
    );
}

#[tokio::test]
async fn nextjs_nested_standalone_ignores_traced_server_js_files_when_selecting_entry() {
    let project = tempfile::tempdir().unwrap();

    let root = project.path().join(".next/standalone");
    let app = root.join("apps/web");
    let traced_package = root.join("packages/api");
    std::fs::create_dir_all(app.join(".next/server")).unwrap();
    std::fs::create_dir_all(&traced_package).unwrap();
    std::fs::write(
        app.join("server.js"),
        "process.env.__NEXT_PRIVATE_STANDALONE_CONFIG = '{}'; require('next/dist/server/lib/start-server');",
    )
    .unwrap();
    std::fs::write(
        traced_package.join("server.js"),
        "// traced workspace helper",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .await
    .unwrap();

    let manifest = result
        .manifest
        .expect("nested Next.js standalone output should produce a manifest");
    assert_eq!(
        manifest.layers.last().unwrap().entry.as_deref(),
        Some("apps/web/server.js")
    );
}

#[tokio::test]
async fn nextjs_nested_standalone_prefers_app_shape_over_generated_traced_file() {
    let project = tempfile::tempdir().unwrap();

    let root = project.path().join(".next/standalone");
    let app = root.join("apps/web");
    let traced_package = root.join("packages/api");
    std::fs::create_dir_all(app.join(".next/server")).unwrap();
    std::fs::create_dir_all(&traced_package).unwrap();
    std::fs::write(app.join("server.js"), "// app server").unwrap();
    std::fs::write(
        traced_package.join("server.js"),
        "process.env.__NEXT_PRIVATE_STANDALONE_CONFIG = '{}'; require('next/dist/server/lib/start-server');",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .await
    .unwrap();

    let manifest = result
        .manifest
        .expect("nested Next.js standalone output should produce a manifest");
    assert_eq!(
        manifest.layers.last().unwrap().entry.as_deref(),
        Some("apps/web/server.js")
    );
}

#[tokio::test]
async fn nextjs_monorepo_standalone_nested_server_generates_manifest() {
    let project = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".next/standalone/peerpulse")).unwrap();
    std::fs::write(
        project.path().join(".next/standalone/peerpulse/server.js"),
        "// server",
    )
    .unwrap();
    std::fs::create_dir_all(
        project
            .path()
            .join(".next/standalone/peerpulse/node_modules/next/dist/server/typescript/rules"),
    )
    .unwrap();
    std::fs::write(
        project.path().join(
            ".next/standalone/peerpulse/node_modules/next/dist/server/typescript/rules/server.js",
        ),
        "// not the app entry",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next", BuildSettingSource::Detected)),
    )
    .await
    .unwrap();

    assert!(result.output_dir.ends_with(".next/standalone"));
    let manifest = result
        .manifest
        .expect("nested Next.js standalone output should produce a manifest");
    assert_eq!(
        manifest.layers.last().unwrap().entry.as_deref(),
        Some("peerpulse/server.js")
    );
    assert!(
        result
            .output_dir
            .join("peerpulse/_static/_next/static/chunks/main.js")
            .is_file()
    );
}

#[tokio::test]
async fn nextjs_user_standalone_output_dir_is_not_rewritten() {
    let project = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".next/standalone/peerpulse")).unwrap();
    std::fs::write(
        project.path().join(".next/standalone/peerpulse/server.js"),
        "// server",
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/static/chunks")).unwrap();
    std::fs::write(
        project.path().join(".next/static/chunks/main.js"),
        "// main",
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next/standalone", BuildSettingSource::User)),
    )
    .await
    .unwrap();

    assert_eq!(result.output_dir, project.path().join(".next/standalone"));
    let manifest = result
        .manifest
        .expect("USER standalone output should produce a manifest");
    assert_eq!(
        manifest.layers.last().unwrap().entry.as_deref(),
        Some("peerpulse/server.js")
    );
}

#[tokio::test]
async fn nextjs_user_root_run_with_hint_preserves_root_manifest() {
    let project = tempfile::tempdir().unwrap();

    std::fs::create_dir_all(project.path().join(".onreza")).unwrap();
    std::fs::write(
        project.path().join(".onreza/manifest.json"),
        r#"{
          "version": 1,
          "layers": [
            {"name": "root-static", "target": "STATIC", "directory": "."}
          ],
          "routes": [
            {"pattern": "^/.*$", "layer": "root-static"}
          ]
        }"#,
    )
    .unwrap();
    std::fs::create_dir_all(project.path().join(".next/standalone")).unwrap();
    std::fs::write(
        project.path().join(".next/standalone/server.js"),
        "// server",
    )
    .unwrap();

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: true,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".", BuildSettingSource::User)),
    )
    .await
    .unwrap();

    assert_eq!(
        result.output_dir,
        project.path().canonicalize().unwrap().join(".")
    );
    let manifest = result
        .manifest
        .expect("USER root output should preserve its explicit root manifest");
    assert_eq!(manifest.layers.len(), 1);
    assert_eq!(manifest.layers[0].name, "root-static");
}

#[tokio::test]
async fn nextjs_user_dot_next_run_with_hint_uses_standalone_artifact() {
    let project = tempfile::tempdir().unwrap();

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

    let detection = make_detection("nextjs", None);
    let config = nrz::config::ProjectConfig::default();
    let args = BuildArgs {
        dir: project.path().to_string_lossy().into_owned(),
        skip_validation: false,
    };

    let result = run_with_hint(
        args,
        true,
        &config,
        Some(&detection),
        Some(output_hint(".next", BuildSettingSource::User)),
    )
    .await
    .unwrap();

    assert_eq!(result.output_dir, project.path().join(".next/standalone"));
    let manifest = result
        .manifest
        .expect("USER .next should use Next.js standalone artifact when present");
    assert_eq!(
        manifest.layers.last().unwrap().entry.as_deref(),
        Some("server.js")
    );
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
    assert_eq!(manifest.routes.len(), 3);
    assert_eq!(manifest.routes[0].pattern, "^/_nuxt/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
    assert_eq!(manifest.routes[1].pattern, "^/.*$");
    assert_eq!(manifest.routes[1].layer, "static-assets");
    assert_eq!(manifest.routes[1].priority, Some(50));
    assert_eq!(manifest.routes[2].pattern, "^/.*$");
    assert_eq!(manifest.routes[2].layer, "server");
    assert_eq!(manifest.routes[2].priority, Some(0));
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
    assert_eq!(manifest.routes.len(), 3);
    assert_eq!(manifest.routes[0].pattern, "^/_app/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
    assert_eq!(manifest.routes[1].pattern, "^/.*$");
    assert_eq!(manifest.routes[1].layer, "static-assets");
    assert_eq!(manifest.routes[1].priority, Some(50));
    assert_eq!(manifest.routes[2].pattern, "^/.*$");
    assert_eq!(manifest.routes[2].layer, "server");
    assert_eq!(manifest.routes[2].priority, Some(0));
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
    assert_eq!(manifest.routes.len(), 3);
    assert_eq!(manifest.routes[0].pattern, "^/assets/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
    assert_eq!(manifest.routes[1].pattern, "^/.*$");
    assert_eq!(manifest.routes[1].layer, "static-assets");
    assert_eq!(manifest.routes[1].priority, Some(50));
    assert_eq!(manifest.routes[2].pattern, "^/.*$");
    assert_eq!(manifest.routes[2].layer, "server");
    assert_eq!(manifest.routes[2].priority, Some(0));
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
    assert_eq!(manifest.routes.len(), 3);
    assert_eq!(manifest.routes[0].pattern, "^/assets/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
    assert_eq!(manifest.routes[1].pattern, "^/.*$");
    assert_eq!(manifest.routes[1].layer, "static-assets");
    assert_eq!(manifest.routes[1].priority, Some(50));
    assert_eq!(manifest.routes[2].pattern, "^/.*$");
    assert_eq!(manifest.routes[2].layer, "server");
    assert_eq!(manifest.routes[2].priority, Some(0));
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
    assert_eq!(manifest.routes.len(), 3);
    assert_eq!(manifest.routes[0].pattern, "^/_astro/.*$");
    assert_eq!(manifest.routes[0].priority, Some(100));
    assert_eq!(manifest.routes[1].pattern, "^/.*$");
    assert_eq!(manifest.routes[1].layer, "static-assets");
    assert_eq!(manifest.routes[1].priority, Some(50));
    assert_eq!(manifest.routes[2].pattern, "^/.*$");
    assert_eq!(manifest.routes[2].layer, "server");
    assert_eq!(manifest.routes[2].priority, Some(0));
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
