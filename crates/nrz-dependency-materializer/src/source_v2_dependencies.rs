//! Owned dependency preparation from copied V2 bytes, without publication authority.
//!
//! The caller admits the private staging parent, all plaintext/image storage and
//! tool resources. The injected executor must retain that SAME reservation/native
//! owner and return pending cleanup unless every writable worker has joined.
//! Selection data must come from the caller's independently validated Authority
//! plan. This library checks content bindings; it cannot authenticate that plan.
//! TempDir cleanup is best effort, never a native cleanup or RootStop receipt.

use std::{
    collections::BTreeSet,
    fs::{self, File, Permissions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsFd, BorrowedFd},
        unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink},
    },
    path::Path,
    sync::Arc,
};

use nrz_contract::{
    application_package_v1::BlobReference,
    source_bundle_v2::{BlobDigest, DigestAlgorithm, FileEntry, FileRole},
};
use nrz_runtime_artifact::{
    ArtifactRoot, VerifiedDependencyMaterializationManifest,
    source_graph_v3::{LayerBuildTarget, SourceDependencyMaterializationV3},
};
use nrz_source_bundle::{DependencySourceTree, dependency::dependency_source_root};
use sha2::{Digest as _, Sha256};
use tempfile::TempDir;

use crate::{
    DependencyMaterializationRequest, DependencyMaterializerError, DependencySymlinkScope,
    DependencyTreeLimits, ErofsToolchain,
    application_erofs_tools::ApplicationErofsTools,
    application_package::ApplicationMaterializationLimits,
    source_v2_materialization::{
        AdmittedRuntimeDependency, MaterializedSourceV2Runtime, SourceV2MaterializationError,
        materialize_source_v2_runtime,
    },
    source_v2_preparation::{PreparationError, PreparedSourceV2, entry_path},
    source_v2_runtime::{
        AdmittedDependencyRoot, RuntimeVerificationError, RuntimeVerificationLimits,
        verify_prepared_source_v2_runtime,
    },
};

