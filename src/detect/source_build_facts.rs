//! Authored recipe facts; malformed package build evidence is rejected only by its consumer.

use nrz_source_bundle::{ApplicationRuntimeFamily, BuildToolchainFamily};
use serde::Serialize;

use super::{fs::Fs, types::ComputeType};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBuildFacts {
    // Missing content is unknown: a remote tree need not prove a file's absence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_output_directory: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_build: Option<PackageBuildFact>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PackageBuildFact {
    Valid {
        #[serde(rename = "hasBuildScript")]
        has_build_script: bool,
    },
    Invalid,
}

pub(super) fn collect(
    fs: &dyn Fs,
    detection: &super::types::DetectionResult,
    config: &crate::config::ProjectConfig,
    config_present: bool,
) -> Option<SourceBuildFacts> {
    let context = detection.metadata.source_build_context.as_ref()?;
    if detection.suggested_compute != ComputeType::Process
        || !matches!(
            context.build_toolchain.family,
            BuildToolchainFamily::Node | BuildToolchainFamily::Bun
        )
        || context.application_runtime.as_ref().is_some_and(|runtime| {
            !matches!(
                runtime.family,
                ApplicationRuntimeFamily::Node | ApplicationRuntimeFamily::Bun
            )
        })
    {
        return None;
    }
    Some(SourceBuildFacts {
        declared_output_directory: config_present.then(|| config.build.output_directory.clone()),
        package_build: fs
            .read_file("package.json")
            .and_then(|source| package_build_fact(&source)),
    })
}

fn package_build_fact(source: &str) -> Option<PackageBuildFact> {
    // JSON outside serde_json's numeric/depth limits remains unknown for the legacy consumer.
    let package: serde_json::Value = serde_json::from_str(source).ok()?;
    Some(match package_build_script(&package) {
        Some(has_build_script) => PackageBuildFact::Valid { has_build_script },
        None => PackageBuildFact::Invalid,
    })
}

// Match the consumer's object schema, including JSON arrays; unrelated fields are not build evidence.
fn package_build_script(package: &serde_json::Value) -> Option<bool> {
    if package.is_array() {
        return Some(false);
    }
    let package = package.as_object()?;
    let Some(scripts) = package.get("scripts") else {
        return Some(false);
    };
    if scripts.is_array() {
        return Some(false);
    }
    let scripts = scripts.as_object()?;
    match scripts.get("build") {
        None => Some(false),
        Some(serde_json::Value::String(_)) => Some(true),
        Some(_) => None,
    }
}
