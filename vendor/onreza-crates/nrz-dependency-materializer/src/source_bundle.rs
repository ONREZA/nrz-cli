// @generated vendored copy of platform crates/nrz-dependency-materializer/src/source_bundle.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use nrz_runtime_artifact::{
    RuntimeArtifactError, SourceDependencyMaterialization,
    VerifiedDependencyMaterializationManifest, VerifiedRuntimeArtifactGraph,
    finalize_source_bundle_runtime_graph_with_dependencies,
};
use nrz_source_bundle::{
    DependencySourceTreeError, PYTHON_314_SITE_PACKAGES_ROOT, SourceLogicalManifest,
    extract_dependency_source_trees,
};
use serde_json::Value;
use thiserror::Error;

use crate::{
    DependencyMaterializationKind, DependencyMaterializationRequest, DependencyMaterializerError,
    DependencySymlinkScope, DependencyTreeLimits, ErofsToolchain,
};

pub struct SourceBundleMaterializationPolicy {
    pub kind: DependencyMaterializationKind,
    pub compatibility: Value,
    pub tree_limits: DependencyTreeLimits,
    pub max_total_files: u64,
    pub max_total_bytes: u64,
}

pub struct SourceBundleMaterializationRequest<'a> {
    pub source_path: &'a Path,
    pub logical_manifest_sha256: &'a str,
    pub source_sha256: &'a str,
    pub source_size_bytes: u64,
    pub manifest: &'a SourceLogicalManifest,
    pub output_root: &'a Path,
    pub policy: SourceBundleMaterializationPolicy,
}

pub struct MaterializedRuntimeDependency {
    pub layer_name: String,
    pub mount_point: String,
    pub image_path: PathBuf,
    pub manifest: VerifiedDependencyMaterializationManifest,
}

pub struct MaterializedSourceBundleRuntime {
    pub dependencies: Vec<MaterializedRuntimeDependency>,
    pub graph: VerifiedRuntimeArtifactGraph,
}

pub fn materialize_source_bundle_runtime(
    toolchain: &ErofsToolchain,
    request: SourceBundleMaterializationRequest<'_>,
) -> Result<MaterializedSourceBundleRuntime, SourceBundleMaterializationError> {
    validate_runtime_family(request.manifest, request.policy.kind)?;
    fs::create_dir(request.output_root).map_err(|source| SourceBundleMaterializationError::Io {
        operation: "create runtime materialization root",
        path: request.output_root.to_path_buf(),
        source,
    })?;
    let tree_root = request.output_root.join("trees");
    let image_root = request.output_root.join("images");
    fs::create_dir(&tree_root).map_err(|source| SourceBundleMaterializationError::Io {
        operation: "create dependency tree root",
        path: tree_root.clone(),
        source,
    })?;
    let trees = extract_dependency_source_trees(request.source_path, request.manifest, &tree_root)?;
    if !trees.is_empty() {
        fs::create_dir(&image_root).map_err(|source| SourceBundleMaterializationError::Io {
            operation: "create dependency image root",
            path: image_root.clone(),
            source,
        })?;
    }

    let allowed_mount_points_by_layer = trees.iter().fold(
        BTreeMap::<String, Vec<String>>::new(),
        |mut by_layer, tree| {
            by_layer
                .entry(tree.layer_name.clone())
                .or_default()
                .push(tree.mount_point.clone());
            by_layer
        },
    );
    let mut total_files = 0_u64;
    let mut total_bytes = 0_u64;
    let mut dependencies = Vec::with_capacity(trees.len());
    for (index, tree) in trees.into_iter().enumerate() {
        if !dependency_root_matches_kind(&tree.source_root, request.policy.kind) {
            return Err(SourceBundleMaterializationError::DependencyKindMismatch {
                source_root: tree.source_root,
                kind: request.policy.kind,
            });
        }
        let allowed_mount_points = allowed_mount_points_by_layer
            .get(&tree.layer_name)
            .expect("every dependency tree has an allowed mount set");
        let image_path = image_root.join(format!("dependency-{index}.erofs"));
        let output = toolchain.materialize(DependencyMaterializationRequest {
            source_tree: &tree.path,
            output_image: &image_path,
            kind: request.policy.kind,
            compatibility: request.policy.compatibility.clone(),
            limits: request.policy.tree_limits,
            symlink_scope: DependencySymlinkScope::RuntimeMounts {
                mount_point: &tree.mount_point,
                allowed_mount_points,
            },
        })?;
        total_files = total_files
            .checked_add(output.tree.expanded_file_count)
            .ok_or(SourceBundleMaterializationError::LimitExceeded)?;
        total_bytes = total_bytes
            .checked_add(output.tree.expanded_bytes)
            .ok_or(SourceBundleMaterializationError::LimitExceeded)?;
        if total_files > request.policy.max_total_files
            || total_bytes > request.policy.max_total_bytes
        {
            return Err(SourceBundleMaterializationError::LimitExceeded);
        }
        dependencies.push(MaterializedRuntimeDependency {
            layer_name: tree.layer_name,
            mount_point: tree.mount_point,
            image_path: output.image_path,
            manifest: output.manifest,
        });
    }

    let graph_dependencies = dependencies
        .iter()
        .map(|dependency| SourceDependencyMaterialization {
            layer_name: &dependency.layer_name,
            mount_point: &dependency.mount_point,
            manifest: &dependency.manifest,
        })
        .collect::<Vec<_>>();
    let graph = finalize_source_bundle_runtime_graph_with_dependencies(
        request.logical_manifest_sha256,
        request.source_sha256,
        request.source_size_bytes,
        request.manifest,
        &graph_dependencies,
    )?;

    Ok(MaterializedSourceBundleRuntime {
        dependencies,
        graph,
    })
}

