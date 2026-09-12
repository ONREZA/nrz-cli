use std::fs;
use std::path::Path;

use axum::http::StatusCode;
use tempfile::tempdir;

use crate::artifact::source_bundle_v1;
use crate::build;
use crate::cli::BuildArgs;
use crate::frameworks::{
    clear_before_build as clear_nextjs_descriptor_before_build, is_nextjs_project,
    is_sveltekit_with_adapter_auto,
};

use super::hash::sha256_hex;
use super::*;

fn fe(path: &str, size: u64, content_hash: &str) -> FileEntry {
    FileEntry {
        path: path.into(),
        size,
        content_hash: content_hash.into(),
        kind: crate::artifact::ArtifactFileKind::File,
        symlink_resolved_path: None,
    }
}

fn git_lfs_pointer() -> &'static str {
    "version https://git-lfs.github.com/spec/v1\n\
     oid sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n\
     size 104857600\n"
}

fn effective_config(
    project_dir: &Path,
    config: nrz::config::ProjectConfig,
) -> nrz::config::EffectiveProjectConfig {
    nrz::config::EffectiveProjectConfig::from_project_config(project_dir.to_path_buf(), config)
}

fn effective_with_server_settings(
    project_dir: &Path,
    config: nrz::config::ProjectConfig,
    settings: nrz::config::ProjectBuildSettings,
) -> nrz::config::EffectiveProjectConfig {
    let mut effective = effective_config(project_dir, config);
    effective.apply_server_settings(Some(&settings));
    effective
}

fn server_build_settings(
    command: Option<&str>,
    source: Option<nrz::config::BuildSettingSource>,
) -> nrz::config::ProjectBuildSettings {
    nrz::config::ProjectBuildSettings {
        build_command: command.map(str::to_string),
        build_command_source: source,
        ..Default::default()
    }
}

fn server_install_settings(
    command: Option<&str>,
    source: Option<nrz::config::BuildSettingSource>,
) -> nrz::config::ProjectBuildSettings {
    nrz::config::ProjectBuildSettings {
        install_command: command.map(str::to_string),
        install_command_source: source,
        ..Default::default()
    }
}

mod command_tests;
mod diagnostics_tests;
mod framework_command_tests;
mod health_output_tests;
mod process_entry_tests;
mod process_output_tests;
mod publication_tests;
mod runtime_files_tests;
mod scan_tests;
mod workspace_artifact_tests;

fn limit_exceeded_publication_error() -> nrz_source_publisher::SourcePublicationError {
    nrz_source_publisher::StructuredControlPlaneError {
        status: reqwest::StatusCode::FORBIDDEN.as_u16(),
        code: "LIMIT_EXCEEDED".into(),
        message: "Deployment exceeds maximum file count (20679 / 20000).".into(),
        retry_after: None,
        details: Some(serde_json::json!({
            "limitType": "maxDeploymentFiles",
            "current": 20679,
            "limit": 20000,
            "plan": "HOBBY"
        })),
    }
    .into()
}

fn file_breakdown() -> crate::artifact::RuntimeArtifactFileBreakdown {
    crate::artifact::RuntimeArtifactFileBreakdown {
        build_output: 1_055,
        node_modules: 19_620,
        python_site_packages: 0,
        metadata: 4,
        workspace_packages: 0,
        other: 0,
        total: 20_679,
    }
}

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

fn expect_code(err: &anyhow::Error, expected: &str) {
    let coded = err
        .chain()
        .find_map(|c| c.downcast_ref::<crate::output::CodedError>())
        .unwrap_or_else(|| panic!("expected CodedError({expected}) in chain: {err:#}"));
    assert_eq!(
        coded.code, expected,
        "wrong code in chain for error: {err:#}"
    );
}
