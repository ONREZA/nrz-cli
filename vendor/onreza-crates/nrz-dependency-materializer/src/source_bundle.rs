// @generated vendored copy of platform crates/nrz-dependency-materializer/src/source_bundle.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::fs;
use std::path::{Path, PathBuf};

use nrz_runtime_artifact::{
    RuntimeArtifactError, SourceDependencyMaterialization,
    VerifiedDependencyMaterializationManifest, VerifiedRuntimeArtifactGraph,
    finalize_source_bundle_runtime_graph_with_dependencies,
};
use nrz_source_bundle::{
    DependencySourceTreeError, SourceLogicalManifest, extract_dependency_source_trees,
};
use serde_json::Value;
use thiserror::Error;

use crate::{
    DependencyMaterializationKind, DependencyMaterializationRequest, DependencyMaterializerError,
    DependencyTreeLimits, ErofsToolchain,
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

    let mut total_files = 0_u64;
    let mut total_bytes = 0_u64;
    let mut dependencies = Vec::with_capacity(trees.len());
    for (index, tree) in trees.into_iter().enumerate() {
        let image_path = image_root.join(format!("dependency-{index}.erofs"));
        let output = toolchain.materialize(DependencyMaterializationRequest {
            source_tree: &tree.path,
            output_image: &image_path,
            kind: request.policy.kind,
            compatibility: request.policy.compatibility.clone(),
            limits: request.policy.tree_limits,
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

#[derive(Debug, Error)]
pub enum SourceBundleMaterializationError {
    #[error("verified dependency trees exceed the materialization policy limits")]
    LimitExceeded,
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
