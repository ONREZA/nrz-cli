//! Runtime checks over the private prepared tree, without publication or READY authority.
//!
//! Targets and dependency roots come from independently admitted build policy. The
//! caller reserves working memory for the bounded borrowed path index and managed payload
//! reachability sets separately from the retained manifest permit, and CPU/time
//! for native metadata parsing.

use std::{collections::HashSet, io::Read, path::Path};

use nrz_contract::source_bundle_v2::{ApplicationRuntimeFamily, FileEntry, FileRole, LayerTarget};
use nrz_runtime_artifact::{
    MAX_LAYER_DEPENDENCIES, MAX_RUNTIME_LAYERS, NativeClosureReadBudget, NativeClosureReadLimits,
    NativeExecutableRequirements, RuntimeArtifactError, compile_source_runtime_layer_for_target,
    source_graph_v3::LayerBuildTarget, verify_linux_x86_64_native_platform,
};
use nrz_source_bundle::{
    PathWorkBudget, PathWorkError, PythonMinor, SourceArchivePathIndex, SourceLogicalManifestLayer,
    dependency::{
        DependencySourceTreeError, dependency_source_root, validate_dependency_source_symlink,
    },
    normalize_source_path,
};

use crate::{
    DependencyMaterializationKind,
    source_v2_preparation::{PreparationError, PreparedSourceV2, entry_path},
};

/// One independently admitted physical dependency root and its build target.
#[derive(Clone, Copy, Debug)]
pub struct AdmittedDependencyRoot<'a> {
    pub layer_name: &'a str,
    pub source_root: &'a str,
    pub kind: DependencyMaterializationKind,
    pub build_runtime_target: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct RuntimeVerificationLimits {
    /// Conservative cumulative allowance for owned native loader/closure metadata.
    /// Reserve independently of max_native_file_bytes, retained manifests and
    /// the caller's path index/Python reachability state; this is not total RSS.
    pub max_metadata_bytes: u64,
    /// Maximum bytes buffered for one ELF entry/provider read.
    pub max_native_file_bytes: u64,
    /// Cumulative actual ELF reads across every layer, including provider rereads.
    pub max_native_total_bytes: u64,
    /// Bounds path-index nodes and reachability entries; not a RAM reservation.
    pub max_path_nodes: u64,
    /// Shared allowance for the path index, symlink resolution and Python traversal.
    pub max_path_work: u64,
}

