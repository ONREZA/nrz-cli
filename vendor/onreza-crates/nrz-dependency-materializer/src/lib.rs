// @generated vendored copy of platform crates/nrz-dependency-materializer/src/lib.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

//! Trusted canonical EROFS materialization for final runtime dependency trees.
//!
//! The caller must stop the untrusted build cell and transfer the staging tree
//! to a trusted, private ownership boundary before invoking this crate. This
//! crate validates the closed tree, normalizes filesystem metadata, generates a
//! self-contained image with a pinned `erofs-utils` toolchain, scans all image
//! data with `fsck.erofs`, and emits the shared runtime artifact contract.

mod source_bundle;

pub use source_bundle::{
    MaterializedRuntimeDependency, MaterializedSourceBundleRuntime,
    SourceBundleMaterializationError, SourceBundleMaterializationPolicy,
    SourceBundleMaterializationRequest, materialize_source_bundle_runtime,
};

use std::ffi::OsStr;
use std::fs::{self, File, Metadata, Permissions};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use nrz_runtime_artifact::{
    DEPENDENCY_EROFS_MEDIA_TYPE, DEPENDENCY_MATERIALIZATION_V1_SCHEMA_VERSION,
    VerifiedDependencyMaterializationManifest, verify_dependency_materialization_manifest,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const FILE_MODE: u32 = 0o644;
const EXECUTABLE_FILE_MODE: u32 = 0o755;
const DIRECTORY_MODE: u32 = 0o755;
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const IO_BUFFER_BYTES: usize = 1024 * 1024;
const TOOL_OUTPUT_LIMIT_BYTES: usize = 4096;

/// Versioned policy input whose SHA-256 is stored in every materialization.
///
/// Any change that can alter tree identity or image bytes must change this
/// descriptor. `generatorDigest` independently identifies the exact
/// `mkfs.erofs` binary.
pub const DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V1: &str = concat!(
    "ONREZA_DEPENDENCY_EROFS_CANONICALIZATION_V1\n",
    "paths=utf8,lexicographic,relative\n",
    "entries=directory,regular-file,safe-relative-symlink\n",
    "file-mode=0644-or-0755-by-executable-bit\n",
    "directory-mode=0755\n",
    "uid-gid=0:0\n",
    "timestamps=unix-epoch\n",
    "xattrs=disabled\n",
    "hardlinks=dereferenced\n",
    "block-size=4096\n",
    "physical-cluster-size=4096\n",
    "compression=zstd-level-3\n",
    "workers=1\n",
    "uuid=clear\n",
);

pub const DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V2: &str = concat!(
    "ONREZA_DEPENDENCY_EROFS_CANONICALIZATION_V2\n",
    "paths=utf8,lexicographic,relative\n",
    "entries=directory,regular-file,safe-relative-symlink\n",
    "symlink-scope=closed-tree-or-manifest-owned-layer-mount\n",
    "file-mode=0644-or-0755-by-executable-bit\n",
    "directory-mode=0755\n",
    "uid-gid=0:0\n",
    "timestamps=unix-epoch\n",
    "xattrs=disabled\n",
    "hardlinks=dereferenced\n",
    "block-size=4096\n",
    "physical-cluster-size=4096\n",
    "compression=zstd-level-3\n",
    "workers=1\n",
    "uuid=clear\n",
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyMaterializationKind {
    JavaScriptNodeModules,
    PythonSitePackages,
}

impl DependencyMaterializationKind {
    fn as_contract_value(self) -> &'static str {
        match self {
            Self::JavaScriptNodeModules => "JAVASCRIPT_NODE_MODULES",
            Self::PythonSitePackages => "PYTHON_SITE_PACKAGES",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DependencyTreeLimits {
    pub max_files: u64,
    pub max_expanded_bytes: u64,
    pub max_path_bytes: usize,
    pub max_symlinks: u64,
}

impl DependencyTreeLimits {
    fn validate(self) -> Result<(), DependencyMaterializerError> {
        if self.max_files == 0
            || self.max_expanded_bytes == 0
            || self.max_path_bytes == 0
            || self.max_symlinks == 0
        {
            return Err(DependencyMaterializerError::InvalidLimits);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyTreeSummary {
    pub logical_tree_digest: String,
    pub expanded_file_count: u64,
    pub expanded_bytes: u64,
    pub regular_file_count: u64,
    pub symlink_count: u64,
    pub native_object_count: u64,
}

pub struct DependencyMaterializationRequest<'a> {
    pub source_tree: &'a Path,
    pub output_image: &'a Path,
    pub kind: DependencyMaterializationKind,
    pub compatibility: Value,
    pub limits: DependencyTreeLimits,
    pub symlink_scope: DependencySymlinkScope<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum DependencySymlinkScope<'a> {
    ClosedTree,
    RuntimeMounts {
        mount_point: &'a str,
        allowed_mount_points: &'a [String],
    },
}

#[derive(Debug, Clone)]
pub struct DependencyMaterializationOutput {
    pub image_path: PathBuf,
    pub image_digest: String,
    pub image_size: u64,
    pub tree: DependencyTreeSummary,
    pub manifest: VerifiedDependencyMaterializationManifest,
}

#[derive(Debug, Clone)]
pub struct ErofsToolchain {
    mkfs_erofs: PathBuf,
    fsck_erofs: PathBuf,
}

impl ErofsToolchain {
    pub fn new(
        mkfs_erofs: impl Into<PathBuf>,
        fsck_erofs: impl Into<PathBuf>,
    ) -> Result<Self, DependencyMaterializerError> {
        let toolchain = Self {
            mkfs_erofs: mkfs_erofs.into(),
            fsck_erofs: fsck_erofs.into(),
        };
        for (name, path) in [
            ("mkfs.erofs", &toolchain.mkfs_erofs),
            ("fsck.erofs", &toolchain.fsck_erofs),
        ] {
            if !path.is_absolute() {
                return Err(DependencyMaterializerError::ToolPath {
                    name,
                    path: path.clone(),
                });
            }
            let metadata =
                fs::metadata(path).map_err(|source| DependencyMaterializerError::Io {
                    operation: "read tool metadata",
                    path: path.clone(),
                    source,
                })?;
            if !metadata.is_file() {
                return Err(DependencyMaterializerError::ToolPath {
                    name,
                    path: path.clone(),
                });
            }
        }
        Ok(toolchain)
    }

    pub fn materialize(
        &self,
        request: DependencyMaterializationRequest<'_>,
    ) -> Result<DependencyMaterializationOutput, DependencyMaterializerError> {
        request.limits.validate()?;
        let source_tree = canonical_source_tree(request.source_tree)?;
        let output_image = canonical_output_path(request.output_image)?;
        if output_image.starts_with(&source_tree) {
            return Err(DependencyMaterializerError::OutputInsideSource(
                output_image,
            ));
        }
        if output_image.exists() {
            return Err(DependencyMaterializerError::OutputExists(output_image));
        }

        normalize_tree(&source_tree, request.limits)?;
        let inspection = inspect_dependency_tree_with_scope(
            &source_tree,
            request.limits,
            request.symlink_scope,
        )?;
        let tree = inspection.summary;
        let generator_digest = sha256_file(&self.mkfs_erofs)?;
        let policy_digest =
            canonicalization_policy_digest_for(inspection.uses_runtime_mount_symlink);

        let partial_path = output_image.with_file_name(format!(
            ".{}.partial-{}",
            output_image
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| DependencyMaterializerError::InvalidOutputPath(
                    output_image.clone()
                ))?,
            std::process::id(),
        ));
        if partial_path.exists() {
            return Err(DependencyMaterializerError::OutputExists(partial_path));
        }
        let partial = PartialImage::new(partial_path.clone());

        let mkfs_output = Command::new(&self.mkfs_erofs)
            .args([
                "-z",
                "zstd,level=3",
                "-C4096",
                "-b4096",
                "-T0",
                "-Uclear",
                "--all-root",
                "--all-time",
                "--hard-dereference",
                "--ignore-mtime",
                "--workers=1",
                "-x",
                "-1",
                "--quiet",
            ])
            .arg(&partial_path)
            .arg(&source_tree)
            .output()
            .map_err(|source| DependencyMaterializerError::Io {
                operation: "execute mkfs.erofs",
                path: self.mkfs_erofs.clone(),
                source,
            })?;
        require_tool_success("mkfs.erofs", &mkfs_output)?;

        let fsck_output = Command::new(&self.fsck_erofs)
            .arg("--extract")
            .arg(&partial_path)
            .output()
            .map_err(|source| DependencyMaterializerError::Io {
                operation: "execute fsck.erofs",
                path: self.fsck_erofs.clone(),
                source,
            })?;
        require_tool_success("fsck.erofs --extract", &fsck_output)?;

        let inspection_after_generation = inspect_dependency_tree_with_scope(
            &source_tree,
            request.limits,
            request.symlink_scope,
        )?;
        if inspection_after_generation.summary != tree
            || inspection_after_generation.uses_runtime_mount_symlink
                != inspection.uses_runtime_mount_symlink
        {
            return Err(DependencyMaterializerError::SourceTreeChanged);
        }

        let image_metadata =
            fs::metadata(&partial_path).map_err(|source| DependencyMaterializerError::Io {
                operation: "read generated image metadata",
                path: partial_path.clone(),
                source,
            })?;
        let image_size = image_metadata.len();
        let image_digest = sha256_file(&partial_path)?;
        fs::set_permissions(&partial_path, Permissions::from_mode(0o444)).map_err(|source| {
            DependencyMaterializerError::Io {
                operation: "make generated image immutable",
                path: partial_path.clone(),
                source,
            }
        })?;
        sync_file(&partial_path)?;

        fs::hard_link(&partial_path, &output_image).map_err(|source| {
            DependencyMaterializerError::Io {
                operation: "publish generated image",
                path: output_image.clone(),
                source,
            }
        })?;
        fs::remove_file(&partial_path).map_err(|source| DependencyMaterializerError::Io {
            operation: "remove generated image staging link",
            path: partial_path.clone(),
            source,
        })?;
        partial.disarm();
        sync_parent(&output_image)?;

        let manifest_value = json!({
            "schemaVersion": DEPENDENCY_MATERIALIZATION_V1_SCHEMA_VERSION,
            "kind": request.kind.as_contract_value(),
            "compatibility": request.compatibility,
            "logicalTreeDigest": tree.logical_tree_digest,
            "expandedFileCount": tree.expanded_file_count,
            "expandedBytes": tree.expanded_bytes,
            "regularFileCount": tree.regular_file_count,
            "symlinkCount": tree.symlink_count,
            "nativeObjectCount": tree.native_object_count,
            "canonicalizationPolicyDigest": policy_digest,
            "generatorDigest": generator_digest,
            "blobDescriptor": {
                "mediaType": DEPENDENCY_EROFS_MEDIA_TYPE,
                "digest": image_digest,
                "size": image_size,
            },
        });
        let manifest = verify_dependency_materialization_manifest(manifest_value)
            .map_err(|error| DependencyMaterializerError::Contract(error.to_string()))?;

        Ok(DependencyMaterializationOutput {
            image_path: output_image,
            image_digest,
            image_size,
            tree,
            manifest,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DependencyMaterializerError {
    #[error("dependency tree limits must all be greater than zero")]
    InvalidLimits,
    #[error("{name} must be an absolute regular-file path, got {path}", path = .path.display())]
    ToolPath { name: &'static str, path: PathBuf },
    #[error("dependency source tree must be a directory: {path}", path = .0.display())]
    InvalidSourceTree(PathBuf),
    #[error("dependency image output path is invalid: {path}", path = .0.display())]
    InvalidOutputPath(PathBuf),
    #[error("dependency image output is inside its source tree: {path}", path = .0.display())]
    OutputInsideSource(PathBuf),
    #[error("dependency image output already exists: {path}", path = .0.display())]
    OutputExists(PathBuf),
    #[error("dependency tree contains a non-UTF-8 path: {path}", path = .0.display())]
    NonUtf8Path(PathBuf),
    #[error("dependency tree path exceeds the configured limit: {path}", path = .0.display())]
    PathLimit(PathBuf),
    #[error("dependency tree contains unsupported entry type: {path}", path = .0.display())]
    UnsupportedEntry(PathBuf),
    #[error("dependency tree contains an unsafe symlink {path} -> {target}", path = .path.display(), target = .target.display())]
    UnsafeSymlink { path: PathBuf, target: PathBuf },
    #[error("dependency tree exceeds {limit_name}: actual={actual} limit={limit}")]
    Limit {
        limit_name: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("dependency source tree changed while the image was generated")]
    SourceTreeChanged,
    #[error("{tool} failed with status {status}: {stderr}")]
    Tool {
        tool: &'static str,
        status: String,
        stderr: String,
    },
    #[error("generated materialization does not satisfy the runtime contract: {0}")]
    Contract(String),
    #[error("{operation} failed for {path}: {source}", path = .path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn canonicalization_policy_digest() -> String {
    sha256_prefixed(DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V1.as_bytes())
}

pub fn inspect_dependency_tree(
    root: &Path,
    limits: DependencyTreeLimits,
) -> Result<DependencyTreeSummary, DependencyMaterializerError> {
    inspect_dependency_tree_with_scope(root, limits, DependencySymlinkScope::ClosedTree)
        .map(|inspection| inspection.summary)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyTreeInspection {
    summary: DependencyTreeSummary,
    uses_runtime_mount_symlink: bool,
}

fn inspect_dependency_tree_with_scope(
    root: &Path,
    limits: DependencyTreeLimits,
    symlink_scope: DependencySymlinkScope<'_>,
) -> Result<DependencyTreeInspection, DependencyMaterializerError> {
    limits.validate()?;
    let root = canonical_source_tree(root)?;
    let mut entries = Vec::new();
    collect_entries(&root, &root, limits, &mut entries)?;
    entries.sort_by(|left, right| left.relative.as_bytes().cmp(right.relative.as_bytes()));

    let mut hasher = Sha256::new();
    hasher.update(b"ONREZA_DEPENDENCY_LOGICAL_TREE_V1\0");
    let mut summary = DependencyTreeSummary {
        logical_tree_digest: String::new(),
        expanded_file_count: 0,
        expanded_bytes: 0,
        regular_file_count: 0,
        symlink_count: 0,
        native_object_count: 0,
    };
    let mut uses_runtime_mount_symlink = false;

    for entry in entries {
        update_field(&mut hasher, entry.relative.as_bytes());
        match entry.kind {
            EntryKind::Directory => {
                hasher.update(b"D");
                hasher.update(DIRECTORY_MODE.to_be_bytes());
            }
            EntryKind::Regular => {
                hasher.update(b"F");
                let executable = entry.metadata.mode() & 0o111 != 0;
                let mode = if executable {
                    EXECUTABLE_FILE_MODE
                } else {
                    FILE_MODE
                };
                hasher.update(mode.to_be_bytes());
                hasher.update(entry.metadata.len().to_be_bytes());

                summary.regular_file_count = summary.regular_file_count.saturating_add(1);
                summary.expanded_file_count = summary.expanded_file_count.saturating_add(1);
                summary.expanded_bytes =
                    summary.expanded_bytes.saturating_add(entry.metadata.len());
                enforce_limit("max_files", summary.expanded_file_count, limits.max_files)?;
                enforce_limit(
                    "max_expanded_bytes",
                    summary.expanded_bytes,
                    limits.max_expanded_bytes,
                )?;

                let (file_digest, native_object) = hash_regular_file(&entry.path)?;
                hasher.update(file_digest);
                if native_object {
                    summary.native_object_count = summary.native_object_count.saturating_add(1);
                }
            }
            EntryKind::Symlink => {
                hasher.update(b"L");
                let target = fs::read_link(&entry.path).map_err(|source| {
                    DependencyMaterializerError::Io {
                        operation: "read dependency symlink",
                        path: entry.path.clone(),
                        source,
                    }
                })?;
                uses_runtime_mount_symlink |=
                    validate_symlink(&entry.relative, &target, &entry.path, symlink_scope)?;
                update_field(&mut hasher, target.as_os_str().as_bytes());
                summary.symlink_count = summary.symlink_count.saturating_add(1);
                summary.expanded_file_count = summary.expanded_file_count.saturating_add(1);
                enforce_limit("max_files", summary.expanded_file_count, limits.max_files)?;
                enforce_limit("max_symlinks", summary.symlink_count, limits.max_symlinks)?;
            }
        }
    }
    summary.logical_tree_digest = hex::encode(hasher.finalize());
    Ok(DependencyTreeInspection {
        summary,
        uses_runtime_mount_symlink,
    })
}

#[derive(Debug)]
struct TreeEntry {
    path: PathBuf,
    relative: String,
    metadata: Metadata,
    kind: EntryKind,
}

#[derive(Debug, Clone, Copy)]
enum EntryKind {
    Directory,
    Regular,
    Symlink,
}

fn collect_entries(
    root: &Path,
    directory: &Path,
    limits: DependencyTreeLimits,
    entries: &mut Vec<TreeEntry>,
) -> Result<(), DependencyMaterializerError> {
    let mut children = fs::read_dir(directory)
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "read dependency directory",
            path: directory.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "read dependency directory entry",
            path: directory.to_path_buf(),
            source,
        })?;
    children.sort_by_key(std::fs::DirEntry::file_name);

    for child in children {
        let path = child.path();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| DependencyMaterializerError::InvalidSourceTree(path.clone()))?;
        let relative = relative_path
            .to_str()
            .ok_or_else(|| DependencyMaterializerError::NonUtf8Path(path.clone()))?
            .to_string();
        if relative.len() > limits.max_path_bytes {
            return Err(DependencyMaterializerError::PathLimit(path));
        }
        let metadata =
            fs::symlink_metadata(&path).map_err(|source| DependencyMaterializerError::Io {
                operation: "read dependency entry metadata",
                path: path.clone(),
                source,
            })?;
        let file_type = metadata.file_type();
        let kind = if file_type.is_dir() {
            EntryKind::Directory
        } else if file_type.is_file() {
            EntryKind::Regular
        } else if file_type.is_symlink() {
            EntryKind::Symlink
        } else {
            return Err(DependencyMaterializerError::UnsupportedEntry(path));
        };
        entries.push(TreeEntry {
            path: path.clone(),
            relative,
            metadata,
            kind,
        });
        if matches!(kind, EntryKind::Directory) {
            collect_entries(root, &path, limits, entries)?;
        }
    }
    Ok(())
}

fn normalize_tree(
    root: &Path,
    limits: DependencyTreeLimits,
) -> Result<(), DependencyMaterializerError> {
    fs::set_permissions(root, Permissions::from_mode(DIRECTORY_MODE)).map_err(|source| {
        DependencyMaterializerError::Io {
            operation: "normalize dependency root mode",
            path: root.to_path_buf(),
            source,
        }
    })?;
    let mut entries = Vec::new();
    collect_entries(root, root, limits, &mut entries)?;
    for entry in entries {
        let mode = match entry.kind {
            EntryKind::Directory => Some(DIRECTORY_MODE),
            EntryKind::Regular if entry.metadata.mode() & 0o111 != 0 => Some(EXECUTABLE_FILE_MODE),
            EntryKind::Regular => Some(FILE_MODE),
            EntryKind::Symlink => None,
        };
        if let Some(mode) = mode {
            fs::set_permissions(&entry.path, Permissions::from_mode(mode)).map_err(|source| {
                DependencyMaterializerError::Io {
                    operation: "normalize dependency entry mode",
                    path: entry.path,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn validate_symlink(
    relative: &str,
    target: &Path,
    source_path: &Path,
    scope: DependencySymlinkScope<'_>,
) -> Result<bool, DependencyMaterializerError> {
    if target.as_os_str().is_empty() || target.to_str().is_none() || target.is_absolute() {
        return Err(DependencyMaterializerError::UnsafeSymlink {
            path: source_path.to_path_buf(),
            target: target.to_path_buf(),
        });
    }
    let mut depth = Path::new(relative)
        .parent()
        .map_or(0, |parent| parent.components().count());
    let mut stays_within_tree = true;
    for component in target.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth = depth.saturating_add(1),
            Component::ParentDir if depth > 0 => depth -= 1,
            Component::ParentDir => stays_within_tree = false,
            Component::RootDir | Component::Prefix(_) => {
                unreachable!("absolute targets rejected above")
            }
        }
    }
    if stays_within_tree {
        return Ok(false);
    }
    if runtime_symlink_target_is_allowed(relative, target, scope) {
        return Ok(true);
    }
    Err(DependencyMaterializerError::UnsafeSymlink {
        path: source_path.to_path_buf(),
        target: target.to_path_buf(),
    })
}

fn canonicalization_policy_digest_for(uses_runtime_mount_symlink: bool) -> String {
    let descriptor = if uses_runtime_mount_symlink {
        DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V2
    } else {
        DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V1
    };
    sha256_prefixed(descriptor.as_bytes())
}

fn runtime_symlink_target_is_allowed(
    relative: &str,
    target: &Path,
    scope: DependencySymlinkScope<'_>,
) -> bool {
    let DependencySymlinkScope::RuntimeMounts {
        mount_point,
        allowed_mount_points,
    } = scope
    else {
        return false;
    };
    let Some(mut resolved) = canonical_absolute_path(mount_point) else {
        return false;
    };
    if let Some(parent) = Path::new(relative).parent() {
        resolved.push(parent);
    }
    for component in target.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => resolved.push(part),
            Component::ParentDir if resolved.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    allowed_mount_points.iter().any(|allowed| {
        canonical_absolute_path(allowed)
            .is_some_and(|allowed| resolved == allowed || resolved.starts_with(allowed))
    })
}

fn canonical_absolute_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Normal(_) => normalized.push(component),
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}

fn hash_regular_file(path: &Path) -> Result<([u8; 32], bool), DependencyMaterializerError> {
    let mut file = File::open(path).map_err(|source| DependencyMaterializerError::Io {
        operation: "open dependency file",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; IO_BUFFER_BYTES];
    let mut prefix = [0u8; 4];
    let mut prefix_len = 0usize;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| DependencyMaterializerError::Io {
                operation: "read dependency file",
                path: path.to_path_buf(),
                source,
            })?;
        if read == 0 {
            break;
        }
        if prefix_len < prefix.len() {
            let copied = (prefix.len() - prefix_len).min(read);
            prefix[prefix_len..prefix_len + copied].copy_from_slice(&buffer[..copied]);
            prefix_len += copied;
        }
        hasher.update(&buffer[..read]);
    }
    Ok((
        hasher.finalize().into(),
        prefix_len == 4 && &prefix == ELF_MAGIC,
    ))
}

fn canonical_source_tree(path: &Path) -> Result<PathBuf, DependencyMaterializerError> {
    let canonical = path
        .canonicalize()
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "canonicalize dependency source tree",
            path: path.to_path_buf(),
            source,
        })?;
    if !fs::metadata(&canonical)
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "read dependency source metadata",
            path: canonical.clone(),
            source,
        })?
        .is_dir()
    {
        return Err(DependencyMaterializerError::InvalidSourceTree(canonical));
    }
    Ok(canonical)
}

fn canonical_output_path(path: &Path) -> Result<PathBuf, DependencyMaterializerError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| DependencyMaterializerError::InvalidOutputPath(path.to_path_buf()))?;
    let parent = path
        .parent()
        .ok_or_else(|| DependencyMaterializerError::InvalidOutputPath(path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(|source| DependencyMaterializerError::Io {
        operation: "create dependency image parent",
        path: parent.to_path_buf(),
        source,
    })?;
    let parent = parent
        .canonicalize()
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "canonicalize dependency image parent",
            path: parent.to_path_buf(),
            source,
        })?;
    Ok(parent.join(file_name))
}

fn enforce_limit(
    name: &'static str,
    actual: u64,
    limit: u64,
) -> Result<(), DependencyMaterializerError> {
    if actual > limit {
        return Err(DependencyMaterializerError::Limit {
            limit_name: name,
            actual,
            limit,
        });
    }
    Ok(())
}

fn update_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(value);
}

fn sha256_file(path: &Path) -> Result<String, DependencyMaterializerError> {
    let mut file = File::open(path).map_err(|source| DependencyMaterializerError::Io {
        operation: "open file for SHA-256",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; IO_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| DependencyMaterializerError::Io {
                operation: "read file for SHA-256",
                path: path.to_path_buf(),
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn require_tool_success(
    tool: &'static str,
    output: &Output,
) -> Result<(), DependencyMaterializerError> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = &output.stderr[..output.stderr.len().min(TOOL_OUTPUT_LIMIT_BYTES)];
    Err(DependencyMaterializerError::Tool {
        tool,
        status: output.status.to_string(),
        stderr: String::from_utf8_lossy(stderr).trim().to_string(),
    })
}

fn sync_file(path: &Path) -> Result<(), DependencyMaterializerError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "sync generated dependency image",
            path: path.to_path_buf(),
            source,
        })
}

fn sync_parent(path: &Path) -> Result<(), DependencyMaterializerError> {
    let parent = path
        .parent()
        .ok_or_else(|| DependencyMaterializerError::InvalidOutputPath(path.to_path_buf()))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| DependencyMaterializerError::Io {
            operation: "sync dependency image parent",
            path: parent.to_path_buf(),
            source,
        })
}

struct PartialImage {
    path: PathBuf,
    armed: std::cell::Cell<bool>,
}

impl PartialImage {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            armed: std::cell::Cell::new(true),
        }
    }

    fn disarm(&self) {
        self.armed.set(false);
    }
}