/// Per-root policy supplied by the trusted command/policy owner, not source JSON.
pub struct SourceV2DependencyPolicy<'a> {
    pub root: AdmittedDependencyRoot<'a>,
    pub compatibility: &'a serde_json::Value,
    pub tree_limits: DependencyTreeLimits,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceV2DependencyLimits {
    pub max_roots: usize,
    pub max_files: u64,
    pub max_bytes: u64,
    /// Additional entries, including private containers, worktrees and image links.
    pub max_staging_nodes: u64,
    /// Aggregate generated image bytes, checked after each stopped tool result.
    /// Physical enforcement during execution belongs to the caller's reservation.
    pub max_image_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceV2DependencyError {
    #[error("dependency preparation exceeds {0}")]
    Limit(&'static str),
    #[error("invalid dependency preparation: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Preparation(#[from] PreparationError),
    #[error(transparent)]
    Runtime(#[from] RuntimeVerificationError),
    #[error(transparent)]
    Materializer(#[from] DependencyMaterializerError),
    #[error("dependency preparation I/O failed: {0}")]
    Io(#[from] io::Error),
}

struct Image {
    file: File,
    manifest: VerifiedDependencyMaterializationManifest,
    physical_blob: BlobReference,
}

/// A root/mount binding and borrowed access to its one immutable image inode.
/// This is local content evidence, never durable location or publication authority.
pub struct OwnedSourceV2Dependency {
    tree: DependencySourceTree,
    target: String,
    kind: crate::DependencyMaterializationKind,
    image: Arc<Image>,
}

impl OwnedSourceV2Dependency {
    pub fn root(&self) -> AdmittedDependencyRoot<'_> {
        AdmittedDependencyRoot {
            layer_name: &self.tree.layer_name,
            source_root: &self.tree.source_root,
            kind: self.kind,
            build_runtime_target: &self.target,
        }
    }
    pub fn mount_point(&self) -> &str {
        &self.tree.mount_point
    }
    pub fn manifest(&self) -> &VerifiedDependencyMaterializationManifest {
        &self.image.manifest
    }
    /// Computed image evidence, not the selected representation. After finish,
    /// use the graph's physical_blob to decide whether this image is published.
    pub fn physical_blob(&self) -> &BlobReference {
        &self.image.physical_blob
    }
}
impl AsFd for OwnedSourceV2Dependency {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.image.file.as_fd()
    }
}

pub(crate) struct DependencyStorage {
    // Close image handles and remove private staging before releasing the executor reservation.
    pub(crate) outputs: Vec<OwnedSourceV2Dependency>,
    _staging: TempDir,
    _toolchain: Arc<ErofsToolchain>,
}

/// Holds the original copied source/inventory and every dependency tool resource
/// while the caller obtains a same-operation physical representation plan.
pub struct PreparedSourceV2Dependencies {
    source: PreparedSourceV2,
    storage: DependencyStorage,
    targets: Vec<(String, String)>,
}
impl PreparedSourceV2Dependencies {
    pub fn source_manifest(
        &self,
    ) -> &nrz_source_bundle::source_bundle_v2::VerifiedSourceBundleV2Manifest {
        self.source.manifest()
    }
    pub fn dependencies(&self) -> &[OwnedSourceV2Dependency] {
        &self.storage.outputs
    }
}

/// Pure selection data. Root must first validate its Authority receipt, operation,
/// birth and currentness. None retains SHA; Some names the exact new BLAKE3 image.
pub struct DependencyGraphSelection<'a> {
    pub source_root: &'a str,
    pub layer_name: &'a str,
    pub mount_point: &'a str,
    pub build_runtime_target: &'a str,
    pub materialization_id: &'a str,
    pub manifest_digest: &'a str,
    pub physical_blob: Option<&'a BlobReference>,
}

pub struct SourceV2DependencySelections<'a> {
    pub source_manifest_blake3: &'a str,
    pub source_manifest_size: u64,
    pub dependencies: &'a [DependencyGraphSelection<'a>],
}