fn dependency_root_matches_kind(root: &str, kind: DependencyMaterializationKind) -> bool {
    match kind {
        DependencyMaterializationKind::JavaScriptNodeModules => root
            .split('/')
            .next_back()
            .is_some_and(|component| component == "node_modules"),
        DependencyMaterializationKind::PythonSitePackages => root == PYTHON_314_SITE_PACKAGES_ROOT,
    }
}

fn validate_runtime_family(
    manifest: &SourceLogicalManifest,
    kind: DependencyMaterializationKind,
) -> Result<(), SourceBundleMaterializationError> {
    let expected = match kind {
        DependencyMaterializationKind::JavaScriptNodeModules => "JAVASCRIPT",
        DependencyMaterializationKind::PythonSitePackages => "PYTHON",
    };
    for layer in manifest
        .layers
        .iter()
        .filter(|layer| layer.target == "COMPUTE")
    {
        let runtime_family = layer
            .runtime_config
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|config| config.get("runtimeFamily"));
        let actual = match runtime_family {
            None => "JAVASCRIPT".to_string(),
            Some(Value::String(value)) => value.clone(),
            Some(value) => value.to_string(),
        };
        if actual != expected {
            return Err(SourceBundleMaterializationError::RuntimeFamilyMismatch {
                layer_name: layer.name.clone(),
                expected,
                actual,
            });
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum SourceBundleMaterializationError {
    #[error("verified dependency trees exceed the materialization policy limits")]
    LimitExceeded,
    #[error("dependency root {source_root} is incompatible with materialization kind {kind:?}")]
    DependencyKindMismatch {
        source_root: String,
        kind: DependencyMaterializationKind,
    },
    #[error(
        "compute layer {layer_name} declares runtime family {actual}, but build policy requires {expected}"
    )]
    RuntimeFamilyMismatch {
        layer_name: String,
        expected: &'static str,
        actual: String,
    },
    #[error("runtime materialization I/O failed while attempting to {operation} at {path}", path = .path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    DependencySource(#[from] DependencySourceTreeError),
    #[error(transparent)]
    DependencyMaterializer(#[from] DependencyMaterializerError),
    #[error(transparent)]
    RuntimeArtifact(#[from] RuntimeArtifactError),
}