impl Drop for PartialImage {
    fn drop(&mut self) {
        if self.armed.get() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::time::{Duration, SystemTime};

    const LIMITS: DependencyTreeLimits = DependencyTreeLimits {
        max_files: 100,
        max_expanded_bytes: 1024 * 1024,
        max_path_bytes: 512,
        max_symlinks: 10,
    };

    #[test]
    fn logical_tree_identity_ignores_mtime_and_non_executable_mode_noise() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("node_modules/example");
        fs::create_dir_all(&package).unwrap();
        let file = package.join("index.js");
        fs::write(&file, "export const value = 1;\n").unwrap();
        symlink(
            "../example/index.js",
            root.path().join("node_modules/link.js"),
        )
        .unwrap();

        normalize_tree(root.path(), LIMITS).unwrap();
        let first = inspect_dependency_tree(root.path(), LIMITS).unwrap();
        fs::set_permissions(&file, Permissions::from_mode(0o600)).unwrap();
        let opened = File::options().write(true).open(&file).unwrap();
        opened
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(9_999)),
            )
            .unwrap();
        let second = inspect_dependency_tree(root.path(), LIMITS).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.expanded_file_count, 2);
        assert_eq!(first.regular_file_count, 1);
        assert_eq!(first.symlink_count, 1);
    }

    #[test]
    fn logical_tree_identity_includes_executable_semantics() {
        let root = tempfile::tempdir().unwrap();
        let helper = root.path().join("helper");
        fs::write(&helper, "#!/bin/sh\n").unwrap();
        let regular = inspect_dependency_tree(root.path(), LIMITS).unwrap();

        fs::set_permissions(&helper, Permissions::from_mode(0o755)).unwrap();
        let executable = inspect_dependency_tree(root.path(), LIMITS).unwrap();

        assert_ne!(regular.logical_tree_digest, executable.logical_tree_digest);
    }

    #[test]
    fn unsafe_symlink_and_special_entry_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        symlink("../../outside", root.path().join("escape")).unwrap();
        assert!(matches!(
            inspect_dependency_tree(root.path(), LIMITS),
            Err(DependencyMaterializerError::UnsafeSymlink { .. })
        ));

        fs::remove_file(root.path().join("escape")).unwrap();
        let fifo = root.path().join("pipe");
        let fifo_path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: fifo_path is a live NUL-terminated path and mode is passed by value.
        assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
        assert!(matches!(
            inspect_dependency_tree(root.path(), LIMITS),
            Err(DependencyMaterializerError::UnsupportedEntry(_))
        ));
    }

    #[test]
    fn manifest_owned_runtime_mount_scope_allows_a_cross_tree_symlink() {
        let root = tempfile::tempdir().unwrap();
        let prisma = root.path().join("@prisma");
        fs::create_dir(&prisma).unwrap();
        symlink(
            "../../../node_modules/@prisma/client",
            prisma.join("client-generated"),
        )
        .unwrap();
        let allowed_mount_points = vec![
            "/output/.next/node_modules".to_string(),
            "/output/node_modules".to_string(),
        ];

        let tree = inspect_dependency_tree_with_scope(
            root.path(),
            LIMITS,
            DependencySymlinkScope::RuntimeMounts {
                mount_point: "/output/.next/node_modules",
                allowed_mount_points: &allowed_mount_points,
            },
        )
        .unwrap();

        assert_eq!(tree.summary.symlink_count, 1);
        assert!(tree.uses_runtime_mount_symlink);
        assert_eq!(
            canonicalization_policy_digest_for(tree.uses_runtime_mount_symlink),
            sha256_prefixed(DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V2.as_bytes())
        );
        assert!(matches!(
            inspect_dependency_tree(root.path(), LIMITS),
            Err(DependencyMaterializerError::UnsafeSymlink { .. })
        ));
    }

    #[test]
    fn expanded_limits_fail_before_image_generation() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("large"), vec![0u8; 32]).unwrap();
        let limits = DependencyTreeLimits {
            max_expanded_bytes: 16,
            ..LIMITS
        };

        assert!(matches!(
            inspect_dependency_tree(root.path(), limits),
            Err(DependencyMaterializerError::Limit {
                limit_name: "max_expanded_bytes",
                ..
            })
        ));
    }

    #[test]
    fn policy_digest_is_stable_and_prefixed() {
        let digest = canonicalization_policy_digest();
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), "sha256:".len() + 64);
        assert_eq!(
            digest,
            canonicalization_policy_digest_for(false),
            "ordinary dependency trees must retain the V1 immutable identity"
        );
        assert_ne!(digest, canonicalization_policy_digest_for(true));
    }
}
