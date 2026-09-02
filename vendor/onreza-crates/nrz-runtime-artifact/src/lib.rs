// @generated vendored copy of platform crates/nrz-runtime-artifact/src/lib.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::collections::{HashMap, HashSet};

use nrz_contract::{DependencyMaterializationManifestV1Wire, RuntimeArtifactGraphV2Wire};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

mod source_graph;

pub use source_graph::{
    SourceDependencyMaterialization, compute_logical_artifact_id,
    compute_source_logical_artifact_id, finalize_source_bundle_runtime_graph,
    finalize_source_bundle_runtime_graph_with_dependencies,
};

pub const RUNTIME_ARTIFACT_GRAPH_V2_SCHEMA_VERSION: &str = "RUNTIME_ARTIFACT_GRAPH_V2.0";
pub const DEPENDENCY_MATERIALIZATION_V1_SCHEMA_VERSION: &str = "DEPENDENCY_MATERIALIZATION_V1.0";
pub const DEPENDENCY_EROFS_MEDIA_TYPE: &str = "application/vnd.onreza.dependency.erofs.v1";
pub const SOURCE_BUNDLE_MEDIA_TYPE: &str = "application/vnd.onreza.source-bundle.tar+zstd.v1";

const MAX_DEPENDENCIES: usize = 16;
const MAX_RUNTIME_LAYERS: usize = 10;
const MAX_LAYER_DEPENDENCIES: usize = 8;
const MAX_POLICY_GENERATION: u64 = 2_147_483_647;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Error)]
pub enum RuntimeArtifactError {
    #[error("runtime artifact does not match the generated contract: {0}")]
    Contract(#[from] serde_json::Error),
    #[error("runtime artifact invariant failed: {0}")]
    Invariant(String),
}

#[derive(Debug, Clone)]
pub struct VerifiedRuntimeArtifactGraph {
    wire: RuntimeArtifactGraphV2Wire,
}

impl VerifiedRuntimeArtifactGraph {
    #[must_use]
    pub fn wire(&self) -> &RuntimeArtifactGraphV2Wire {
        &self.wire
    }

    #[must_use]
    pub fn into_wire(self) -> RuntimeArtifactGraphV2Wire {
        self.wire
    }

