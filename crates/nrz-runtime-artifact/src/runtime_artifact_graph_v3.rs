//! Bounded immutable metadata proof, separate from publication and execution authority.

use std::collections::{HashMap, HashSet};

use nrz_contract::{
    application_package_v1::BlobReference,
    artifact_json_v1::{self, Limits},
    runtime_artifact_graph_v3::{Layer, RuntimeArtifactGraphV3},
    source_bundle_v2::{BlobDigest, DigestAlgorithm},
};
use nrz_source_bundle::source_bundle_v2::{
    SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES, SOURCE_BUNDLE_V2_MEDIA_TYPE, valid_runtime_policy_lineage,
};
use serde_json::json;

use crate::application_package_v1::{
    ApplicationPackagePolicy, PackageError, validate_application_package_descriptor,
    validate_blob_reference,
};

pub const RUNTIME_ARTIFACT_GRAPH_V3_MEDIA_TYPE: &str =
    "application/vnd.onreza.runtime-artifact-graph.v3+json";
pub const RUNTIME_ARTIFACT_GRAPH_V3_MAX_BYTES: usize = 32 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum GraphV3Error {
    #[error(transparent)]
    Canonical(#[from] artifact_json_v1::Error),
    #[error(transparent)]
    Shape(#[from] serde_json::Error),
    #[error(transparent)]
    Package(#[from] PackageError),
    #[error(transparent)]
    Legacy(#[from] crate::RuntimeArtifactError),
    #[error("runtime graph V3 invariant failed: {0}")]
    Invariant(String),
}

/// Consistent wire, canonical bytes and external identities, created only by verification.
/// This does not attest source/package contents, native compatibility or authorization.
/// ```compile_fail,E0616
/// use nrz_runtime_artifact::runtime_artifact_graph_v3::VerifiedRuntimeArtifactGraphV3;
/// fn mutate(graph: &mut VerifiedRuntimeArtifactGraphV3) { graph.wire.layers.clear(); }
/// ```
/// ```compile_fail,E0596
/// use nrz_runtime_artifact::runtime_artifact_graph_v3::VerifiedRuntimeArtifactGraphV3;
/// fn mutate(graph: &mut VerifiedRuntimeArtifactGraphV3) { graph.wire().layers.clear(); }
/// ```
#[derive(Debug)]
pub struct VerifiedRuntimeArtifactGraphV3 {
    wire: RuntimeArtifactGraphV3,
    canonical_bytes: Vec<u8>,
    graph_digest: BlobDigest,
    application_identity: BlobDigest,
}

impl VerifiedRuntimeArtifactGraphV3 {
    #[must_use]
    pub fn wire(&self) -> &RuntimeArtifactGraphV3 {
        &self.wire
    }
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    #[must_use]
    pub fn graph_digest(&self) -> &BlobDigest {
        &self.graph_digest
    }
    #[must_use]
    pub fn application_identity(&self) -> &BlobDigest {
        &self.application_identity
    }
}

/// Producer metadata validation. Full source relations belong to the source compiler.
pub fn verify_runtime_artifact_graph_v3(
    wire: RuntimeArtifactGraphV3,
    policy: &ApplicationPackagePolicy,
) -> Result<VerifiedRuntimeArtifactGraphV3, GraphV3Error> {
    validate(&wire, policy)?;
    let bytes = artifact_json_v1::encode(&serde_json::to_value(&wire)?, limits())?;
    finish(wire, bytes)
}

/// Verify the exact external reference before parsing. Its authorization remains the
/// caller's responsibility; a self-supplied reference is only an integrity check.
pub fn decode_runtime_artifact_graph_v3(
    bytes: &[u8],
    expected: &BlobReference,
    policy: &ApplicationPackagePolicy,
) -> Result<VerifiedRuntimeArtifactGraphV3, GraphV3Error> {
    require(
        bytes.len() <= RUNTIME_ARTIFACT_GRAPH_V3_MAX_BYTES,
        "graph byte budget exceeded",
    )?;
    validate_blob_reference(expected)?;
    require(
        expected.media_type == RUNTIME_ARTIFACT_GRAPH_V3_MEDIA_TYPE
            && expected.size == bytes.len() as u64
            && expected.digest == digest(bytes),
        "graph reference does not match exact bytes",
    )?;
    let wire = serde_json::from_value(artifact_json_v1::decode(bytes, limits())?)?;
    validate(&wire, policy)?;
    finish(wire, bytes.to_vec())
}

fn finish(
    wire: RuntimeArtifactGraphV3,
    bytes: Vec<u8>,
) -> Result<VerifiedRuntimeArtifactGraphV3, GraphV3Error> {
    let membership = artifact_json_v1::encode(&serde_json::to_value(&wire.application)?, limits())?;
    Ok(VerifiedRuntimeArtifactGraphV3 {
        graph_digest: digest(&bytes),
        application_identity: digest(&membership),
        canonical_bytes: bytes,
        wire,
    })
}

fn digest(bytes: &[u8]) -> BlobDigest {
    BlobDigest {
        algorithm: DigestAlgorithm::Blake3,
        value: blake3::hash(bytes).to_hex().to_string(),
    }
}

fn limits() -> Limits {
    Limits {
        max_bytes: RUNTIME_ARTIFACT_GRAPH_V3_MAX_BYTES,
        max_depth: artifact_json_v1::MAX_SUPPORTED_DEPTH,
    }
}

fn validate(
    wire: &RuntimeArtifactGraphV3,
    policy: &ApplicationPackagePolicy,
) -> Result<(), GraphV3Error> {
    let application = &wire.application;
    validate_blob_reference(&application.manifest)?;
    require(
        application.manifest.media_type == SOURCE_BUNDLE_V2_MEDIA_TYPE
            && (1..=SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES as u64)
                .contains(&application.manifest.size),
        "application manifest must reference bounded Source V2 bytes",
    )?;
    require(
        application.components.len() <= 2,
        "too many application components",
    )?;
    let mut blob_sizes = HashMap::new();
    require_consistent_blob_size(
        &mut blob_sizes,
        ("BLAKE3", application.manifest.digest.value.as_str()),
        application.manifest.size,
    )?;
    for component in &application.components {
        validate_application_package_descriptor(component, policy)?;
        require_consistent_blob_size(
            &mut blob_sizes,
            ("BLAKE3", component.blob.digest.value.as_str()),
            component.blob.size,
        )?;
    }
    require(
        application
            .components
            .windows(2)
            .all(|pair| pair[0].role < pair[1].role),
        "components must have unique roles in STATIC, COMPUTE order",
    )?;
    require(
        wire.dependencies.len() <= crate::MAX_DEPENDENCIES,
        "too many dependencies",
    )?;
    require(
        wire.layers.len() <= crate::MAX_RUNTIME_LAYERS,
        "too many layers",
    )?;
    let mut dependencies = HashMap::new();
    for descriptor in &wire.dependencies {
        let legacy = &descriptor.legacy;
        crate::verify_runtime_dependency(legacy)?;
        portable_mount(legacy.mount_point.as_str())?;
        require(
            dependencies
                .insert(legacy.materialization_id.as_str(), legacy)
                .is_none(),
            "duplicate dependency identity",
        )?;
        if let Some(blob) = &descriptor.physical_blob {
            validate_blob_reference(blob)?;
            require(
                blob.media_type == legacy.blob_descriptor.media_type
                    && blob.size == legacy.blob_descriptor.size as u64,
                "new physical dependency must retain media type and size",
            )?;
            require_consistent_blob_size(
                &mut blob_sizes,
                ("BLAKE3", blob.digest.value.as_str()),
                blob.size,
            )?;
        } else {
            require_consistent_blob_size(
                &mut blob_sizes,
                ("SHA256", legacy.blob_descriptor.digest.as_str()),
                legacy.blob_descriptor.size as u64,
            )?;
        }
    }
    let mut names = HashSet::new();
    let mut referenced = HashSet::new();
    let mut representative_mounts = HashSet::new();
    for layer in &wire.layers {
        let (name, root) = match layer {
            Layer::Static { name, root_path }
            | Layer::Compute {
                name, root_path, ..
            } => (name, root_path),
        };
        require(
            !name.0.is_empty()
                && name.0.chars().count() <= 64
                && !name.0.chars().any(char::is_control)
                && names.insert(&name.0),
            "invalid or duplicate layer name",
        )?;
        if let Some(root) = root {
            portable_path(&root.0)?;
        }
        let Layer::Compute {
            entrypoint,
            launch,
            network,
            limits,
            build_runtime_target,
            runtime_policy_lineage,
            dependency_mounts,
            ..
        } = layer
        else {
            continue;
        };
        portable_path(&entrypoint.0)?;
        require(
            root.as_ref().is_none_or(|root| {
                entrypoint
                    .0
                    .strip_prefix(&root.0)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            }),
            "entrypoint must be a child of application root",
        )?;
        require(
            limits
                .timeout_ms
                .is_none_or(|ms| (1..=crate::MAX_SAFE_INTEGER).contains(&ms))
                && limits.memory_mb.is_none_or(|mb| (32..=8192).contains(&mb)),
            "invalid requested runtime limits",
        )?;
        require(
            runtime_policy_lineage
                .as_deref()
                .is_none_or(valid_runtime_policy_lineage),
            "invalid runtime policy lineage",
        )?;
        require(
            !build_runtime_target.is_empty() && build_runtime_target.chars().count() <= 128,
            "invalid build target",
        )?;
        // Reuse the source launch/target and network domains without an inventory clone.
        let family = match launch.profile {
            crate::RuntimeProfile::Bun => "BUN",
            crate::RuntimeProfile::Node22
            | crate::RuntimeProfile::Node24
            | crate::RuntimeProfile::Node26 => "NODE",
            crate::RuntimeProfile::Cpython312
            | crate::RuntimeProfile::Cpython313
            | crate::RuntimeProfile::Cpython314 => "PYTHON",
            crate::RuntimeProfile::Executable => "EXECUTABLE",
        };
        let config = json!({"applicationRuntime": {"family": family, "args": launch.args}, "network": network});
        let compiled =
            crate::source_layer_launch_for_target(Some(&config), Some(build_runtime_target))?;
        require(
            compiled.profile == launch.profile,
            "launch profile conflicts with build target",
        )?;
        nrz_source_bundle::layer_compute_network(Some(&config)).map_err(GraphV3Error::Invariant)?;
        require(
            dependency_mounts.len() <= crate::MAX_LAYER_DEPENDENCIES,
            "too many dependency mounts",
        )?;
        for (index, mount) in dependency_mounts.iter().enumerate() {
            portable_mount(mount.mount_point.as_str())?;
            require(
                !dependency_mounts[..index].iter().any(|other| {
                    crate::paths_overlap(other.mount_point.as_str(), mount.mount_point.as_str())
                }),
                "dependency mounts overlap",
            )?;
            let id = mount.materialization_id.as_str();
            let descriptor = dependencies.get(id).ok_or_else(|| {
                GraphV3Error::Invariant("mount references an unknown dependency".into())
            })?;
            crate::verify_dependency_build_profile(descriptor, launch.profile)?;
            referenced.insert(id);
            if descriptor.mount_point.as_str() == mount.mount_point.as_str() {
                representative_mounts.insert(id);
            }
        }
    }
    require(
        referenced.len() == dependencies.len(),
        "unreferenced dependency",
    )?;
    require(
        representative_mounts.len() == dependencies.len(),
        "dependency representative mount is absent from bindings",
    )
}

fn require_consistent_blob_size<'a>(
    sizes: &mut HashMap<(&'static str, &'a str), u64>,
    identity: (&'static str, &'a str),
    size: u64,
) -> Result<(), GraphV3Error> {
    require(
        sizes
            .insert(identity, size)
            .is_none_or(|previous| previous == size),
        "physical blob identity has conflicting sizes",
    )
}

fn portable_path(path: &str) -> Result<(), GraphV3Error> {
    nrz_source_bundle::normalize_source_path(path).map_err(GraphV3Error::Invariant)?;
    Ok(())
}

fn portable_mount(path: &str) -> Result<(), GraphV3Error> {
    crate::verify_mount_point(path)?;
    portable_path(&path["/output/".len()..])
}

fn require(condition: bool, message: &str) -> Result<(), GraphV3Error> {
    if condition {
        Ok(())
    } else {
        Err(GraphV3Error::Invariant(message.into()))
    }
}

#[cfg(test)]
#[path = "runtime_artifact_graph_v3_tests.rs"]
mod tests;
