//! Publication-time metadata relations between verified source, packages and graph.
//! This does not verify file/package bytes, native compatibility or durable publication.

use std::collections::{BTreeSet, HashMap, HashSet};

use nrz_contract::{
    application_package_v1::{BlobReference, Role},
    runtime_artifact_graph_v3 as graph, source_bundle_v2 as source,
};
use nrz_source_bundle::{
    SourceLogicalManifestLayer, source_bundle_v2::VerifiedSourceBundleV2Manifest,
};
use serde_json::json;

use crate::{
    VerifiedDependencyMaterializationManifest,
    application_package_v1::{ApplicationPackagePolicy, VerifiedApplicationPackageDescriptor},
    compile_source_runtime_layer_for_target,
    runtime_artifact_graph_v3::{
        GraphV3Error, VerifiedRuntimeArtifactGraphV3, verify_runtime_artifact_graph_v3,
    },
};

/// Independently admitted publication target; source build witnesses cannot supply it.
pub struct LayerBuildTarget<'a> {
    pub layer_name: &'a str,
    pub build_runtime_target: &'a str,
}

/// Physical BLAKE3 supplements the retained materialization's SHA-256 proof.
/// `None` explicitly retains the legacy physical descriptor.
pub struct SourceDependencyMaterializationV3<'a> {
    pub layer_name: &'a str,
    pub mount_point: &'a str,
    pub manifest: &'a VerifiedDependencyMaterializationManifest,
    pub physical_blob: Option<&'a BlobReference>,
}