    #[must_use]
    pub fn graph_digest(&self) -> &str {
        self.wire.graph_digest.as_str()
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedDependencyMaterializationManifest {
    wire: DependencyMaterializationManifestV1Wire,
    materialization_id: String,
    manifest_digest: String,
}

impl VerifiedDependencyMaterializationManifest {
    #[must_use]
    pub fn wire(&self) -> &DependencyMaterializationManifestV1Wire {
        &self.wire
    }

    #[must_use]
    pub fn materialization_id(&self) -> &str {
        &self.materialization_id
    }

    #[must_use]
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    #[must_use]
    pub fn into_wire(self) -> DependencyMaterializationManifestV1Wire {
        self.wire
    }
}

pub fn verify_dependency_materialization_manifest(
    value: Value,
) -> Result<VerifiedDependencyMaterializationManifest, RuntimeArtifactError> {
    let wire: DependencyMaterializationManifestV1Wire = serde_json::from_value(value)?;
    let normalized_value = serde_json::to_value(&wire)?;

    require_equal(
        "schemaVersion",
        &wire.schema_version,
        DEPENDENCY_MATERIALIZATION_V1_SCHEMA_VERSION,
    )?;
    require_equal(
        "blobDescriptor.mediaType",
        &wire.blob_descriptor.media_type,
        DEPENDENCY_EROFS_MEDIA_TYPE,
    )?;
    require_safe_size("blobDescriptor.size", wire.blob_descriptor.size)?;
    require_policy_generation(
        "compatibility.buildPolicyGeneration",
        wire.compatibility.build_policy_generation.get(),
    )?;
    require_equal("compatibility.os", &wire.compatibility.os, "linux")?;

    for (name, count) in [
        ("expandedFileCount", wire.expanded_file_count),
        ("regularFileCount", wire.regular_file_count),
        ("symlinkCount", wire.symlink_count),
        ("nativeObjectCount", wire.native_object_count),
    ] {
        require_bounded_count(name, count)?;
    }
    require_safe_size("expandedBytes", wire.expanded_bytes)?;

    if wire.regular_file_count + wire.symlink_count != wire.expanded_file_count {
        return invariant("expandedFileCount must equal regularFileCount + symlinkCount");
    }
    if wire.native_object_count > wire.regular_file_count {
        return invariant("nativeObjectCount cannot exceed regularFileCount");
    }

    let object = normalized_value
        .as_object()
        .ok_or_else(|| RuntimeArtifactError::Invariant("manifest must be an object".into()))?;
    let identity = json!({
        "schemaVersion": required_value(object, "schemaVersion")?,
        "kind": required_value(object, "kind")?,
        "compatibility": required_value(object, "compatibility")?,
        "logicalTreeDigest": required_value(object, "logicalTreeDigest")?,
        "erofsBlobDigest": required_object_value(object, "blobDescriptor", "digest")?,
    });

    Ok(VerifiedDependencyMaterializationManifest {
        wire,
        materialization_id: hash_canonical_json(&identity),
        manifest_digest: hash_canonical_json(&normalized_value),
    })
}

pub fn verify_runtime_artifact_graph(
    value: Value,
    application_paths: &[String],
) -> Result<VerifiedRuntimeArtifactGraph, RuntimeArtifactError> {
    let wire: RuntimeArtifactGraphV2Wire = serde_json::from_value(value)?;
    let mut normalized_value = serde_json::to_value(&wire)?;

    require_equal(
        "schemaVersion",
        &wire.schema_version,
        RUNTIME_ARTIFACT_GRAPH_V2_SCHEMA_VERSION,
    )?;
    require_equal(
        "application.blobDescriptor.mediaType",
        &wire.application.blob_descriptor.media_type,
        SOURCE_BUNDLE_MEDIA_TYPE,
    )?;
    require_safe_size(
        "application.blobDescriptor.size",
        wire.application.blob_descriptor.size,
    )?;
    if wire.dependencies.len() > MAX_DEPENDENCIES {
        return invariant(format!("dependencies exceeds {MAX_DEPENDENCIES} entries"));
    }
    if wire.runtime_layers.len() > MAX_RUNTIME_LAYERS {
        return invariant(format!(
            "runtimeLayers exceeds {MAX_RUNTIME_LAYERS} entries"
        ));
    }

    let mut dependencies = HashMap::new();
    for dependency in &wire.dependencies {
        let materialization_id = dependency.materialization_id.as_str();
        if dependencies
            .insert(materialization_id, dependency.mount_point.as_str())
            .is_some()
        {
            return invariant(format!(
                "duplicate dependency materialization '{materialization_id}'"
            ));
        }
        verify_mount_point(dependency.mount_point.as_str())?;
        require_equal(
            "dependency.blobDescriptor.mediaType",
            &dependency.blob_descriptor.media_type,
            DEPENDENCY_EROFS_MEDIA_TYPE,
        )?;
        require_safe_size(
            "dependency.blobDescriptor.size",
            dependency.blob_descriptor.size,
        )?;
        require_equal(
            "dependency.compatibility.os",
            &dependency.compatibility.os,
            "linux",
        )?;
        require_policy_generation(
            "dependency.compatibility.buildPolicyGeneration",
            dependency.compatibility.build_policy_generation.get(),
        )?;
    }

    let mounts = dependencies.values().copied().collect::<Vec<_>>();
    for (index, left) in mounts.iter().enumerate() {
        for right in mounts.iter().skip(index + 1) {
            if paths_overlap(left, right) {
                return invariant(format!(
                    "dependency mount points overlap: '{left}' and '{right}'"
                ));
            }
        }
    }

    let mut layer_names = HashSet::new();
    let mut referenced_dependencies = HashSet::new();
    for layer in &wire.runtime_layers {
        let layer_name = layer.layer_name.as_str();
        if !layer_names.insert(layer_name) {
            return invariant(format!("duplicate runtime layer '{layer_name}'"));
        }
        verify_safe_relative_path(
            "runtime layer applicationRoot",
            layer.application_root.as_str(),
        )?;
        verify_safe_relative_path("runtime layer entrypoint", layer.entrypoint.as_str())?;
        if layer.dependency_materialization_ids.len() > MAX_LAYER_DEPENDENCIES {
            return invariant(format!(
                "runtime layer '{layer_name}' exceeds {MAX_LAYER_DEPENDENCIES} dependencies"
            ));
        }
        if let Some(memory_mb) = layer.runtime_config.memory_mb
            && !(32..=8192).contains(&memory_mb)
        {
            return invariant(format!(
                "runtime layer '{layer_name}' memoryMb must be between 32 and 8192"
            ));
        }
        if let Some(timeout_ms) = layer.runtime_config.timeout_ms
            && timeout_ms.get() > MAX_SAFE_INTEGER
        {
            return invariant(format!(
                "runtime layer '{layer_name}' timeoutMs exceeds the safe integer range"
            ));
        }
        if let Some(max_concurrency) = layer.runtime_config.max_concurrency
            && max_concurrency.get() > 100
        {
            return invariant(format!(
                "runtime layer '{layer_name}' maxConcurrency exceeds 100"
            ));
        }

        let mut layer_dependencies = HashSet::new();
        for materialization_id in &layer.dependency_materialization_ids {
            let materialization_id = materialization_id.as_str();
            if !dependencies.contains_key(materialization_id) {
                return invariant(format!(
                    "runtime layer '{layer_name}' references unknown dependency '{materialization_id}'"
                ));
            }
            if !layer_dependencies.insert(materialization_id) {
                return invariant(format!(
                    "runtime layer '{layer_name}' references dependency '{materialization_id}' more than once"
                ));
            }
            referenced_dependencies.insert(materialization_id);
        }
    }

    for materialization_id in dependencies.keys() {
        if !referenced_dependencies.contains(materialization_id) {
            return invariant(format!(
                "dependency '{materialization_id}' is not referenced by a runtime layer"
            ));
        }
    }

    verify_application_ownership(&dependencies, application_paths)?;

    let digest_object = normalized_value
        .as_object_mut()
        .ok_or_else(|| RuntimeArtifactError::Invariant("graph must be an object".into()))?;
    digest_object.remove("graphDigest");
    let expected_digest = hash_canonical_json(&normalized_value);
    if wire.graph_digest.as_str() != expected_digest {
        return invariant(format!(
            "graphDigest does not match canonical graph: expected '{expected_digest}'"
        ));
    }

    Ok(VerifiedRuntimeArtifactGraph { wire })
}

pub fn finalize_runtime_artifact_graph(
    mut value: Value,
    application_paths: &[String],
) -> Result<VerifiedRuntimeArtifactGraph, RuntimeArtifactError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| RuntimeArtifactError::Invariant("graph must be an object".into()))?;
    object.remove("graphDigest");
    let graph_digest = hash_canonical_json(&value);
    value
        .as_object_mut()
        .expect("runtime artifact graph object was checked above")
        .insert("graphDigest".to_string(), Value::String(graph_digest));
    verify_runtime_artifact_graph(value, application_paths)
}

fn verify_application_ownership(
    dependencies: &HashMap<&str, &str>,
    application_paths: &[String],
) -> Result<(), RuntimeArtifactError> {
    for path in application_paths {
        verify_safe_relative_path("application path", path)?;
        let normalized = if path == "." { "" } else { path.as_str() };
        for mount_point in dependencies.values() {
            let mount_path = &mount_point["/output/".len()..];
            if normalized == mount_path
                || normalized
                    .strip_prefix(mount_path)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            {
                return invariant(format!(
                    "application path '{path}' collides with dependency mount '{mount_point}'"
                ));
            }
        }
    }
    Ok(())
}

fn verify_safe_relative_path(label: &str, path: &str) -> Result<(), RuntimeArtifactError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\0')
        || (path != "." && path.ends_with('/'))
        || (path != "."
            && path
                .split('/')
                .any(|segment| segment.is_empty() || segment == "." || segment == ".."))
    {
        return invariant(format!(
            "{label} is not a canonical safe relative path: '{path}'"
        ));
    }
    Ok(())
}

