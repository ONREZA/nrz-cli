use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use nrz_runtime_artifact::{
    RuntimeArtifactError, SourceDependencyMaterialization,
    VerifiedDependencyMaterializationManifest, VerifiedRuntimeArtifactGraph,
    compile_source_runtime_layer_for_target,
    finalize_source_bundle_runtime_graph_for_layer_targets,
};
use nrz_source_bundle::{
    DependencySourceTreeError, PythonMinor, SourceLogicalManifest, dependency_source_tree_specs,
    extract_dependency_source_trees,
};
use serde_json::Value;
use thiserror::Error;

use crate::{
    DependencyMaterializationKind, DependencyMaterializationRequest, DependencyMaterializerError,
    DependencySymlinkScope, DependencyTreeLimits, ErofsToolchain,
};

pub struct SourceBundleMaterializationPolicy {
    /// None explicitly permits only artifacts without dependency trees.
    pub kind: Option<DependencyMaterializationKind>,
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
    let version = request
        .policy
        .compatibility
        .get("runtimeVersion")
        .and_then(Value::as_str)
        .ok_or_else(|| RuntimeArtifactError::Invariant("missing trusted runtime version".into()))?;
    let version = if request
        .policy
        .compatibility
        .get("runtimeFamily")
        .and_then(Value::as_str)
        == Some("bun")
        && !version.starts_with("bun-")
    {
        format!("bun-{version}")
    } else {
        version.to_string()
    };
    nrz_source_bundle::source_application_runtime(request.manifest)
        .map_err(RuntimeArtifactError::Invariant)?;
    if request.policy.kind.is_none()
        && let Some(file) = request
            .manifest
            .files
            .iter()
            .find(|file| file.role == "dependency")
    {
        return Err(SourceBundleMaterializationError::UnexpectedDependencies {
            source_root: file.path.clone(),
        });
    }
    let targets = freeze_layer_targets(request.manifest, request.policy.kind, &version)?;
    for layer in request
        .manifest
        .layers
        .iter()
        .filter(|layer| layer.target == "COMPUTE")
    {
        // Source semantics remain owned by the shared compiler, before any IO.
        compile_source_runtime_layer_for_target(
            layer,
            &[],
            targets.get(&layer.name).map(String::as_str),
        )?;
    }
    for tree in dependency_source_tree_specs(request.manifest)? {
        let kind = request.policy.kind.ok_or_else(|| {
            SourceBundleMaterializationError::UnexpectedDependencies {
                source_root: tree.source_root.clone(),
            }
        })?;
        if !dependency_root_matches_kind(&tree.source_root, kind) {
            return Err(SourceBundleMaterializationError::DependencyKindMismatch {
                source_root: tree.source_root,
                kind,
            });
        }
    }
    // A single dependency policy cannot attest a second interpreter's tree.
    // Code-only siblings retain their independent frozen launch target.
    if request.policy.kind.is_some()
        && request.manifest.files.iter().any(|file| {
            file.role == "dependency"
                && file
                    .layer_name
                    .as_ref()
                    .and_then(|name| targets.get(name))
                    .is_some_and(|target| target != &version)
        })
    {
        return Err(RuntimeArtifactError::Invariant(
            "sibling dependencies require their own frozen build policy".into(),
        )
        .into());
    }
    if let Some(minor) = PythonMinor::from_target(&version)
        && request.policy.kind == Some(DependencyMaterializationKind::PythonSitePackages)
        && request.manifest.files.iter().any(|file| {
            file.role == "dependency" && PythonMinor::for_dependency_path(&file.path) != Some(minor)
        })
    {
        return Err(RuntimeArtifactError::Invariant(
            "Python dependency root differs from frozen build minor".into(),
        )
        .into());
    }
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
        let kind = request.policy.kind.ok_or_else(|| {
            SourceBundleMaterializationError::UnexpectedDependencies {
                source_root: tree.source_root.clone(),
            }
        })?;
        let allowed_mount_points = allowed_mount_points_by_layer
            .get(&tree.layer_name)
            .expect("every dependency tree has an allowed mount set");
        let image_path = image_root.join(format!("dependency-{index}.erofs"));
        let output = toolchain.materialize(DependencyMaterializationRequest {
            source_tree: &tree.path,
            output_image: &image_path,
            kind,
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
    let graph = finalize_source_bundle_runtime_graph_for_layer_targets(
        request.logical_manifest_sha256,
        request.source_sha256,
        request.source_size_bytes,
        request.manifest,
        &graph_dependencies,
        &targets,
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
        DependencyMaterializationKind::PythonSitePackages => PythonMinor::ALL
            .into_iter()
            .any(|minor| root == minor.site_packages_root()),
    }
}

fn freeze_layer_targets(
    manifest: &SourceLogicalManifest,
    kind: Option<DependencyMaterializationKind>,
    primary_target: &str,
) -> Result<HashMap<String, String>, SourceBundleMaterializationError> {
    let expected = match kind {
        Some(DependencyMaterializationKind::JavaScriptNodeModules) => "JAVASCRIPT",
        Some(DependencyMaterializationKind::PythonSitePackages) => "PYTHON",
        None => "NATIVE",
    };
    let mut targets = HashMap::new();
    let mut siblings = Vec::new();
    for layer in manifest
        .layers
        .iter()
        .filter(|layer| layer.target == "COMPUTE")
    {
        let intent = nrz_source_bundle::layer_application_runtime(layer.runtime_config.as_ref())
            .map_err(RuntimeArtifactError::Invariant)?;
        if intent.as_ref().is_some_and(|intent| {
            intent.family == nrz_source_bundle::ApplicationRuntimeFamily::Executable
        }) || layer
            .runtime_config
            .as_ref()
            .and_then(|c| c.get("isBinaryEntry"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            if kind.is_none() {
                if primary_target != nrz_runtime_artifact::NATIVE_EXECUTION_TARGET {
                    return Err(RuntimeArtifactError::Invariant(
                        "native layer requires the qualified build target".into(),
                    )
                    .into());
                }
                // DEPRECATED: isBinaryEntry-only layers retain targetless
                // executable launches and are excluded from the managed map.
                if intent.is_some() {
                    targets.insert(layer.name.clone(), primary_target.to_owned());
                }
            } else if intent.is_some() {
                let target = layer
                    .runtime_config
                    .as_ref()
                    .and_then(|config| config.get("buildRuntimeVersion"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        RuntimeArtifactError::Invariant(
                            "native sibling requires a frozen build target".into(),
                        )
                    })?;
                if target != nrz_runtime_artifact::NATIVE_EXECUTION_TARGET {
                    return Err(RuntimeArtifactError::Invariant(
                        "native sibling requires the qualified build target".into(),
                    )
                    .into());
                }
                targets.insert(layer.name.clone(), target.to_owned());
            }
            continue;
        }
        let runtime_family = layer
            .runtime_config
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|config| config.get("runtimeFamily"));
        let actual = match runtime_family {
            None => if intent.as_ref().is_some_and(|intent| {
                intent.family == nrz_source_bundle::ApplicationRuntimeFamily::Python
            }) {
                "PYTHON"
            } else {
                "JAVASCRIPT"
            }
            .to_string(),
            Some(Value::String(value)) => value.clone(),
            Some(value) => value.to_string(),
        };
        let foreign_javascript_interpreter =
            intent.as_ref().is_some_and(|intent| match intent.family {
                nrz_source_bundle::ApplicationRuntimeFamily::Bun => {
                    primary_target.starts_with("node-")
                }
                nrz_source_bundle::ApplicationRuntimeFamily::Node => {
                    primary_target.starts_with("bun-")
                }
                _ => false,
            });
        let foreign_build_target = (actual == "PYTHON" || intent.is_some())
            && layer
                .runtime_config
                .as_ref()
                .and_then(|config| config.get("buildRuntimeVersion"))
                .and_then(Value::as_str)
                .is_some_and(|target| target != primary_target);
        if actual == expected && !foreign_javascript_interpreter && !foreign_build_target {
            targets.insert(layer.name.clone(), primary_target.to_owned());
        } else if matches!(actual.as_str(), "JAVASCRIPT" | "PYTHON") {
            siblings.push((layer, actual));
        } else {
            return Err(SourceBundleMaterializationError::RuntimeFamilyMismatch {
                layer_name: layer.name.clone(),
                expected,
                actual,
            });
        }
    }
    if targets.is_empty()
        && let Some((layer, actual)) = siblings.iter().find(|(layer, _)| {
            // Untyped legacy layers still depend on their primary policy for
            // interpreter selection. Independent serving requires typed intent.
            nrz_source_bundle::layer_application_runtime(layer.runtime_config.as_ref())
                .is_ok_and(|intent| intent.is_none())
        })
    {
        if actual == expected {
            return Err(RuntimeArtifactError::Invariant(
                "application runtime conflicts with the admitted primary build target".into(),
            )
            .into());
        }
        return Err(SourceBundleMaterializationError::RuntimeFamilyMismatch {
            layer_name: layer.name.clone(),
            expected,
            actual: actual.clone(),
        });
    }
    // One policy still materializes one dependency kind. A sibling with no
    // dependency tree contributes only its immutable author declaration.
    for (layer, _) in siblings {
        let target = layer
            .runtime_config
            .as_ref()
            .and_then(|c| c.get("buildRuntimeVersion"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RuntimeArtifactError::Invariant(format!(
                    "managed sibling '{}' requires a frozen build runtime declaration",
                    layer.name
                ))
            })?;
        targets.insert(layer.name.clone(), target.to_owned());
    }
    Ok(targets)
}

#[derive(Debug, Error)]
pub enum SourceBundleMaterializationError {
    #[error("dependency root {source_root} is forbidden by the no-dependency build policy")]
    UnexpectedDependencies { source_root: String },
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
