//! Bounded private copies of declared V2 contents, without READY or publication authority.
//!
//! The caller owns staging admission and must keep the staging parent inaccessible
//! to producers. These per-operation limits do not reserve global disk capacity.
//! TempDir cleanup is best effort, not a durable recovery or cleanup receipt.

use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

use nrz_contract::source_bundle_v2::{BlobDigest, FileEntry};
use nrz_runtime_artifact::ArtifactRoot;
use nrz_source_bundle::source_bundle_v2::{
    VerifiedSourceBundleV2Manifest, admission::AdmittedSourceBundleV2Manifest,
};
use tempfile::TempDir;

const COPY_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct PreparationLimits {
    /// Aggregate declared regular-file bytes; symlinks do not contain file data.
    pub max_file_bytes: u64,
    /// Includes the private outer directory and the application tree root.
    pub max_nodes: u64,
    /// Applies independently to each entry path and raw symlink target.
    pub max_path_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparationUsage {
    pub file_bytes: u64,
    pub nodes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum PreparationError {
    #[error("source preparation exceeds {0}")]
    Limit(&'static str),
    #[error("source content differs from the manifest at {path}: {reason}")]
    Content { path: String, reason: &'static str },
    #[error("source preparation failed to {operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("requested path is not a declared regular file")]
    NotDeclaredFile,
}

/// Owns copied bytes and the admitted manifest's memory reservation together.
/// This proves declared content, not native compatibility or materializer output.
pub struct PreparedSourceV2 {
    // Declaration order closes the handle and removes the snapshot before the
    // admitted inventory releases its memory reservation.
    pub(crate) root: ArtifactRoot,
    _staging: TempDir,
    source: AdmittedSourceBundleV2Manifest,
    usage: PreparationUsage,
}

impl PreparedSourceV2 {
    #[must_use]
    pub fn manifest(&self) -> &VerifiedSourceBundleV2Manifest {
        self.source.verified()
    }

    #[must_use]
    pub fn usage(&self) -> PreparationUsage {
        self.usage
    }

    pub(crate) fn release_snapshot(
        self,
    ) -> Result<AdmittedSourceBundleV2Manifest, PreparationError> {
        let Self {
            root,
            _staging,
            source,
            ..
        } = self;
        drop(root);
        _staging
            .close()
            .map_err(|source| io_error("remove prepared snapshot", source))?;
        Ok(source)
    }

    /// Readers cannot outlive the snapshot or its inventory reservation.
    /// ```compile_fail,E0505
    /// use nrz_dependency_materializer::source_v2_preparation::PreparedSourceV2;
    /// use std::io::Read;
    /// fn detach(prepared: PreparedSourceV2) {
    ///     let mut reader = prepared.open_file("index.html").unwrap();
    ///     drop(prepared);
    ///     reader.read_exact(&mut [0]).unwrap();
    /// }
    /// ```
    pub fn open_file(&self, path: &str) -> Result<PreparedSourceFile<'_>, PreparationError> {
        let files = &self.manifest().wire().files;
        let index = files
            .binary_search_by(|entry| entry_path(entry).cmp(path))
            .map_err(|_| PreparationError::NotDeclaredFile)?;
        if !matches!(files[index], FileEntry::File { .. }) {
            return Err(PreparationError::NotDeclaredFile);
        }
        Ok(PreparedSourceFile {
            file: self
                .root
                .open_file(Path::new(path))
                .map_err(|source| io_error("open prepared regular file", source))?,
            owner: PhantomData,
        })
    }
}

/// Read and seek authority only; the owned file and directory handle stay private.
pub struct PreparedSourceFile<'a> {
    file: File,
    owner: PhantomData<&'a PreparedSourceV2>,
}

impl Read for PreparedSourceFile<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.read(bytes)
    }
}

impl Seek for PreparedSourceFile<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}

/// Copy declared contents while permitting producer mutation of the original input.
/// The caller must keep the staging parent under trusted custody for the entire
/// result lifetime; this function cannot establish host process isolation.
pub fn prepare_source_v2(
    source: AdmittedSourceBundleV2Manifest,
    input: &ArtifactRoot,
    staging_parent: &Path,
    limits: PreparationLimits,
) -> Result<PreparedSourceV2, PreparationError> {
    prepare_source_v2_inner(source, Some(input), staging_parent, limits, |path, _, _| {
        input.open_file(Path::new(path))
    })
}

/// Construct the same private snapshot from independently held exact regular-file
/// capabilities. Symlink targets come from the admitted canonical inventory.
/// The caller must retain physical custody and bound every descriptor producer.
pub fn prepare_source_v2_from_files(
    source: AdmittedSourceBundleV2Manifest,
    staging_parent: &Path,
    limits: PreparationLimits,
    open: impl FnMut(&str, &BlobDigest, u64) -> io::Result<File>,
) -> Result<PreparedSourceV2, PreparationError> {
    prepare_source_v2_inner(source, None, staging_parent, limits, open)
}