fn verify_mount_point(path: &str) -> Result<(), RuntimeArtifactError> {
    if !path.starts_with("/output/")
        || path.ends_with('/')
        || path.contains('\0')
        || path
            .split('/')
            .skip(1)
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return invariant(format!(
            "dependency mount point is not a canonical child of /output: '{path}'"
        ));
    }
    Ok(())
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn require_equal(label: &str, actual: &str, expected: &str) -> Result<(), RuntimeArtifactError> {
    if actual != expected {
        return invariant(format!("{label} must be '{expected}', got '{actual}'"));
    }
    Ok(())
}

fn require_safe_size(label: &str, value: i64) -> Result<(), RuntimeArtifactError> {
    if !(0..=MAX_SAFE_INTEGER as i64).contains(&value) {
        return invariant(format!("{label} must be a non-negative safe integer"));
    }
    Ok(())
}

fn require_bounded_count(label: &str, value: i64) -> Result<(), RuntimeArtifactError> {
    if !(0..=MAX_POLICY_GENERATION as i64).contains(&value) {
        return invariant(format!(
            "{label} must be between 0 and {MAX_POLICY_GENERATION}"
        ));
    }
    Ok(())
}

fn require_policy_generation(label: &str, value: u64) -> Result<(), RuntimeArtifactError> {
    if value > MAX_POLICY_GENERATION {
        return invariant(format!("{label} exceeds {MAX_POLICY_GENERATION}"));
    }
    Ok(())
}

fn required_value(object: &Map<String, Value>, key: &str) -> Result<Value, RuntimeArtifactError> {
    object
        .get(key)
        .cloned()
        .ok_or_else(|| RuntimeArtifactError::Invariant(format!("missing '{key}'")))
}

fn required_object_value(
    object: &Map<String, Value>,
    object_key: &str,
    value_key: &str,
) -> Result<Value, RuntimeArtifactError> {
    object
        .get(object_key)
        .and_then(Value::as_object)
        .and_then(|nested| nested.get(value_key))
        .cloned()
        .ok_or_else(|| {
            RuntimeArtifactError::Invariant(format!("missing '{object_key}.{value_key}'"))
        })
}

fn hash_canonical_json(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(stable_stringify(value));
    hex::encode(hasher.finalize())
}

fn stable_stringify(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value.to_string(),
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(stable_stringify)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Object(entries) => {
            let mut keys = entries.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let fields = keys
                .into_iter()
                .map(|key| {
                    let encoded_key = serde_json::to_string(key)
                        .expect("serializing a JSON object key cannot fail");
                    format!("{encoded_key}:{}", stable_stringify(&entries[key]))
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
    }
}

fn invariant<T>(message: impl Into<String>) -> Result<T, RuntimeArtifactError> {
    Err(RuntimeArtifactError::Invariant(message.into()))
}