impl RuntimeVerificationLimits {
    /// Caller-funded working allowance, excluding the retained manifest permit,
    /// process/allocator headroom and external tools. Sum phases conservatively:
    /// the borrowed index stays live during native inspection, and managed
    /// reachability state is built afterwards.
    pub fn working_memory_bytes(self, max_files: u64, max_path_bytes: u64) -> Option<u64> {
        // HashSet old/new tables, control bytes and minimum table allocations.
        let paths = max_files
            .checked_mul(2)?
            .checked_mul((std::mem::size_of::<(usize, bool)>() as u64 + 1) * 4)?
            .checked_add(128)?;
        let directories = self
            .max_path_nodes
            .checked_mul(2)?
            .checked_mul((std::mem::size_of::<(usize, usize, bool)>() as u64 + 1) * 4)?
            .checked_add(128)?;
        let pending = max_files.checked_mul(4 * std::mem::size_of::<usize>() as u64)?;
        self.max_native_file_bytes
            .min(self.max_native_total_bytes)
            .checked_add(self.max_metadata_bytes)?
            .checked_add(SourceArchivePathIndex::working_memory_bytes(
                self.max_path_nodes,
                max_path_bytes,
            )?)?
            .checked_add(paths)?
            .checked_add(directories)?
            .checked_add(pending)?
            .checked_add(max_path_bytes.checked_mul(4)?.checked_add(4)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeVerificationError {
    #[error("source runtime verification exceeds {0}")]
    Limit(&'static str),
    #[error("invalid source runtime preparation: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Runtime(#[from] RuntimeArtifactError),
    #[error(transparent)]
    Preparation(#[from] PreparationError),
    #[error(transparent)]
    Dependency(#[from] DependencySourceTreeError),
    #[error(transparent)]
    Path(#[from] PathWorkError),
}

type Result<T> = std::result::Result<T, RuntimeVerificationError>;

pub(crate) fn verify_prepared_source_v2_runtime(
    source: &PreparedSourceV2,
    targets: &[LayerBuildTarget<'_>],
    dependencies: &[AdmittedDependencyRoot<'_>],
    limits: RuntimeVerificationLimits,
) -> Result<()> {
    let manifest = source.manifest().wire();
    if targets.len() > MAX_RUNTIME_LAYERS
        || dependencies.len() > MAX_RUNTIME_LAYERS * MAX_LAYER_DEPENDENCIES
    {
        return invalid("runtime policy cardinality exceeds the graph bound");
    }
    if source.usage().nodes > limits.max_path_nodes {
        return Err(RuntimeVerificationError::Limit("path node limit"));
    }
    for (index, target) in targets.iter().enumerate() {
        if targets[..index]
            .iter()
            .any(|previous| previous.layer_name == target.layer_name)
            || !manifest.layers.iter().any(|layer| {
                layer.target == LayerTarget::Compute && layer.name.0 == target.layer_name
            })
        {
            return invalid("build target references a duplicate or unknown compute layer");
        }
    }
    for layer in manifest
        .layers
        .iter()
        .filter(|layer| layer.target == LayerTarget::Compute)
    {
        let target = targets
            .iter()
            .find(|target| target.layer_name == layer.name.0)
            .ok_or(RuntimeVerificationError::Invalid(
                "compute layer has no independently admitted target",
            ))?;
        let bounded: SourceLogicalManifestLayer = serde_json::from_value(
            serde_json::to_value(layer).map_err(RuntimeArtifactError::from)?,
        )
        .map_err(RuntimeArtifactError::from)?;
        compile_source_runtime_layer_for_target(&bounded, &[], Some(target.build_runtime_target))?;
    }
    verify_dependencies(source, targets, dependencies)?;

    let needs_index = manifest.files.iter().any(|file| {
        matches!(file, FileEntry::Symlink { .. })
            && (file_role(file) == &FileRole::Dependency
                || managed_file_family(source, file).is_some())
    });
    let mut work = PathWorkBudget::new(limits.max_path_work);
    let index = needs_index
        .then(|| {
            SourceArchivePathIndex::from_entries_with_budget(
                manifest.files.iter().map(|file| {
                    (
                        entry_path(file),
                        match file {
                            FileEntry::File { .. } => None,
                            FileEntry::Symlink { link_target, .. } => Some(link_target.0.as_str()),
                        },
                    )
                }),
                &mut work,
            )
        })
        .transpose()?;
    let mut native_budget = NativeClosureReadBudget::new(NativeClosureReadLimits {
        max_metadata_bytes: limits.max_metadata_bytes,
        max_file_bytes: limits.max_native_file_bytes,
        max_total_bytes: limits.max_native_total_bytes,
    });
    for layer in manifest.layers.iter().filter(|layer| {
        layer.target == LayerTarget::Compute
            && layer
                .runtime_config
                .as_ref()
                .and_then(|config| config.application_runtime.as_ref())
                .is_some_and(|intent| intent.family == ApplicationRuntimeFamily::Executable)
    }) {
        let entry = layer.entrypoint.as_ref().expect("verified compute entry");
        let file = find_file(source, &entry.0).expect("verified compute entry is declared");
        if !matches!(file, FileEntry::File { .. }) {
            return invalid("native entry must be a declared regular file");
        }
        let (_, members) = NativeExecutableRequirements::verify_rooted_artifact_closure_bounded(
            &source.root,
            Path::new(&entry.0),
            Path::new(layer.root_path.as_ref().map_or(".", |root| &root.0)),
            &mut native_budget,
            |path, bytes| {
                let file = path.to_str().and_then(|path| find_file(source, path));
                if file.is_some_and(|file| {
                    matches!(file, FileEntry::File { size, blob_digest, .. }
                        if *size == bytes.len() as u64
                            && blob_digest.value == blake3::hash(bytes).to_hex().as_str())
                        && owns_compute_file(file, &layer.name.0)
                }) {
                    Ok(())
                } else {
                    Err(RuntimeArtifactError::Invariant(
                        "native closure bytes must belong to their compute layer".into(),
                    ))
                }
            },
        )?;
        if members.iter().any(|member| {
            !member
                .to_str()
                .and_then(|path| find_file(source, path))
                .is_some_and(|file| owns_compute_file(file, &layer.name.0))
        }) {
            return invalid("native closure aliases must belong to their compute layer");
        }
    }
    let mut checked_paths = HashSet::new();
    let mut checked_directories = HashSet::new();
    for (file_index, file) in manifest.files.iter().enumerate() {
        if let FileEntry::Symlink { link_target, .. } = file
            && file_role(file) == &FileRole::Dependency
        {
            let root = dependency_source_root(entry_path(file)).expect("checked dependency root");
            let owner = file_owner(file).expect("verified dependency owner");
            validate_dependency_source_symlink(
                entry_path(file),
                &link_target.0,
                Ok(index
                    .as_ref()
                    .expect("dependency symlink index")
                    .resolve_symlink_with_budget(entry_path(file), &mut work)?),
                root,
                owner,
                dependencies
                    .iter()
                    .map(|root| (root.source_root, root.layer_name)),
                |path| {
                    dependencies.iter().any(|root| {
                        root.layer_name == owner
                            && (within(path, root.source_root) || within(root.source_root, path))
                    })
                },
            )?;
        }
        let dependency_family = (file_role(file) == &FileRole::Dependency)
            .then(|| {
                dependencies.iter().find(|root| {
                    file_owner(file) == Some(root.layer_name)
                        && within(entry_path(file), root.source_root)
                })
            })
            .flatten()
            .map(|root| match root.kind {
                DependencyMaterializationKind::PythonSitePackages => {
                    ApplicationRuntimeFamily::Python
                }
                DependencyMaterializationKind::JavaScriptNodeModules => {
                    ApplicationRuntimeFamily::Node
                }
            });
        let Some(family) = managed_file_family(source, file).or(dependency_family) else {
            continue;
        };
        verify_managed_payload(
            source,
            file_index,
            index.as_ref(),
            &mut work,
            &mut checked_paths,
            &mut checked_directories,
            family == ApplicationRuntimeFamily::Python,
        )?;
    }
    Ok(())
}

/// Directory aliases expose their descendants, including further directory aliases.
/// Queue each declared entry once and deduplicate directory ranges to bound cycles.
fn verify_managed_payload(
    source: &PreparedSourceV2,
    file_index: usize,
    index: Option<&SourceArchivePathIndex<'_>>,
    work: &mut PathWorkBudget,
    checked_paths: &mut HashSet<(usize, bool)>,
    checked_directories: &mut HashSet<(usize, usize, bool)>,
    python: bool,
) -> Result<()> {
    let files = &source.manifest().wire().files;
    work.charge(entry_path(&files[file_index]).len() as u64 + 1)?;
    if !checked_paths.insert((file_index, python)) {
        return Ok(());
    }
    let mut pending = vec![file_index];
    while let Some(file_index) = pending.pop() {
        let file = &files[file_index];
        let path = entry_path(file);
        if matches!(file, FileEntry::File { .. }) {
            if !python && path.ends_with(".node") {
                verify_node_addon(source, path)?;
            }
            if python {
                let mut reader = source.open_file(path)?;
                verify_linux_x86_64_native_platform(&mut reader)?;
            }
            continue;
        }
        let (_, resolved) = index
            .expect("managed symlink index")
            .resolve_symlink_with_budget(path, work)?;
        let searches = files.len().ilog2() as u64 + 2;
        work.charge((resolved.len() as u64 + 1).saturating_mul(searches))?;
        if let Ok(target) = files.binary_search_by(|entry| entry_path(entry).cmp(&resolved)) {
            work.charge(1)?;
            if !python && path.ends_with(".node") {
                verify_node_addon(source, &resolved)?;
            }
            if checked_paths.insert((target, python)) {
                pending.push(target);
            }
            continue;
        }
        // Sorted paths make every implicit directory a contiguous entry range.
        // Pay for both binary searches before comparing any source path bytes.
        let prefix = format!("{resolved}/");
        let comparisons = searches * 2;
        work.charge((prefix.len() as u64 + 1).saturating_mul(comparisons))?;
        let first = files.partition_point(|entry| entry_path(entry) < prefix.as_str());
        let end =
            first + files[first..].partition_point(|entry| entry_path(entry).starts_with(&prefix));
        if !checked_directories.insert((first, end, python)) {
            continue;
        }
        for (offset, entry) in files[first..end].iter().enumerate() {
            work.charge(entry_path(entry).len() as u64 + 1)?;
            let entry_index = first + offset;
            if checked_paths.insert((entry_index, python)) {
                pending.push(entry_index);
            }
        }
    }
    Ok(())
}

fn verify_dependencies(
    source: &PreparedSourceV2,
    targets: &[LayerBuildTarget<'_>],
    dependencies: &[AdmittedDependencyRoot<'_>],
) -> Result<()> {
    for (index, root) in dependencies.iter().enumerate() {
        if normalize_source_path(root.source_root).ok().as_deref() != Some(root.source_root)
            || dependency_source_root(root.source_root) != Some(root.source_root)
            || dependencies[..index]
                .iter()
                .any(|previous| previous.source_root == root.source_root)
            || dependencies
                .iter()
                .filter(|other| other.layer_name == root.layer_name)
                .count()
                > MAX_LAYER_DEPENDENCIES
        {
            return invalid("dependency policy has an invalid, duplicate or excessive root");
        }
        let target = targets
            .iter()
            .find(|target| target.layer_name == root.layer_name)
            .ok_or(RuntimeVerificationError::Invalid(
                "dependency policy references an unknown compute layer",
            ))?;
        if root.build_runtime_target != target.build_runtime_target {
            return invalid("dependency policy target differs from its compute layer target");
        }
        let compatible = match root.kind {
            DependencyMaterializationKind::JavaScriptNodeModules => {
                root.source_root.rsplit('/').next() == Some("node_modules")
                    && (root.build_runtime_target.starts_with("bun-")
                        || root.build_runtime_target.starts_with("node-"))
            }
            DependencyMaterializationKind::PythonSitePackages => {
                PythonMinor::from_target(root.build_runtime_target)
                    .is_some_and(|minor| minor.site_packages_root() == root.source_root)
            }
        };
        if !compatible {
            return invalid("dependency root kind or minor differs from its admitted target");
        }
        if !source.manifest().wire().files.iter().any(|file| {
            file_role(file) == &FileRole::Dependency
                && file_owner(file) == Some(root.layer_name)
                && descendant(entry_path(file), root.source_root)
        }) {
            return invalid("dependency policy root has no owned source files");
        }
    }
    for file in &source.manifest().wire().files {
        let path = entry_path(file);
        if file_role(file) == &FileRole::Dependency {
            let root = dependency_source_root(path);
            if !dependencies.iter().any(|policy| {
                root == Some(policy.source_root)
                    && file_owner(file) == Some(policy.layer_name)
                    && descendant(path, policy.source_root)
            }) {
                return invalid("dependency source file has no independently admitted root");
            }
        } else if dependencies
            .iter()
            .any(|root| within(path, root.source_root) || within(root.source_root, path))
        {
            return invalid("application path collides with a dependency root");
        }
    }
    Ok(())
}

fn file_role(file: &FileEntry) -> &FileRole {
    let (FileEntry::File { role, .. } | FileEntry::Symlink { role, .. }) = file;
    role
}

fn file_owner(file: &FileEntry) -> Option<&str> {
    let (FileEntry::File { layer_name, .. } | FileEntry::Symlink { layer_name, .. }) = file;
    layer_name.as_ref().map(|name| name.0.as_str())
}

fn owns_compute_file(file: &FileEntry, layer: &str) -> bool {
    file_role(file) == &FileRole::Compute && file_owner(file) == Some(layer)
}

fn managed_file_family(
    source: &PreparedSourceV2,
    file: &FileEntry,
) -> Option<ApplicationRuntimeFamily> {
    if file_role(file) != &FileRole::Compute {
        return None;
    }
    source
        .manifest()
        .wire()
        .layers
        .iter()
        .find(|layer| file_owner(file) == Some(layer.name.0.as_str()))?
        .runtime_config
        .as_ref()?
        .application_runtime
        .as_ref()
        .map(|intent| intent.family.clone())
        .filter(|family| *family != ApplicationRuntimeFamily::Executable)
}

fn verify_node_addon(source: &PreparedSourceV2, path: &str) -> Result<()> {
    let mut reader = source.open_file(path)?;
    let mut magic = [0; 4];
    if reader.read_exact(&mut magic).is_err() || &magic != b"\x7fELF" {
        return invalid("JavaScript native addon requires Linux x86_64 ELF");
    }
    verify_linux_x86_64_native_platform(&mut magic.as_slice().chain(reader))?;
    Ok(())
}

fn find_file<'a>(source: &'a PreparedSourceV2, path: &str) -> Option<&'a FileEntry> {
    let files = &source.manifest().wire().files;
    files
        .binary_search_by(|file| entry_path(file).cmp(path))
        .ok()
        .map(|index| &files[index])
}

fn descendant(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('/'))
}

fn within(path: &str, root: &str) -> bool {
    path == root || descendant(path, root)
}

fn invalid<T>(message: &'static str) -> Result<T> {
    Err(RuntimeVerificationError::Invalid(message))
}

#[cfg(test)]
#[path = "source_v2_runtime_tests.rs"]
mod tests;