pub fn prepare_source_v2_dependencies(
    source: PreparedSourceV2,
    toolchain: Arc<ErofsToolchain>,
    staging_parent: &Path,
    targets: &[LayerBuildTarget<'_>],
    policies: &[SourceV2DependencyPolicy<'_>],
    limits: SourceV2DependencyLimits,
    runtime_limits: RuntimeVerificationLimits,
) -> Result<PreparedSourceV2Dependencies, SourceV2DependencyError> {
    if policies.len() > limits.max_roots
        || policies.len()
            > nrz_runtime_artifact::MAX_RUNTIME_LAYERS
                * nrz_runtime_artifact::MAX_LAYER_DEPENDENCIES
    {
        return Err(SourceV2DependencyError::Limit("root count"));
    }
    let roots = policies
        .iter()
        .map(|policy| policy.root)
        .collect::<Vec<_>>();
    verify_prepared_source_v2_runtime(&source, targets, &roots, runtime_limits)?;
    preflight(&source, policies, limits)?;
    let staging = tempfile::Builder::new()
        .prefix(".nrz-dependencies-v2-")
        .tempdir_in(staging_parent)?;
    fs::set_permissions(staging.path(), Permissions::from_mode(0o700))?;
    let mut owner = PreparedSourceV2Dependencies {
        source,
        storage: DependencyStorage {
            outputs: Vec::new(),
            _staging: staging,
            _toolchain: toolchain,
        },
        targets: targets
            .iter()
            .map(|target| {
                (
                    target.layer_name.to_owned(),
                    target.build_runtime_target.to_owned(),
                )
            })
            .collect(),
    };
    if let Err(error) = prepare(&mut owner, policies, limits) {
        if let SourceV2DependencyError::Materializer(DependencyMaterializerError::ToolExecution(
            tool,
        )) = &error
        {
            // The existing partial-image handoff alone does not retain source,
            // previous images, sibling trees or the Root executor reservation.
            tool.retain_until_cleanup(owner);
        }
        return Err(error);
    }
    Ok(owner)
}

fn preflight(
    source: &PreparedSourceV2,
    policies: &[SourceV2DependencyPolicy<'_>],
    limits: SourceV2DependencyLimits,
) -> Result<(), SourceV2DependencyError> {
    let mut files = 0u64;
    let mut bytes = 0u64;
    // Outer stage, tree/image containers, root directories and transient image links.
    let mut nodes = 3u64
        .checked_add(
            (policies.len() as u64)
                .checked_mul(3)
                .ok_or(SourceV2DependencyError::Limit("staging nodes"))?,
        )
        .ok_or(SourceV2DependencyError::Limit("staging nodes"))?;
    let mut directories = BTreeSet::new();
    let mut root_files = vec![0u64; policies.len()];
    let mut root_bytes = vec![0u64; policies.len()];
    let mut root_links = vec![0u64; policies.len()];
    for policy in policies {
        policy.tree_limits.validate()?;
        let family = policy
            .compatibility
            .get("runtimeFamily")
            .and_then(serde_json::Value::as_str);
        let version = policy
            .compatibility
            .get("runtimeVersion")
            .and_then(serde_json::Value::as_str);
        let expected = if family == Some("bun") {
            policy.root.build_runtime_target.strip_prefix("bun-")
        } else {
            Some(policy.root.build_runtime_target)
        };
        let actual = version.map(|version| {
            if family == Some("bun") {
                version.strip_prefix("bun-").unwrap_or(version)
            } else {
                version
            }
        });
        let family_matches = match policy.root.kind {
            crate::DependencyMaterializationKind::PythonSitePackages => family == Some("python"),
            crate::DependencyMaterializationKind::JavaScriptNodeModules => {
                family == Some("javascript")
                    || (family == Some("bun")
                        && policy.root.build_runtime_target.starts_with("bun-"))
            }
        };
        if !family_matches || expected.is_none() || expected != actual {
            return Err(SourceV2DependencyError::Invalid(
                "dependency compatibility differs from frozen target",
            ));
        }
    }
    for entry in &source.manifest().wire().files {
        if role(entry) != &FileRole::Dependency {
            continue;
        }
        let path = entry_path(entry);
        let (index, policy) = policies
            .iter()
            .enumerate()
            .find(|(_, policy)| dependency_source_root(path) == Some(policy.root.source_root))
            .ok_or(SourceV2DependencyError::Invalid("dependency root missing"))?;
        let relative = relative(path, policy.root.source_root)?;
        if relative.len() > policy.tree_limits.max_path_bytes {
            return Err(SourceV2DependencyError::Limit("path bytes"));
        }
        files = files
            .checked_add(1)
            .ok_or(SourceV2DependencyError::Limit("files"))?;
        root_files[index] = root_files[index]
            .checked_add(1)
            .ok_or(SourceV2DependencyError::Limit("root files"))?;

        nodes = nodes
            .checked_add(1)
            .ok_or(SourceV2DependencyError::Limit("staging nodes"))?;
        match entry {
            FileEntry::File { size, .. } => {
                bytes = bytes
                    .checked_add(*size)
                    .ok_or(SourceV2DependencyError::Limit("bytes"))?;
                root_bytes[index] = root_bytes[index]
                    .checked_add(*size)
                    .ok_or(SourceV2DependencyError::Limit("root bytes"))?;
            }
            FileEntry::Symlink { link_target, .. }
                if link_target.0.len() > policy.tree_limits.max_path_bytes =>
            {
                return Err(SourceV2DependencyError::Limit("link bytes"));
            }
            FileEntry::Symlink { .. } => {
                root_links[index] = root_links[index]
                    .checked_add(1)
                    .ok_or(SourceV2DependencyError::Limit("root links"))?
            }
        }
        for (slash, _) in relative.match_indices('/') {
            if directories.insert((index, &relative[..slash])) {
                nodes = nodes
                    .checked_add(1)
                    .ok_or(SourceV2DependencyError::Limit("staging nodes"))?;
            }
        }
        if root_files[index] > policy.tree_limits.max_files
            || root_bytes[index] > policy.tree_limits.max_expanded_bytes
            || root_links[index] > policy.tree_limits.max_symlinks
        {
            return Err(SourceV2DependencyError::Limit("root files/bytes/links"));
        }
        if files > limits.max_files || bytes > limits.max_bytes || nodes > limits.max_staging_nodes
        {
            return Err(SourceV2DependencyError::Limit(
                "aggregate files/bytes/staging nodes",
            ));
        }
    }
    if nodes > limits.max_staging_nodes {
        return Err(SourceV2DependencyError::Limit("staging nodes"));
    }
    Ok(())
}

fn prepare(
    owner: &mut PreparedSourceV2Dependencies,
    policies: &[SourceV2DependencyPolicy<'_>],
    limits: SourceV2DependencyLimits,
) -> Result<(), SourceV2DependencyError> {
    let root = owner.storage._staging.path();
    let trees_root = root.join("trees");
    let images_root = root.join("images");
    fs::DirBuilder::new().mode(0o700).create(&trees_root)?;
    fs::DirBuilder::new().mode(0o700).create(&images_root)?;
    let mut trees = Vec::with_capacity(policies.len());
    for (index, policy) in policies.iter().enumerate() {
        let path = trees_root.join(index.to_string());
        fs::create_dir(&path)?;
        fs::set_permissions(&path, Permissions::from_mode(0o755))?;
        let mut tree = DependencySourceTree {
            source_root: policy.root.source_root.to_owned(),
            layer_name: policy.root.layer_name.to_owned(),
            mount_point: format!("/output/{}", policy.root.source_root),
            path,
            file_count: 0,
            logical_bytes: 0,
        };
        for entry in &owner.source.manifest().wire().files {
            if role(entry) != &FileRole::Dependency
                || dependency_source_root(entry_path(entry)) != Some(policy.root.source_root)
            {
                continue;
            }
            let destination = tree
                .path
                .join(relative(entry_path(entry), policy.root.source_root)?);
            fs::create_dir_all(destination.parent().expect("relative file has parent"))?;
            match entry {
                FileEntry::File {
                    size,
                    blob_digest,
                    executable,
                    ..
                } => {
                    let mut input = owner.source.open_file(entry_path(entry))?;
                    let mut output = File::options()
                        .write(true)
                        .create_new(true)
                        .open(&destination)?;
                    let mut remaining = *size;
                    let mut hash = blake3::Hasher::new();
                    let mut buffer = [0u8; 64 * 1024];
                    while remaining != 0 {
                        let wanted = remaining.min(buffer.len() as u64) as usize;
                        let count = input.read(&mut buffer[..wanted])?;
                        if count == 0 {
                            return Err(SourceV2DependencyError::Invalid(
                                "prepared file shortened",
                            ));
                        }
                        output.write_all(&buffer[..count])?;
                        hash.update(&buffer[..count]);
                        remaining -= count as u64;
                    }
                    if input.read(&mut buffer[..1])? != 0
                        || hash.finalize().to_hex().as_str() != blob_digest.value
                    {
                        return Err(SourceV2DependencyError::Invalid("prepared file changed"));
                    }
                    output.set_permissions(Permissions::from_mode(if *executable {
                        0o755
                    } else {
                        0o644
                    }))?;
                    tree.logical_bytes += size;
                }
                FileEntry::Symlink { link_target, .. } => {
                    if owner
                        .source
                        .root
                        .read_link(Path::new(entry_path(entry)))?
                        .as_os_str()
                        != std::ffi::OsStr::new(&link_target.0)
                    {
                        return Err(SourceV2DependencyError::Invalid("prepared link changed"));
                    }
                    symlink(&link_target.0, destination)?;
                }
            }
            tree.file_count += 1;
        }
        trees.push(tree);
    }
    let images = ArtifactRoot::open(&images_root)?;
    let mut total_images = 0u64;
    let mut total_files = 0u64;
    let mut total_bytes = 0u64;
    for (index, (tree, policy)) in trees.iter().zip(policies).enumerate() {
        let output = owner
            .storage
            ._toolchain
            .materialize(DependencyMaterializationRequest {
                source_tree: &tree.path,
                output_image: &images_root.join(format!("{index}.erofs")),
                kind: policy.root.kind,
                compatibility: policy.compatibility.clone(),
                limits: policy.tree_limits,
                symlink_scope: DependencySymlinkScope::RuntimeMounts {
                    mount_point: &tree.mount_point,
                    trees: &trees,
                },
            })?;
        total_files = total_files
            .checked_add(output.tree.expanded_file_count)
            .ok_or(SourceV2DependencyError::Limit("files"))?;
        total_bytes = total_bytes
            .checked_add(output.tree.expanded_bytes)
            .ok_or(SourceV2DependencyError::Limit("bytes"))?;
        total_images = total_images
            .checked_add(output.image_size)
            .ok_or(SourceV2DependencyError::Limit("image bytes"))?;
        if total_files > limits.max_files
            || total_bytes > limits.max_bytes
            || total_images > limits.max_image_bytes
        {
            return Err(SourceV2DependencyError::Limit("generated aggregate"));
        }
        let image = open_image(&images, &output)?;
        let image = if let Some(previous) = owner.storage.outputs.iter().find(|previous| {
            previous.manifest().materialization_id() == image.manifest.materialization_id()
        }) {
            if previous.manifest().manifest_digest() != image.manifest.manifest_digest()
                || previous.physical_blob() != &image.physical_blob
            {
                return Err(SourceV2DependencyError::Invalid(
                    "same materialization has conflicting image",
                ));
            }
            // Retain one image inode for equal materializations, not a second copy.
            drop(image);
            fs::remove_file(&output.image_path)?;
            File::open(&images_root)?.sync_all()?;
            Arc::clone(&previous.image)
        } else {
            Arc::new(image)
        };
        owner.storage.outputs.push(OwnedSourceV2Dependency {
            tree: tree.clone(),
            target: policy.root.build_runtime_target.to_owned(),
            kind: policy.root.kind,
            image,
        });
    }
    Ok(())
}

fn open_image(
    root: &ArtifactRoot,
    output: &crate::DependencyMaterializationOutput,
) -> Result<Image, SourceV2DependencyError> {
    let file = root.open_file(Path::new(
        output
            .image_path
            .file_name()
            .ok_or(SourceV2DependencyError::Invalid("image name"))?,
    ))?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o222 != 0
        || metadata.len() != output.image_size
    {
        return Err(SourceV2DependencyError::Invalid("image custody changed"));
    }
    let mut reader = &file;
    let mut sha = Sha256::new();
    let mut blake = blake3::Hasher::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .ok_or(SourceV2DependencyError::Limit("image bytes"))?;
        if bytes > output.image_size {
            return Err(SourceV2DependencyError::Invalid("image grew"));
        }
        sha.update(&buffer[..count]);
        blake.update(&buffer[..count]);
    }
    if bytes != output.image_size
        || format!("sha256:{}", hex::encode(sha.finalize())) != output.image_digest
    {
        return Err(SourceV2DependencyError::Invalid("image SHA mismatch"));
    }
    let after = file.metadata()?;
    if (
        after.dev(),
        after.ino(),
        after.len(),
        after.nlink(),
        after.mode(),
    ) != (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.nlink(),
        metadata.mode(),
    ) {
        return Err(SourceV2DependencyError::Invalid(
            "image changed during readback",
        ));
    }
    reader.seek(SeekFrom::Start(0))?;
    Ok(Image {
        file,
        manifest: output.manifest.clone(),
        physical_blob: BlobReference {
            media_type: nrz_runtime_artifact::DEPENDENCY_EROFS_MEDIA_TYPE.into(),
            digest: BlobDigest {
                algorithm: DigestAlgorithm::Blake3,
                value: blake.finalize().to_hex().to_string(),
            },
            size: bytes,
        },
    })
}

/// Consume once, after Root has authenticated and frozen the same-birth plan.
pub fn materialize_prepared_source_v2_runtime(
    prepared: PreparedSourceV2Dependencies,
    selections: SourceV2DependencySelections<'_>,
    staging_parent: &Path,
    tools: &ApplicationErofsTools,
    package_limits: ApplicationMaterializationLimits,
    runtime_limits: RuntimeVerificationLimits,
) -> Result<MaterializedSourceV2Runtime, SourceV2MaterializationError> {
    let invalid = || {
        SourceV2MaterializationError::Dependency(
            "physical selections differ from prepared dependencies",
        )
    };
    if selections.source_manifest_blake3 != prepared.source.manifest().manifest_blake3()
        || selections.source_manifest_size != prepared.source.manifest().manifest_size_bytes()
        || selections.dependencies.len() != prepared.storage.outputs.len()
    {
        return Err(invalid());
    }
    let mut dependencies = Vec::with_capacity(prepared.storage.outputs.len());
    for (index, selection) in selections.dependencies.iter().enumerate() {
        if selections.dependencies[..index].iter().any(|previous| {
            previous.source_root == selection.source_root
                || (previous.materialization_id == selection.materialization_id
                    && previous.physical_blob != selection.physical_blob)
        }) {
            return Err(invalid());
        }
        let dependency = prepared
            .storage
            .outputs
            .iter()
            .find(|dependency| dependency.tree.source_root == selection.source_root)
            .ok_or_else(invalid)?;
        if selection.layer_name != dependency.tree.layer_name
            || selection.mount_point != dependency.tree.mount_point
            || selection.build_runtime_target != dependency.target
            || selection.materialization_id != dependency.manifest().materialization_id()
            || selection.manifest_digest != dependency.manifest().manifest_digest()
            || selection
                .physical_blob
                .is_some_and(|blob| blob != dependency.physical_blob())
        {
            return Err(invalid());
        }
        dependencies.push(AdmittedRuntimeDependency {
            root: dependency.root(),
            materialization: SourceDependencyMaterializationV3 {
                layer_name: &dependency.tree.layer_name,
                mount_point: &dependency.tree.mount_point,
                manifest: dependency.manifest(),
                physical_blob: selection.physical_blob,
            },
        });
    }
    let targets = prepared
        .targets
        .iter()
        .map(|(layer, target)| LayerBuildTarget {
            layer_name: layer,
            build_runtime_target: target,
        })
        .collect::<Vec<_>>();
    let result = materialize_source_v2_runtime(
        prepared.source,
        staging_parent,
        tools,
        &targets,
        &dependencies,
        package_limits,
        runtime_limits,
    );
    drop(dependencies);
    match result {
        Ok(mut runtime) => {
            runtime.retain_dependencies(prepared.storage);
            Ok(runtime)
        }
        Err(error) => {
            if let SourceV2MaterializationError::Package(
                crate::application_package::ApplicationMaterializationError::Tool(tool),
            ) = &error
            {
                tool.retain_until_cleanup(prepared.storage);
            }
            Err(error)
        }
    }
}

fn role(entry: &FileEntry) -> &FileRole {
    match entry {
        FileEntry::File { role, .. } | FileEntry::Symlink { role, .. } => role,
    }
}
fn relative<'a>(path: &'a str, root: &str) -> Result<&'a str, SourceV2DependencyError> {
    path.strip_prefix(root)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .filter(|path| !path.is_empty())
        .ok_or(SourceV2DependencyError::Invalid(
            "dependency path outside root",
        ))
}

#[cfg(test)]
#[path = "source_v2_dependencies_tests.rs"]
mod tests;
