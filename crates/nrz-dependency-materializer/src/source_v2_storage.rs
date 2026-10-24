//! Request-specific storage upper bounds for the fixed 4 KiB EROFS recipes.
//!
//! These are capacity inputs, not a reservation or publication authority. The
//! caller adds its filesystem's per-inode overhead and retains the resulting
//! promise until the actual native workers and staging owner have closed.

use std::collections::BTreeMap;

use nrz_contract::source_bundle_v2::{FileEntry, FileRole};
use nrz_runtime_artifact::application_package_v1::application_package_layout;
use nrz_source_bundle::{
    dependency::dependency_source_root, source_bundle_v2::VerifiedSourceBundleV2Manifest,
};

use crate::source_v2_preparation::{common_prefix_bytes, entry_path};

const BLOCK: u64 = 4096;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceV2StoragePlan {
    /// Block-rounded source snapshot, including symlink contents.
    pub source_bytes: u64,
    pub source_nodes: u64,
    /// All source/scratch copies and retained images; excludes filesystem overhead.
    pub staging_bytes: u64,
    pub staging_nodes: u64,
    /// Static, Compute. Zero means that role is absent.
    pub application_package_bytes: [u64; 2],
    pub dependency_image_bytes: u64,
    pub dependency_count: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid source storage plan: {0}")]
pub struct SourceV2StorageError(&'static str);

fn add(left: u64, right: u64) -> Result<u64, SourceV2StorageError> {
    left.checked_add(right)
        .ok_or(SourceV2StorageError("byte/node overflow"))
}
fn mul(left: u64, right: u64) -> Result<u64, SourceV2StorageError> {
    left.checked_mul(right)
        .ok_or(SourceV2StorageError("byte/node overflow"))
}
fn rounded(bytes: u64) -> Result<u64, SourceV2StorageError> {
    mul(bytes.div_ceil(BLOCK), BLOCK)
}

#[derive(Default)]
struct Tree<'a> {
    previous: &'a str,
    nodes: u64,
    bytes: u64,
    erofs: u64,
}
impl<'a> Tree<'a> {
    fn push(&mut self, entry: &'a FileEntry) -> Result<(), SourceV2StorageError> {
        let path = entry_path(entry);
        if path.split('/').any(|component| component.len() > 255) {
            return Err(SourceV2StorageError("EROFS component byte limit"));
        }
        if self.nodes == 0 {
            self.nodes = 1;
            // Root inode, dot/dotdot directory records, superblock/config.
            self.erofs = 5 * BLOCK;
        }
        let common = common_prefix_bytes(self.previous, path);
        let directories = path
            .match_indices('/')
            .filter(|(i, _)| *i >= common)
            .count() as u64;
        self.nodes = add(self.nodes, add(directories, 1)?)?;
        // One block per directory entry covers its 12-byte record, <=255-byte
        // component and any packing gap. Each new directory also has an inode
        // and dot/dotdot. No xattrs or hardlinks are emitted by this recipe.
        self.erofs = add(self.erofs, mul(directories, 4 * BLOCK)?)?;
        self.erofs = add(self.erofs, BLOCK)?;
        let (data, metadata) = match entry {
            FileEntry::File { size, .. } => {
                let data = rounded(*size)?;
                self.bytes = add(self.bytes, data)?;
                // erofs-utils 1.9.4 compress.c: a non-saving compressed file
                // falls back to plain data. Count BOTH the attempted output and
                // fallback, so correctness does not depend on tail rollback.
                // The largest extent record is 32 bytes; include its alignment,
                // map header, extended inode and a final cluster record.
                let index = mul(add(size.div_ceil(BLOCK), 1)?, 32)?;
                (mul(data, 2)?, rounded(add(64 + 32 + 8, index)?)?)
            }
            FileEntry::Symlink { link_target, .. } => {
                let data = rounded(link_target.0.len() as u64)?;
                self.bytes = add(self.bytes, data)?;
                (data, BLOCK)
            }
        };
        self.erofs = add(self.erofs, add(data, metadata)?)?;
        self.previous = path;
        Ok(())
    }
}

pub(crate) fn application_package_bounds(
    files: &[FileEntry],
) -> Result<[u64; 2], SourceV2StorageError> {
    let mut roles = [Tree::default(), Tree::default()];
    for entry in files {
        let role = match entry {
            FileEntry::File { role, .. } | FileEntry::Symlink { role, .. } => role,
        };
        match role {
            FileRole::Static | FileRole::Prerender | FileRole::Config => roles[0].push(entry)?,
            FileRole::Compute => roles[1].push(entry)?,
            FileRole::Dependency => {}
        }
    }
    let mut bounds = [0; 2];
    for (index, tree) in roles.iter().enumerate().filter(|(_, tree)| tree.nodes != 0) {
        bounds[index] = package_bound(tree)?;
    }
    Ok(bounds)
}

fn package_bound(tree: &Tree<'_>) -> Result<u64, SourceV2StorageError> {
    application_package_layout(tree.erofs)
        .map(|layout| layout.package_size)
        .map_err(|_| SourceV2StorageError("package geometry"))
}

/// Uses the verified, sorted inventory, never an upload's declared total alone.
/// Bounds compression without a compression ratio, and uses the canonical
/// verity geometry. Global package/tree limits remain independent policy caps.
pub fn source_v2_storage_plan(
    source: &VerifiedSourceBundleV2Manifest,
) -> Result<SourceV2StoragePlan, SourceV2StorageError> {
    let mut all = Tree::default();
    let mut roles = [Tree::default(), Tree::default()];
    let mut dependencies = BTreeMap::<&str, Tree<'_>>::new();
    for entry in &source.wire().files {
        all.push(entry)?;
        let role = match entry {
            FileEntry::File { role, .. } | FileEntry::Symlink { role, .. } => role,
        };
        match role {
            FileRole::Static | FileRole::Prerender | FileRole::Config => roles[0].push(entry)?,
            FileRole::Compute => roles[1].push(entry)?,
            FileRole::Dependency => {
                let root = dependency_source_root(entry_path(entry))
                    .ok_or(SourceV2StorageError("dependency root"))?;
                dependencies.entry(root).or_default().push(entry)?;
            }
        }
    }
    let mut plan = SourceV2StoragePlan {
        source_bytes: all.bytes,
        source_nodes: add(all.nodes, 1)?,
        // Snapshot plus the copied role tree and verified extraction. Summing
        // both roles is conservative: the actual scratch owner runs sequentially.
        staging_bytes: all.bytes,
        staging_nodes: add(all.nodes, 1)?,
        ..SourceV2StoragePlan::default()
    };
    for (index, tree) in roles.iter().enumerate().filter(|(_, tree)| tree.nodes != 0) {
        let package = package_bound(tree)?;
        plan.application_package_bytes[index] = package;
        plan.staging_bytes = add(plan.staging_bytes, add(mul(tree.bytes, 2)?, package)?)?;
        // Role scratch, tree, extraction tree, package and package container.
        plan.staging_nodes = add(plan.staging_nodes, add(mul(tree.nodes, 2)?, 3)?)?;
    }
    for tree in dependencies.values() {
        plan.dependency_image_bytes = add(plan.dependency_image_bytes, tree.erofs)?;
        plan.dependency_count = add(plan.dependency_count, 1)?;
        // Dependency normalization uses one copied tree; fsck verifies the
        // image without an extraction directory. Image links share one inode.
        plan.staging_bytes = add(plan.staging_bytes, add(tree.bytes, tree.erofs)?)?;
        plan.staging_nodes = add(plan.staging_nodes, add(tree.nodes, 3)?)?;
    }
    // Dependency stage, trees and images containers (also covers no-dep case).
    plan.staging_nodes = add(plan.staging_nodes, 3)?;
    Ok(plan)
}

#[cfg(test)]
#[path = "source_v2_storage_tests.rs"]
mod tests;