fn prepare_source_v2_inner(
    source: AdmittedSourceBundleV2Manifest,
    input: Option<&ArtifactRoot>,
    staging_parent: &Path,
    limits: PreparationLimits,
    mut open: impl FnMut(&str, &BlobDigest, u64) -> io::Result<File>,
) -> Result<PreparedSourceV2, PreparationError> {
    let files = &source.verified().wire().files;
    let usage = preflight(files, limits)?;
    let staging = tempfile::Builder::new()
        .prefix(".nrz-source-v2-")
        .tempdir_in(staging_parent)
        .map_err(|source| io_error("create private staging directory", source))?;
    fs::set_permissions(staging.path(), Permissions::from_mode(0o700))
        .map_err(|source| io_error("restrict private staging directory", source))?;
    let tree = staging.path().join("tree");
    create_directory(&tree)?;
    let mut previous = "";
    let mut buffer = [0_u8; COPY_BUFFER_BYTES];
    for entry in files {
        let path = entry_path(entry);
        let common = common_prefix_bytes(previous, path);
        for (slash, _) in path
            .match_indices('/')
            .filter(|(slash, _)| *slash >= common)
        {
            create_directory(&tree.join(&path[..slash]))?;
        }
        let destination = tree.join(path);
        match entry {
            FileEntry::File {
                size, blob_digest, ..
            } => {
                let mut reader = open(path, blob_digest, *size)
                    .map_err(|source| io_error("open source regular file", source))?;
                let mut writer = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(destination)
                    .map_err(|source| io_error("create private regular file", source))?;
                let mut remaining = *size;
                let mut hash = blake3::Hasher::new();
                while remaining != 0 {
                    let wanted = remaining.min(buffer.len() as u64) as usize;
                    let read = reader
                        .read(&mut buffer[..wanted])
                        .map_err(|source| io_error("read source regular file", source))?;
                    if read == 0 {
                        return Err(content_error(path, "short file"));
                    }
                    writer
                        .write_all(&buffer[..read])
                        .map_err(|source| io_error("write private regular file", source))?;
                    hash.update(&buffer[..read]);
                    remaining -= read as u64;
                }
                if reader
                    .read(&mut buffer[..1])
                    .map_err(|source| io_error("check source file length", source))?
                    != 0
                {
                    return Err(content_error(path, "long file"));
                }
                if hash.finalize().to_hex().as_str() != blob_digest.value {
                    return Err(content_error(path, "BLAKE3 mismatch"));
                }
                writer
                    .set_permissions(Permissions::from_mode(0o775))
                    .map_err(|source| io_error("set private regular file mode", source))?;
            }
            FileEntry::Symlink { link_target, .. } => {
                if let Some(input) = input {
                    let actual = input
                        .read_link(Path::new(path))
                        .map_err(|source| io_error("read source symlink", source))?;
                    if actual.as_os_str() != OsStr::new(&link_target.0) {
                        return Err(content_error(path, "symlink target mismatch"));
                    }
                }
                symlink(&link_target.0, destination)
                    .map_err(|source| io_error("create private symlink", source))?;
            }
        }
        previous = path;
    }
    let root =
        ArtifactRoot::open(&tree).map_err(|source| io_error("acquire prepared tree", source))?;
    Ok(PreparedSourceV2 {
        root,
        _staging: staging,
        source,
        usage,
    })
}

fn preflight(
    files: &[FileEntry],
    limits: PreparationLimits,
) -> Result<PreparationUsage, PreparationError> {
    let mut usage = PreparationUsage {
        file_bytes: 0,
        nodes: 2,
    };
    let mut previous = "";
    for entry in files {
        let path = entry_path(entry);
        if path.len() > limits.max_path_bytes {
            return Err(PreparationError::Limit("entry path byte limit"));
        }
        match entry {
            FileEntry::File { size, .. } => {
                usage.file_bytes = usage
                    .file_bytes
                    .checked_add(*size)
                    .ok_or(PreparationError::Limit("file byte limit"))?;
            }
            FileEntry::Symlink { link_target, .. }
                if link_target.0.len() > limits.max_path_bytes =>
            {
                return Err(PreparationError::Limit("symlink target byte limit"));
            }
            FileEntry::Symlink { .. } => {}
        }
        let common = common_prefix_bytes(previous, path);
        let new_directories = path
            .bytes()
            .enumerate()
            .filter(|(index, byte)| *byte == b'/' && *index >= common)
            .count() as u64;
        usage.nodes = usage
            .nodes
            .checked_add(new_directories)
            .and_then(|nodes| nodes.checked_add(1))
            .ok_or(PreparationError::Limit("node limit"))?;
        previous = path;
    }
    if usage.file_bytes > limits.max_file_bytes {
        return Err(PreparationError::Limit("file byte limit"));
    }
    if usage.nodes > limits.max_nodes {
        return Err(PreparationError::Limit("node limit"));
    }
    Ok(usage)
}

pub(crate) fn common_prefix_bytes(left: &str, right: &str) -> usize {
    left.bytes()
        .zip(right.bytes())
        .take_while(|(a, b)| a == b)
        .count()
}

pub(crate) fn entry_path(entry: &FileEntry) -> &str {
    let (FileEntry::File { path, .. } | FileEntry::Symlink { path, .. }) = entry;
    &path.0
}

pub(crate) fn create_directory(path: &Path) -> Result<(), PreparationError> {
    fs::create_dir(path).map_err(|source| io_error("create private directory", source))?;
    fs::set_permissions(path, Permissions::from_mode(0o775))
        .map_err(|source| io_error("set private directory mode", source))
}

fn content_error(path: &str, reason: &'static str) -> PreparationError {
    let mut end = path.len().min(512);
    while !path.is_char_boundary(end) {
        end -= 1;
    }
    PreparationError::Content {
        path: path[..end].into(),
        reason,
    }
}

fn io_error(operation: &'static str, source: io::Error) -> PreparationError {
    PreparationError::Io { operation, source }
}

#[cfg(test)]
#[path = "source_v2_preparation_tests.rs"]
mod tests;