/// Compile bounded graph metadata without cloning the source inventory or routes.
/// Package role presence and file/mount ownership are checked here; package contents,
/// native entry compatibility and authenticated publication bindings remain external.
pub fn compile_source_bundle_v2_runtime_graph(
    source: &VerifiedSourceBundleV2Manifest,
    components: &[&VerifiedApplicationPackageDescriptor],
    targets: &[LayerBuildTarget<'_>],
    dependencies: &[SourceDependencyMaterializationV3<'_>],
    policy: &ApplicationPackagePolicy,
) -> Result<VerifiedRuntimeArtifactGraphV3, GraphV3Error> {
    if components.len() > 2
        || targets.len() > crate::MAX_RUNTIME_LAYERS
        || dependencies.len() > crate::MAX_RUNTIME_LAYERS * crate::MAX_LAYER_DEPENDENCIES
    {
        return invalid("compiler input exceeds bounded graph cardinality");
    }
    let manifest = source.wire();
    let roles = manifest
        .files
        .iter()
        .filter_map(|file| match file_fields(file).1 {
            source::FileRole::Dependency => None,
            source::FileRole::Compute => Some(Role::Compute),
            source::FileRole::Static | source::FileRole::Prerender | source::FileRole::Config => {
                Some(Role::Static)
            }
        })
        .collect::<BTreeSet<_>>();
    let supplied_roles = components
        .iter()
        .map(|component| component.wire().role)
        .collect::<BTreeSet<_>>();
    if supplied_roles.len() != components.len() || supplied_roles != roles {
        return invalid("packages must match exactly the nonempty source application roles");
    }
    let mut components = components
        .iter()
        .map(|component| component.wire().clone())
        .collect::<Vec<_>>();
    components.sort_unstable_by_key(|component| component.role);

    let mut by_target = HashMap::new();
    for target in targets {
        if !manifest.layers.iter().any(|layer| {
            layer.name.0 == target.layer_name && layer.target == source::LayerTarget::Compute
        }) || by_target
            .insert(target.layer_name, target.build_runtime_target)
            .is_some()
        {
            return invalid("build target names an unknown/noncompute or duplicate layer");
        }
    }
    verify_file_mount_ownership(manifest, dependencies)?;

    let mut ordered_dependencies = dependencies.iter().collect::<Vec<_>>();
    ordered_dependencies.sort_unstable_by_key(|dependency| {
        (
            dependency.manifest.materialization_id(),
            dependency.mount_point,
            dependency.layer_name,
        )
    });
    let mut descriptors: Vec<graph::DependencyDescriptor> = Vec::new();
    let mut by_identity = HashMap::new();
    for dependency in &ordered_dependencies {
        let descriptor = dependency_descriptor(dependency)?;
        let id = dependency.manifest.materialization_id();
        if let Some(&index) = by_identity.get(id) {
            let existing: &graph::DependencyDescriptor = &descriptors[index];
            let mut legacy = existing.legacy.clone();
            legacy.mount_point = descriptor.legacy.mount_point.clone();
            if legacy != descriptor.legacy || existing.physical_blob != descriptor.physical_blob {
                return invalid("shared dependency materialization has conflicting descriptors");
            }
        } else {
            by_identity.insert(id, descriptors.len());
            descriptors.push(descriptor);
        }
    }
    let mut layers = Vec::with_capacity(manifest.layers.len());
    for layer in &manifest.layers {
        match layer.target {
            source::LayerTarget::Static => layers.push(graph::Layer::Static {
                name: layer.name.clone(),
                root_path: layer.root_path.clone(),
            }),
            source::LayerTarget::Compute => {
                let target = by_target.get(layer.name.0.as_str()).ok_or_else(|| {
                    GraphV3Error::Invariant(format!(
                        "compute layer '{}' has no independently admitted build target",
                        layer.name.0
                    ))
                })?;
                let bounded: SourceLogicalManifestLayer =
                    serde_json::from_value(serde_json::to_value(layer)?)?;
                let compiled =
                    compile_source_runtime_layer_for_target(&bounded, &[], Some(target))?;
                let config = layer
                    .runtime_config
                    .as_ref()
                    .expect("verified compute config");
                let legacy_launch: crate::RuntimeLaunchWire =
                    serde_json::from_value(compiled["launch"].clone())?;
                let mut mounts: Vec<graph::DependencyMount> = ordered_dependencies
                    .iter()
                    .filter(|dependency| dependency.layer_name == layer.name.0)
                    .map(|dependency| {
                        serde_json::from_value(json!({
                            "materializationId": dependency.manifest.materialization_id(),
                            "mountPoint": dependency.mount_point,
                        }))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                mounts.sort_unstable_by(|left, right| {
                    (left.mount_point.as_str(), left.materialization_id.as_str()).cmp(&(
                        right.mount_point.as_str(),
                        right.materialization_id.as_str(),
                    ))
                });
                layers.push(graph::Layer::Compute {
                    name: layer.name.clone(),
                    root_path: layer.root_path.clone(),
                    entrypoint: layer
                        .entrypoint
                        .clone()
                        .expect("verified compute entrypoint"),
                    launch: graph::Launch {
                        profile: legacy_launch.profile,
                        args: legacy_launch
                            .args
                            .iter()
                            .map(|argument| argument.as_str().to_owned())
                            .collect(),
                    },
                    network: config.network.clone().expect("verified compute network"),
                    limits: graph::RuntimeLimits {
                        timeout_ms: config.timeout_ms,
                        memory_mb: config.memory_mb,
                    },
                    build_runtime_target: (*target).to_owned(),
                    runtime_policy_lineage: config.runtime_policy_lineage.clone(),
                    dependency_mounts: mounts,
                });
            }
        }
    }
    verify_runtime_artifact_graph_v3(
        graph::RuntimeArtifactGraphV3 {
            schema_version: graph::SchemaVersion::V3,
            application: graph::ApplicationMembership {
                schema_version: graph::MembershipSchemaVersion::V1,
                manifest: BlobReference {
                    media_type: nrz_source_bundle::source_bundle_v2::SOURCE_BUNDLE_V2_MEDIA_TYPE
                        .into(),
                    digest: source::BlobDigest {
                        algorithm: source::DigestAlgorithm::Blake3,
                        value: source.manifest_blake3().into(),
                    },
                    size: source.manifest_size_bytes(),
                },
                components,
            },
            dependencies: descriptors,
            layers,
        },
        policy,
    )
}

fn verify_file_mount_ownership(
    manifest: &nrz_contract::SourceBundleV2ManifestWire,
    dependencies: &[SourceDependencyMaterializationV3<'_>],
) -> Result<(), GraphV3Error> {
    let mut covered = HashSet::new();
    for dependency in dependencies {
        crate::verify_mount_point(dependency.mount_point)?;
        if !manifest.layers.iter().any(|layer| {
            layer.target == source::LayerTarget::Compute && layer.name.0 == dependency.layer_name
        }) {
            return invalid("dependency references an unknown compute layer");
        }
    }
    for file in &manifest.files {
        let (path, role, owner) = file_fields(file);
        let mut file_covered = false;
        for (index, dependency) in dependencies.iter().enumerate() {
            let mount = &dependency.mount_point["/output/".len()..];
            if role == &source::FileRole::Dependency {
                if owner == Some(dependency.layer_name)
                    && path
                        .strip_prefix(mount)
                        .is_some_and(|suffix| suffix.starts_with('/'))
                {
                    covered.insert(index);
                    file_covered = true;
                }
            } else if crate::paths_overlap(path, mount) {
                return invalid("application path collides with a dependency mount");
            }
        }
        if role == &source::FileRole::Dependency && !file_covered {
            return invalid("dependency file has no owned mount");
        }
    }
    if dependencies.iter().enumerate().any(|(index, dependency)| {
        !covered.contains(&index) && dependency.manifest.wire().expanded_file_count != 0
    }) {
        return invalid("dependency mount has no dependency-owned source files");
    }
    Ok(())
}

fn dependency_descriptor(
    dependency: &SourceDependencyMaterializationV3<'_>,
) -> Result<graph::DependencyDescriptor, GraphV3Error> {
    let manifest = dependency.manifest.wire();
    Ok(graph::DependencyDescriptor {
        legacy: serde_json::from_value(json!({
            "materializationId": dependency.manifest.materialization_id(),
            "kind": manifest.kind,
            "mountPoint": dependency.mount_point,
            "compatibility": manifest.compatibility,
            "manifestDigest": dependency.manifest.manifest_digest(),
            "blobDescriptor": manifest.blob_descriptor,
        }))?,
        physical_blob: dependency.physical_blob.cloned(),
    })
}

fn file_fields(file: &source::FileEntry) -> (&str, &source::FileRole, Option<&str>) {
    let (source::FileEntry::File {
        path,
        role,
        layer_name,
        ..
    }
    | source::FileEntry::Symlink {
        path,
        role,
        layer_name,
        ..
    }) = file;
    (
        &path.0,
        role,
        layer_name.as_ref().map(|name| name.0.as_str()),
    )
}

fn invalid<T>(message: impl Into<String>) -> Result<T, GraphV3Error> {
    Err(GraphV3Error::Invariant(message.into()))
}

#[cfg(test)]
#[path = "source_graph_v3_tests.rs"]
mod tests;
