//! Conservative requested-heap bounds for the pinned Rust/standard-library layout.
//! Allocator arenas, stacks and caller copies require process/cgroup headroom.

use std::mem::size_of;

use nrz_contract::source_bundle_v2::FileEntry;

use super::{
    Node, StaticFileMetadata, StaticProjectionError, Target, entry_path, public, retained,
};
use crate::PathWorkBudget;

#[derive(Default)]
pub(super) struct Counts {
    pub selected_nodes: u64,
    pub public_files: u64,
    pub aliases: u64,
    full_nodes: u64,
    selected_labels: u64,
    public_content_bytes: u64,
    max_path_bytes: u64,
}

impl Counts {
    pub fn inspect(
        files: &[FileEntry],
        work: &mut PathWorkBudget,
    ) -> Result<Self, StaticProjectionError> {
        if size_of::<usize>() != 8
            || size_of::<Node>() > 128
            || size_of::<StaticFileMetadata>() > 128
            || size_of::<Target>() > 48
        {
            return Err(StaticProjectionError::UnsupportedEstimate);
        }
        let mut counts = Self {
            full_nodes: 1,
            selected_nodes: 1,
            ..Self::default()
        };
        let mut previous = "";
        let mut previous_selected = "";
        for entry in files {
            let path = entry_path(entry);
            work.charge(
                (path.len() as u64)
                    .checked_mul(12)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(StaticProjectionError::PathWorkLimit)?,
            )?;
            let shared = common_components(previous, path);
            add(
                &mut counts.full_nodes,
                path.split('/').skip(shared).count() as u64,
            )?;
            previous = path;
            counts.max_path_bytes = counts.max_path_bytes.max(path.len() as u64);
            match entry {
                FileEntry::File {
                    role,
                    content_type,
                    blob_digest,
                    ..
                } if public(role) => {
                    add(&mut counts.public_files, 1)?;
                    let content_bytes = content_type.as_ref().map_or(0, String::len) as u64;
                    work.charge(
                        content_bytes
                            .checked_add(blob_digest.value.len() as u64)
                            .and_then(|n| n.checked_mul(3))
                            .ok_or(StaticProjectionError::PathWorkLimit)?,
                    )?;
                    add(&mut counts.public_content_bytes, content_bytes)?;
                }
                FileEntry::Symlink { .. } => add(&mut counts.aliases, 1)?,
                _ => {}
            }
            if retained(entry) {
                let shared = common_components(previous_selected, path);
                for component in path.split('/').skip(shared) {
                    add(&mut counts.selected_nodes, 1)?;
                    add(&mut counts.selected_labels, component.len() as u64)?;
                }
                previous_selected = path;
            }
        }
        Ok(counts)
    }

    pub fn retained_entries(&self) -> Result<u64, StaticProjectionError> {
        checked(|| self.public_files.checked_add(self.aliases))
    }

    pub fn scratch_bytes(&self) -> Result<u64, StaticProjectionError> {
        // Full borrowed path index: old/new node Vec and hash tables during growth.
        // Borrowed selected/endpoints/cache Vecs + interner include capacity/control bytes.
        checked(|| {
            self.full_nodes
                .checked_mul(1024)?
                .checked_add(self.retained_entries().ok()?.checked_mul(512)?)?
                .checked_add(self.max_path_bytes.checked_mul(24)?)?
                .checked_add(crate::path_graph::SYMLINK_VALIDATION_SCRATCH_BYTES as u64)?
                .checked_add(64 * 1024)
        })
    }

    pub fn owned_bytes(
        &self,
        targets: u64,
        target_bytes: u64,
    ) -> Result<u64, StaticProjectionError> {
        // Each edge owns one component label. Per-node table allowance covers small
        // tables, bucket/control capacity and simultaneous old/new growth allocations.
        // Arc headers/alignment, metadata Vecs, String cloning and manifest identity
        // are independent of the admitted manifest's verification-peak allowance.
        checked(|| {
            self.selected_nodes
                .checked_mul(size_of::<Node>() as u64 + 512)?
                .checked_add(self.selected_labels.checked_mul(3)?)?
                .checked_add(targets.checked_mul(size_of::<Target>() as u64 + 64)?)?
                .checked_add(target_bytes.checked_mul(3)?)?
                .checked_add(
                    self.public_files
                        .checked_mul(size_of::<StaticFileMetadata>() as u64 + 192)?,
                )?
                .checked_add(self.public_content_bytes.checked_mul(3)?)?
                .checked_add(16 * 1024)
        })
    }
}

fn common_components(left: &str, right: &str) -> usize {
    left.split('/')
        .zip(right.split('/'))
        .take_while(|(a, b)| a == b)
        .count()
}

fn add(counter: &mut u64, value: u64) -> Result<(), StaticProjectionError> {
    *counter = counter
        .checked_add(value)
        .ok_or(StaticProjectionError::UnsupportedEstimate)?;
    Ok(())
}

fn checked(estimate: impl FnOnce() -> Option<u64>) -> Result<u64, StaticProjectionError> {
    estimate().ok_or(StaticProjectionError::UnsupportedEstimate)
}
