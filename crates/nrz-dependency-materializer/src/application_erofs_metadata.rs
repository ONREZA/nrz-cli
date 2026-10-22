//! Bounded metadata preflight for the pinned application EROFS recipe.
use std::collections::HashSet;
use std::fs::File;
use std::mem::size_of;
use std::os::unix::fs::FileExt;

use nrz_contract::source_bundle_v2::FileEntry;

const BLOCK: usize = 4096;
const NAME: usize = 255;

#[derive(Clone, Copy)]
pub(crate) struct MetadataLimits {
    pub max_nodes: u64,
    pub max_path_bytes: usize,
    pub max_memory_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MetadataError {
    #[error("unsupported or invalid application EROFS metadata: {0}")]
    Invalid(&'static str),
    #[error("application EROFS metadata exceeds {0} limit")]
    Limit(&'static str),
    #[error("cannot read application EROFS metadata: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, MetadataError>;

#[derive(Clone, Copy)]
struct Inode {
    nid: u64,
    offset: u64,
    header_size: u64,
    mode: u16,
    size: u64,
    start_block: u32,
    layout: u16,
    links: u32,
}

struct Frame {
    inode: Inode,
    parent: u64,
    block_offset: u64,
    entry: usize,
    path_len: usize,
    previous: [u8; NAME],
    previous_len: usize,
    dots: u8,
    child_dirs: u32,
}

impl Frame {
    fn new(inode: Inode, parent: u64, path_len: usize) -> Self {
        Self {
            inode,
            parent,
            block_offset: 0,
            entry: 0,
            path_len,
            previous: [0; NAME],
            previous_len: 0,
            dots: 0,
            child_dirs: 0,
        }
    }
}

struct Image<'a> {
    file: &'a File,
    size: u64,
    metadata: u64,
}

impl Image<'_> {
    fn read(&self, offset: u64, buffer: &mut [u8]) -> Result<()> {
        if offset
            .checked_add(buffer.len() as u64)
            .is_none_or(|end| end > self.size)
        {
            return Err(MetadataError::Invalid("offset outside filesystem"));
        }
        self.file.read_exact_at(buffer, offset)?;
        Ok(())
    }

    fn inode(&self, nid: u64) -> Result<Inode> {
        let offset = nid
            .checked_mul(32)
            .and_then(|n| self.metadata.checked_add(n))
            .ok_or(MetadataError::Invalid("inode offset overflow"))?;
        let mut header = [0; 64];
        self.read(offset, &mut header[..32])?;
        let format = u16_at(&header, 0);
        // This recipe has neither 48-bit block addresses nor omitted dot entries.
        if format & !0xf != 0 || u16_at(&header, 2) != 0 {
            return Err(MetadataError::Invalid("inode format or xattrs"));
        }
        let extended = format & 1 != 0;
        if extended {
            self.read(offset, &mut header)?;
        }
        let (uid, gid, time, nsec, size, links) = if extended {
            (
                u32_at(&header, 24),
                u32_at(&header, 28),
                u64_at(&header, 32),
                u32_at(&header, 40),
                u64_at(&header, 8),
                u32_at(&header, 44),
            )
        } else {
            (
                u16_at(&header, 24) as u32,
                u16_at(&header, 26) as u32,
                u32_at(&header, 12) as u64,
                0,
                u32_at(&header, 8) as u64,
                u16_at(&header, 6) as u32,
            )
        };
        if uid != 0 || gid != 0 || time != 0 || nsec != 0 {
            return Err(MetadataError::Invalid("inode ownership or timestamp"));
        }
        let mode = u16_at(&header, 4);
        let layout = (format >> 1) & 7;
        match mode {
            0o40775 if layout == 0 || layout == 2 => {}
            0o100775 if layout <= 3 && links == 1 => {}
            0o120777 if (layout == 0 || layout == 2) && links == 1 => {}
            _ => return Err(MetadataError::Invalid("inode mode, links or layout")),
        }
        Ok(Inode {
            nid,
            offset,
            header_size: if extended { 64 } else { 32 },
            mode,
            size,
            start_block: u32_at(&header, 16),
            layout,
            links,
        })
    }

    fn flat_read(&self, inode: Inode, offset: u64, bytes: &mut [u8]) -> Result<()> {
        if offset
            .checked_add(bytes.len() as u64)
            .is_none_or(|end| end > inode.size)
        {
            return Err(MetadataError::Invalid("flat data bounds"));
        }
        let full_bytes = if inode.layout == 2 {
            inode.size / BLOCK as u64 * BLOCK as u64
        } else {
            inode.size
        };
        let physical = if offset < full_bytes {
            if offset + bytes.len() as u64 > full_bytes {
                return Err(MetadataError::Invalid("flat read crosses inline tail"));
            }
            (inode.start_block as u64)
                .checked_mul(BLOCK as u64)
                .and_then(|n| n.checked_add(offset))
        } else {
            let tail_offset = offset % BLOCK as u64;
            let start = inode
                .offset
                .checked_add(inode.header_size)
                .and_then(|n| n.checked_add(tail_offset));
            if start.is_none_or(|n| n % BLOCK as u64 + bytes.len() as u64 > BLOCK as u64) {
                return Err(MetadataError::Invalid("inline tail crosses block"));
            }
            start
        }
        .ok_or(MetadataError::Invalid("flat offset overflow"))?;
        self.read(physical, bytes)
    }
}

/// Read metadata before any extraction; compressed regular-file data stays with fsck.
pub(crate) fn verify_erofs_metadata(
    image: &mut File,
    expected: &[&FileEntry],
    expected_uuid: [u8; 16],
    limits: MetadataLimits,
) -> Result<u64> {
    let (nodes, depth, path_bytes) = expected_bounds(expected, limits)?;
    // Hash table buckets include control bytes and allocator alignment; charge twice the
    // next power of two and 32 bytes per bucket, before asking the allocator for storage.
    let buckets = nodes
        .checked_mul(2)
        .and_then(u64::checked_next_power_of_two)
        .ok_or(MetadataError::Limit("memory"))?
        .max(4);
    let memory = buckets
        .checked_mul(32)
        .and_then(|n| n.checked_add((depth as u64).checked_mul(size_of::<Frame>() as u64)?))
        .and_then(|n| n.checked_add(path_bytes as u64))
        .and_then(|n| n.checked_add(BLOCK as u64 + 2048))
        .ok_or(MetadataError::Limit("memory"))?;
    if memory > limits.max_memory_bytes {
        return Err(MetadataError::Limit("memory"));
    }
    let mut visited = HashSet::new();
    visited
        .try_reserve(usize::try_from(nodes).map_err(|_| MetadataError::Limit("nodes"))?)
        .map_err(|_| MetadataError::Limit("memory"))?;
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(depth)
        .map_err(|_| MetadataError::Limit("memory"))?;
    let mut path = Vec::new();
    path.try_reserve_exact(path_bytes)
        .map_err(|_| MetadataError::Limit("memory"))?;
    let mut header = [0; 128];
    image.read_exact_at(&mut header, 1024)?;
    if u32_at(&header, 0) != 0xe0f5e1e2
        || header[12] != 12
        || header[13] != 0
        || u32_at(&header, 8) != 3
        || u32_at(&header, 80) != 3
        || u16_at(&header, 84) != 8
        || header[48..64] != expected_uuid
        || header[24..36] != [0; 12]
        || header[44..48] != [0; 4]
        || header[64..80] != [0; 16]
        || header[86..128] != [0; 42]
    {
        return Err(MetadataError::Invalid("superblock profile"));
    }
    let size = (u32_at(&header, 36) as u64) * BLOCK as u64;
    if size < BLOCK as u64 || size != image.metadata()?.len() || u64_at(&header, 16) != nodes {
        return Err(MetadataError::Invalid("filesystem size or inode inventory"));
    }
    let reader = Image {
        file: image,
        size,
        metadata: (u32_at(&header, 40) as u64) * BLOCK as u64,
    };
    let mut compression = [0; 8];
    reader.read(1152, &mut compression)?;
    if u16_at(&compression, 0) != 6
        || compression[2] != 0
        || compression[3] > 10
        || compression[4..] != [0; 4]
    {
        return Err(MetadataError::Invalid("zstd configuration"));
    }
    let root = reader.inode(u16_at(&header, 14) as u64)?;
    if root.mode != 0o40775
        || root.size
            > nodes
                .checked_mul(12 + NAME as u64)
                .and_then(|n| n.checked_add(BLOCK as u64))
                .ok_or(MetadataError::Limit("nodes"))?
    {
        return Err(MetadataError::Invalid("root type"));
    }
    visited.insert(root.nid);
    stack.push(Frame::new(root, root.nid, 0));
    let mut block = [0; BLOCK];
    let mut leaves = 0;
    let mut cached_block = None;
    while !stack.is_empty() {
        let stack_depth = stack.len();
        let frame = stack.last_mut().unwrap();
        path.truncate(frame.path_len);
        if frame.block_offset >= frame.inode.size {
            if frame.dots != 3
                || frame.inode.links
                    != frame
                        .child_dirs
                        .checked_add(2)
                        .ok_or(MetadataError::Invalid("directory link overflow"))?
            {
                return Err(MetadataError::Invalid("directory dots or link count"));
            }
            stack.pop();
            continue;
        }
        let bytes = (frame.inode.size - frame.block_offset).min(BLOCK as u64) as usize;
        let key = (frame.inode.nid, frame.block_offset);
        if cached_block != Some(key) {
            reader.flat_read(frame.inode, frame.block_offset, &mut block[..bytes])?;
            cached_block = Some(key);
        }
        if bytes < 12 {
            return Err(MetadataError::Invalid("short directory block"));
        }
        let names = u16_at(&block, 8) as usize;
        if names < 12 || !names.is_multiple_of(12) || names >= bytes {
            return Err(MetadataError::Invalid("directory name table"));
        }
        let entries = names / 12;
        if frame.entry == entries {
            frame.block_offset += bytes as u64;
            frame.entry = 0;
            continue;
        }
        let entry = frame.entry * 12;
        let start = u16_at(&block, entry + 8) as usize;
        let end = if frame.entry + 1 == entries {
            bytes
        } else {
            u16_at(&block, entry + 20) as usize
        };
        if start < names
            || start >= end
            || end > bytes
            || (frame.entry == 0 && start != names)
            || block[entry + 11] != 0
        {
            return Err(MetadataError::Invalid("directory entry bounds"));
        }
        let mut name_end = end;
        if frame.entry + 1 == entries
            && let Some(nul) = block[start..end].iter().position(|b| *b == 0)
        {
            name_end = start + nul;
            if block[name_end..end].iter().any(|b| *b != 0) {
                return Err(MetadataError::Invalid("directory padding"));
            }
        }
        let name = &block[start..name_end];
        if name.is_empty() || name.len() > NAME {
            return Err(MetadataError::Invalid("directory name length"));
        }
        if name.iter().any(|b| *b == 0 || *b == b'/')
            || name <= &frame.previous[..frame.previous_len]
        {
            return Err(MetadataError::Invalid("directory name or sort order"));
        }
        frame.previous[..name.len()].copy_from_slice(name);
        frame.previous_len = name.len();
        frame.entry += 1;
        let nid = u64_at(&block, entry);
        let kind = block[entry + 10];
        if name == b"." || name == b".." {
            let (bit, wanted) = if name == b"." {
                (1, frame.inode.nid)
            } else {
                (2, frame.parent)
            };
            if frame.dots & bit != 0 || nid != wanted || kind != 2 {
                return Err(MetadataError::Invalid("directory parent identity"));
            }
            frame.dots |= bit;
            continue;
        }
        let new_len = path
            .len()
            .checked_add(usize::from(!path.is_empty()))
            .and_then(|n| n.checked_add(name.len()))
            .ok_or(MetadataError::Limit("path"))?;
        if new_len > path_bytes || new_len > limits.max_path_bytes {
            return Err(MetadataError::Limit("path"));
        }
        if !path.is_empty() {
            path.push(b'/');
        }
        path.extend_from_slice(name);
        if visited.len() as u64 >= nodes || !visited.insert(nid) {
            return Err(MetadataError::Invalid("duplicate inode or node inventory"));
        }
        let inode = reader.inode(nid)?;
        let current =
            std::str::from_utf8(&path).map_err(|_| MetadataError::Invalid("non-UTF8 path"))?;
        if inode.mode == 0o40775 {
            if kind != 2 || !implicit_directory(expected, current) || stack_depth >= depth {
                return Err(MetadataError::Invalid("unexpected directory"));
            }
            // Every child directory consumes at least one expected descendant.
            if inode.size
                > nodes
                    .checked_mul(12 + NAME as u64)
                    .and_then(|n| n.checked_add(BLOCK as u64))
                    .ok_or(MetadataError::Limit("nodes"))?
            {
                return Err(MetadataError::Invalid("oversized directory"));
            }
            let parent = frame.inode.nid;
            frame.child_dirs = frame
                .child_dirs
                .checked_add(1)
                .ok_or(MetadataError::Invalid("directory child count overflow"))?;
            stack.push(Frame::new(inode, parent, path.len()));
        } else {
            let index = expected
                .binary_search_by(|entry| entry_path(entry).cmp(current))
                .map_err(|_| MetadataError::Invalid("unexpected leaf"))?;
            match expected[index] {
                FileEntry::File { size, .. }
                    if kind == 1 && inode.mode == 0o100775 && inode.size == *size => {}
                FileEntry::Symlink { link_target, .. }
                    if kind == 7
                        && inode.mode == 0o120777
                        && inode.size == link_target.0.len() as u64 =>
                {
                    let target: &str = link_target.0.as_str();
                    if target.len() > BLOCK {
                        return Err(MetadataError::Limit("symlink"));
                    }
                    cached_block = None;
                    reader.flat_read(inode, 0, &mut block[..target.len()])?;
                    if &block[..target.len()] != target.as_bytes() {
                        return Err(MetadataError::Invalid("symlink target"));
                    }
                }
                _ => return Err(MetadataError::Invalid("leaf type or size")),
            }
            leaves += 1;
        }
    }
    if leaves != expected.len() || visited.len() as u64 != nodes {
        return Err(MetadataError::Invalid("incomplete inventory"));
    }
    Ok(size)
}

fn expected_bounds(expected: &[&FileEntry], limits: MetadataLimits) -> Result<(u64, usize, usize)> {
    let mut nodes = 1u64;
    let mut depth = 1;
    let mut path_bytes = 0;
    let mut previous = "";
    for entry in expected {
        let path = entry_path(entry);
        if path.is_empty()
            || path <= previous
            || path.starts_with('/')
            || path.as_bytes().contains(&0)
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".." || part.len() > NAME)
        {
            return Err(MetadataError::Invalid("expected path inventory"));
        }
        if path.len() > limits.max_path_bytes {
            return Err(MetadataError::Limit("path"));
        }
        path_bytes = path_bytes.max(path.len());
        let mut entry_depth = 1;
        nodes = nodes.checked_add(1).ok_or(MetadataError::Limit("nodes"))?;
        let common = super::source_v2_preparation::common_prefix_bytes(previous, path);
        for (index, _) in path.match_indices('/') {
            entry_depth += 1;
            if index >= common {
                nodes = nodes.checked_add(1).ok_or(MetadataError::Limit("nodes"))?;
            }
        }
        depth = depth.max(entry_depth);
        previous = path;
        if nodes > limits.max_nodes {
            return Err(MetadataError::Limit("nodes"));
        }
    }
    if nodes > limits.max_nodes {
        return Err(MetadataError::Limit("nodes"));
    }
    Ok((nodes, depth, path_bytes))
}

fn implicit_directory(expected: &[&FileEntry], path: &str) -> bool {
    // Compare against path + '/' without allocating; 'a-' sorts before 'a/'.
    let compare = |candidate: &str| {
        candidate
            .as_bytes()
            .iter()
            .copied()
            .cmp(path.as_bytes().iter().copied().chain(std::iter::once(b'/')))
    };
    let index = expected.partition_point(|entry| compare(entry_path(entry)).is_lt());
    expected.get(index).is_some_and(|entry| {
        entry_path(entry)
            .strip_prefix(path)
            .is_some_and(|rest| rest.starts_with('/'))
    })
}

fn entry_path(entry: &FileEntry) -> &str {
    match entry {
        FileEntry::File { path, .. } | FileEntry::Symlink { path, .. } => path.0.as_str(),
    }
}
fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

#[cfg(test)]
#[path = "application_erofs_metadata_tests.rs"]
mod tests;
