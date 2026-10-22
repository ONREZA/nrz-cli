//! Independent readback after the extractor has terminated. The caller retains
//! exclusive private custody of this tree throughout verification. Image UID/GID
//! and xattrs are verified from image metadata before extraction, not host stat.

use crate::source_v2_preparation::{common_prefix_bytes, entry_path};
use nrz_contract::source_bundle_v2::FileEntry;
use nrz_runtime_artifact::ArtifactRoot;
use std::{
    fs::{self, ReadDir},
    io::Read,
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::Path,
};

const BUFFER_BYTES: usize = 64 * 1024;
const ITERATOR_ALLOWANCE: u64 = 256;

#[derive(Debug, thiserror::Error)]
pub enum ReadbackError {
    #[error("application image readback exceeds {0}")]
    Limit(&'static str),
    #[error("application image readback mismatch: {0}")]
    Mismatch(&'static str),
    #[error("application image readback failed: {0}")]
    Io(#[from] std::io::Error),
}

struct Frame {
    entries: ReadDir,
    path_len: usize,
}

/// Bounds requested traversal storage; allocator/kernel iterator overhead and
/// process-wide admission remain the caller's responsibility.
pub(crate) fn verify_extracted_tree(
    root: &Path,
    expected: &[&FileEntry],
    max_nodes: u64,
    max_path_bytes: usize,
    max_memory_bytes: u64,
) -> Result<(), ReadbackError> {
    let (expected_nodes, depth) = expected_bounds(expected, max_nodes, max_path_bytes)?;
    let frame_bytes = (root.as_os_str().len() as u64)
        .checked_add(max_path_bytes as u64)
        .and_then(|bytes| bytes.checked_add(ITERATOR_ALLOWANCE + size_of::<Frame>() as u64))
        .ok_or(ReadbackError::Limit("working memory"))?;
    let base_bytes = (BUFFER_BYTES as u64)
        .checked_add(max_path_bytes as u64)
        .and_then(|bytes| bytes.checked_add(frame_bytes))
        .ok_or(ReadbackError::Limit("working memory"))?;
    check_memory(base_bytes, frame_bytes, depth, max_memory_bytes)?;
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() {
        return Err(ReadbackError::Mismatch("root is not a directory"));
    }
    check_metadata(&metadata, 0o775)?;
    let owner = ArtifactRoot::open(root)?;
    let mut relative = String::new();
    relative
        .try_reserve_exact(max_path_bytes)
        .map_err(|_| ReadbackError::Limit("working memory"))?;
    let mut frames = Vec::new();
    frames
        .try_reserve_exact(depth)
        .map_err(|_| ReadbackError::Limit("working memory"))?;
    frames.push(Frame {
        entries: fs::read_dir(root)?,
        path_len: 0,
    });
    let mut visited = 1_u64;
    let mut buffer = [0_u8; BUFFER_BYTES];
    while let Some(frame) = frames.last_mut() {
        relative.truncate(frame.path_len);
        let Some(entry) = frame.entries.next() else {
            frames.pop();
            continue;
        };
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or(ReadbackError::Mismatch("non-UTF-8 entry"))?;
        let extra = name.len() + usize::from(!relative.is_empty());
        if relative
            .len()
            .checked_add(extra)
            .is_none_or(|len| len > max_path_bytes)
        {
            return Err(ReadbackError::Limit("path bytes"));
        }
        if !relative.is_empty() {
            relative.push('/');
        }
        relative.push_str(name);
        visited = visited
            .checked_add(1)
            .filter(|nodes| *nodes <= max_nodes)
            .ok_or(ReadbackError::Limit("nodes"))?;
        let metadata = fs::symlink_metadata(entry.path())?;
        let found = expected.binary_search_by(|file| entry_path(file).cmp(&relative));
        if metadata.is_dir() {
            if found.is_ok() || !is_implicit_directory(expected, &relative) {
                return Err(ReadbackError::Mismatch("unexpected directory"));
            }
            check_metadata(&metadata, 0o775)?;
            if frames.len() == depth {
                return Err(ReadbackError::Mismatch("unexpected directory depth"));
            }
            frames.push(Frame {
                entries: fs::read_dir(entry.path())?,
                path_len: relative.len(),
            });
            continue;
        }
        let file = expected
            .get(found.map_err(|_| ReadbackError::Mismatch("unexpected entry"))?)
            .ok_or(ReadbackError::Mismatch("unexpected entry"))?;
        match file {
            FileEntry::File {
                size, blob_digest, ..
            } if metadata.is_file() => {
                check_metadata(&metadata, 0o775)?;
                if metadata.len() != *size {
                    return Err(ReadbackError::Mismatch("file size"));
                }
                let mut input = owner.open_file(Path::new(&relative))?;
                let mut hash = blake3::Hasher::new();
                let mut remaining = *size;
                while remaining != 0 {
                    let wanted = remaining.min(buffer.len() as u64) as usize;
                    let count = input.read(&mut buffer[..wanted])?;
                    if count == 0 {
                        return Err(ReadbackError::Mismatch("short file"));
                    }
                    hash.update(&buffer[..count]);
                    remaining -= count as u64;
                }
                if input.read(&mut buffer[..1])? != 0
                    || hash.finalize().to_hex().as_str() != blob_digest.value
                {
                    return Err(ReadbackError::Mismatch("file bytes"));
                }
            }
            FileEntry::Symlink { link_target, .. } if metadata.is_symlink() => {
                check_metadata(&metadata, 0o777)?;
                if link_target.0.len() > max_path_bytes {
                    return Err(ReadbackError::Limit("link bytes"));
                }
                if metadata.len() != link_target.0.len() as u64
                    || owner
                        .read_link(Path::new(&relative))?
                        .as_os_str()
                        .as_bytes()
                        != link_target.0.as_bytes()
                {
                    return Err(ReadbackError::Mismatch("symlink text"));
                }
            }
            _ => return Err(ReadbackError::Mismatch("entry type")),
        }
    }
    if visited != expected_nodes {
        return Err(ReadbackError::Mismatch("missing entries"));
    }
    Ok(())
}

fn check_metadata(metadata: &fs::Metadata, mode: u32) -> Result<(), ReadbackError> {
    if metadata.mode() & 0o7777 != mode || metadata.mtime() != 0 || metadata.mtime_nsec() != 0 {
        return Err(ReadbackError::Mismatch("mode or timestamp"));
    }
    Ok(())
}

fn check_memory(base: u64, frame: u64, depth: usize, limit: u64) -> Result<(), ReadbackError> {
    if frame
        .checked_mul(depth as u64)
        .and_then(|bytes| base.checked_add(bytes))
        .is_none_or(|bytes| bytes > limit)
    {
        return Err(ReadbackError::Limit("working memory"));
    }
    Ok(())
}

fn is_implicit_directory(expected: &[&FileEntry], directory: &str) -> bool {
    let index = expected.partition_point(|file| {
        entry_path(file)
            .bytes()
            .cmp(directory.bytes().chain(std::iter::once(b'/')))
            .is_lt()
    });
    expected.get(index).is_some_and(|file| {
        entry_path(file).starts_with(directory)
            && entry_path(file).as_bytes().get(directory.len()) == Some(&b'/')
    })
}

fn expected_bounds(
    expected: &[&FileEntry],
    max_nodes: u64,
    max_path: usize,
) -> Result<(u64, usize), ReadbackError> {
    let mut nodes = 1_u64;
    let mut depth = 1;
    let mut previous = "";
    for file in expected {
        let path = entry_path(file);
        if path.is_empty() || path <= previous {
            return Err(ReadbackError::Mismatch("unordered expected paths"));
        }
        if path.len() > max_path {
            return Err(ReadbackError::Limit("path bytes"));
        }
        let common = common_prefix_bytes(previous, path);
        depth = depth.max(path.bytes().filter(|byte| *byte == b'/').count() + 1);
        let directories = path
            .bytes()
            .enumerate()
            .filter(|(index, byte)| *index >= common && *byte == b'/')
            .count();
        nodes = nodes
            .checked_add(directories as u64)
            .and_then(|nodes| nodes.checked_add(1))
            .filter(|nodes| *nodes <= max_nodes)
            .ok_or(ReadbackError::Limit("nodes"))?;
        previous = path;
    }
    if nodes > max_nodes {
        return Err(ReadbackError::Limit("nodes"));
    }
    Ok((nodes, depth))
}

#[cfg(test)]
#[path = "application_erofs_readback_tests.rs"]
mod tests;
