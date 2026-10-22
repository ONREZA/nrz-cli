//! Owned Static visibility prepared from an admitted manifest, without serving authority.
//! The caller retains the projection with its exact component/generation capability.
//! Explicit caller clones, request normalization and process overhead need their own budget.

use std::{collections::HashMap, sync::Arc};

use nrz_contract::source_bundle_v2::{BlobDigest, FileEntry, FileRole};

use super::admission::{AdmissionError, AdmittedSourceBundleV2Manifest, MemoryReservation};
use crate::{PathWorkBudget, PathWorkError, SourceArchivePathIndex};

#[path = "source_bundle_v2_static_memory.rs"]
mod memory;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StaticProjectionError {
    #[error("Static projection memory capacity unavailable (requested {requested_bytes} bytes)")]
    MemoryUnavailable { requested_bytes: u64 },
    #[error("Static projection memory estimate overflows or uses an unsupported layout")]
    UnsupportedEstimate,
    #[error("Static projection allocation failed")]
    AllocationFailure,
    #[error("Static projection path work budget exhausted")]
    PathWorkLimit,
    #[error("Static lookup requires a normalized relative path")]
    InvalidRequestPath,
    #[error("Static preparation conflicts with its admitted manifest")]
    InconsistentManifest,
}

impl From<PathWorkError> for StaticProjectionError {
    fn from(error: PathWorkError) -> Self {
        match error {
            PathWorkError::Exhausted => Self::PathWorkLimit,
            PathWorkError::Invalid(_) => Self::InconsistentManifest,
        }
    }
}

impl From<AdmissionError> for StaticProjectionError {
    fn from(error: AdmissionError) -> Self {
        match error {
            AdmissionError::MemoryUnavailable { requested_bytes } => {
                Self::MemoryUnavailable { requested_bytes }
            }
            _ => Self::InconsistentManifest,
        }
    }
}

#[derive(Debug, Default)]
struct Node {
    children: HashMap<Box<str>, usize>,
    key_bytes: u64,
    file: Option<usize>,
    alias: Option<(usize, usize)>,
}

#[derive(Debug)]
struct Target {
    path: Arc<str>,
    destination: Option<usize>,
}

/// Borrowed getters cannot detach a record from its projection's memory permit.
#[derive(Debug)]
pub struct StaticFileMetadata {
    canonical_path: Arc<str>,
    blob_digest: BlobDigest,
    size: u64,
    content_type: Option<String>,
}

impl StaticFileMetadata {
    pub fn canonical_path(&self) -> &str {
        &self.canonical_path
    }
    pub fn blob_digest(&self) -> &BlobDigest {
        &self.blob_digest
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }
}

/// Public regular files and every normalized alias; the full manifest is not retained.
/// This is metadata evidence, not a mounted generation or an execution grant.
#[derive(Debug)]
pub struct PreparedStaticProjection {
    manifest_blake3: String,
    nodes: Vec<Node>,
    targets: Vec<Target>,
    files: Vec<StaticFileMetadata>,
    // Free all owned data before returning its allowance to the shared input pool.
    reservation: MemoryReservation,
}

impl PreparedStaticProjection {
    pub fn manifest_blake3(&self) -> &str {
        &self.manifest_blake3
    }
    pub fn reserved_bytes(&self) -> u64 {
        self.reservation.bytes()
    }

    /// No allocation, including misses, invalid paths and fuel refusal. Normalization belongs to the caller.
    /// ```compile_fail,E0505
    /// use nrz_source_bundle::{PathWorkBudget, source_bundle_v2::static_projection::PreparedStaticProjection};
    /// fn detach(projection: PreparedStaticProjection) {
    ///     let mut work = PathWorkBudget::unlimited();
    ///     let file = projection.lookup("file", &mut work).unwrap().unwrap();
    ///     drop(projection);
    ///     let _ = file.canonical_path();
    /// }
    /// ```
    pub fn lookup(
        &self,
        normalized_path: &str,
        work: &mut PathWorkBudget,
    ) -> Result<Option<&StaticFileMetadata>, StaticProjectionError> {
        work.charge(
            (normalized_path.len() as u64)
                .checked_mul(4)
                .and_then(|n| n.checked_add(1))
                .ok_or(StaticProjectionError::PathWorkLimit)?,
        )?;
        if normalized_path.contains(['\0', '\\'])
            || normalized_path
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(StaticProjectionError::InvalidRequestPath);
        }
        let mut node = 0;
        let mut expansions = 0;
        for component in normalized_path.split('/') {
            let Some(current) = self.jump(node, &mut expansions, work)? else {
                return Ok(None);
            };
            if self.nodes[current].file.is_some() {
                return Ok(None);
            }
            // The initial debit covers request scans; debit hashing/equality before each table lookup.
            work.charge((component.len() as u64).saturating_mul(2).saturating_add(1))?;
            let Some(child) = self.nodes[current].children.get(component) else {
                return Ok(None);
            };
            node = *child;
        }
        Ok(self
            .jump(node, &mut expansions, work)?
            .and_then(|node| self.nodes[node].file)
            .map(|file| &self.files[file]))
    }

    fn jump(
        &self,
        node: usize,
        expansions: &mut usize,
        work: &mut PathWorkBudget,
    ) -> Result<Option<usize>, StaticProjectionError> {
        let Some((target, count)) = self.nodes[node].alias else {
            return Ok(Some(node));
        };
        work.charge(1)?;
        *expansions += count;
        if *expansions > 40 {
            return Ok(None);
        }
        Ok(self.targets[target].destination)
    }
}

struct Alias<'a> {
    path: &'a str,
    target: usize,
    expansions: usize,
}

pub(super) fn prepare(
    input: &AdmittedSourceBundleV2Manifest,
    max_path_work: u64,
) -> Result<PreparedStaticProjection, StaticProjectionError> {
    let mut work = PathWorkBudget::new(max_path_work);
    let source = &input.verified().wire().files;
    let counts = memory::Counts::inspect(source, &mut work)?;
    let _scratch = input.reserve_memory(counts.scratch_bytes()?)?;
    let bound = usize::try_from(counts.retained_entries()?)
        .map_err(|_| StaticProjectionError::UnsupportedEstimate)?;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(bound)
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    selected.extend(
        source
            .iter()
            .filter(|entry| retained(entry))
            .map(entry_path),
    );
    let index = SourceArchivePathIndex::from_entries_with_budget(
        source.iter().map(|entry| match entry {
            FileEntry::File { path, .. } => (path.0.as_str(), None),
            FileEntry::Symlink {
                path, link_target, ..
            } => (path.0.as_str(), Some(link_target.0.as_str())),
        }),
        &mut work,
    )?;
    let mut paths = Vec::new();
    paths
        .try_reserve_exact(bound)
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    let mut interned = HashMap::new();
    interned
        .try_reserve(bound)
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    let mut aliases = Vec::new();
    aliases
        .try_reserve_exact(
            usize::try_from(counts.aliases)
                .map_err(|_| StaticProjectionError::UnsupportedEstimate)?,
        )
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    let mut target_bytes = 0_u64;
    for entry in source {
        if matches!(entry, FileEntry::File { role, .. } if public(role)) {
            intern(
                entry_path(entry),
                &mut paths,
                &mut interned,
                &mut target_bytes,
                &mut work,
            )?;
        }
    }
    for entry in source {
        if matches!(entry, FileEntry::Symlink { .. }) {
            let path = entry_path(entry);
            let (endpoint, expansions) = index.resolved_endpoint_with_budget(path, &mut work)?;
            let target = intern(
                endpoint,
                &mut paths,
                &mut interned,
                &mut target_bytes,
                &mut work,
            )?;
            aliases.push(Alias {
                path,
                target,
                expansions,
            });
        }
    }
    let reservation =
        input.reserve_memory(counts.owned_bytes(paths.len() as u64, target_bytes)?)?;
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(
            usize::try_from(counts.selected_nodes)
                .map_err(|_| StaticProjectionError::UnsupportedEstimate)?,
        )
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    nodes.push(Node::default());
    for path in &selected {
        insert(&mut nodes, path, &mut work)?;
    }
    let mut targets = Vec::new();
    targets
        .try_reserve_exact(paths.len())
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    for endpoint in paths {
        // Prove deadness from the complete retained path set, independently of the trie lookup.
        work.charge(
            (endpoint.len() as u64)
                .checked_add(1)
                .and_then(|n| {
                    n.checked_mul((2 * selected.len().checked_ilog2().unwrap_or(0) + 8) as u64)
                })
                .ok_or(StaticProjectionError::PathWorkLimit)?,
        )?;
        let position = selected.partition_point(|path| {
            path.bytes()
                .cmp(endpoint.bytes().chain(std::iter::once(b'/')))
                .is_lt()
        });
        let live = selected.binary_search(&endpoint).is_ok()
            || selected.get(position).is_some_and(|path| {
                path.strip_prefix(endpoint)
                    .is_some_and(|suffix| suffix.starts_with('/'))
            });
        let destination = find(&nodes, endpoint, &mut work)?;
        if destination.is_some() != live {
            return Err(StaticProjectionError::InconsistentManifest);
        }
        targets.push(Target {
            path: Arc::from(endpoint),
            destination,
        });
    }
    let mut files = Vec::new();
    files
        .try_reserve_exact(
            usize::try_from(counts.public_files)
                .map_err(|_| StaticProjectionError::UnsupportedEstimate)?,
        )
        .map_err(|_| StaticProjectionError::AllocationFailure)?;
    for entry in source {
        if let FileEntry::File {
            path,
            role,
            blob_digest,
            size,
            content_type,
            ..
        } = entry
            && public(role)
        {
            work.charge((path.0.len() as u64).saturating_mul(3).saturating_add(1))?;
            let node = find(&nodes, &path.0, &mut work)?
                .ok_or(StaticProjectionError::InconsistentManifest)?;
            nodes[node].file = Some(files.len());
            files.push(StaticFileMetadata {
                canonical_path: Arc::clone(&targets[interned[path.0.as_str()]].path),
                blob_digest: blob_digest.clone(),
                size: *size,
                content_type: content_type.clone(),
            });
        }
    }
    for alias in aliases {
        let node = find(&nodes, alias.path, &mut work)?
            .ok_or(StaticProjectionError::InconsistentManifest)?;
        nodes[node].alias = Some((alias.target, alias.expansions));
    }
    if targets
        .iter()
        .filter_map(|target| target.destination)
        .any(|node| nodes[node].alias.is_some())
    {
        return Err(StaticProjectionError::InconsistentManifest);
    }
    work.charge(64)?;
    Ok(PreparedStaticProjection {
        manifest_blake3: input.verified().manifest_blake3().to_owned(),
        nodes,
        targets,
        files,
        reservation,
    })
}

fn intern<'a>(
    path: &'a str,
    paths: &mut Vec<&'a str>,
    interned: &mut HashMap<&'a str, usize>,
    bytes: &mut u64,
    work: &mut PathWorkBudget,
) -> Result<usize, StaticProjectionError> {
    work.charge((path.len() as u64).saturating_mul(3).saturating_add(1))?;
    if let Some(number) = interned.get(path) {
        return Ok(*number);
    }
    *bytes = bytes
        .checked_add(path.len() as u64)
        .ok_or(StaticProjectionError::UnsupportedEstimate)?;
    let number = paths.len();
    paths.push(path);
    interned.insert(path, number);
    Ok(number)
}

fn insert(
    nodes: &mut Vec<Node>,
    path: &str,
    work: &mut PathWorkBudget,
) -> Result<(), StaticProjectionError> {
    let mut node = 0;
    for component in path.split('/') {
        work.charge((component.len() as u64).saturating_mul(4).saturating_add(1))?;
        node = if let Some(child) = nodes[node].children.get(component) {
            *child
        } else {
            if nodes[node].children.len() == nodes[node].children.capacity() {
                work.charge(
                    nodes[node]
                        .key_bytes
                        .saturating_add(nodes[node].children.len() as u64),
                )?;
            }
            nodes[node]
                .children
                .try_reserve(1)
                .map_err(|_| StaticProjectionError::AllocationFailure)?;
            let child = nodes.len();
            nodes.push(Node::default());
            nodes[node].key_bytes += component.len() as u64;
            nodes[node].children.insert(component.into(), child);
            child
        };
    }
    Ok(())
}

fn find(
    nodes: &[Node],
    path: &str,
    work: &mut PathWorkBudget,
) -> Result<Option<usize>, StaticProjectionError> {
    work.charge((path.len() as u64).saturating_mul(3).saturating_add(1))?;
    let mut node = 0;
    for component in path.split('/') {
        let Some(child) = nodes[node].children.get(component) else {
            return Ok(None);
        };
        node = *child;
    }
    Ok(Some(node))
}

fn entry_path(entry: &FileEntry) -> &str {
    match entry {
        FileEntry::File { path, .. } | FileEntry::Symlink { path, .. } => &path.0,
    }
}
fn public(role: &FileRole) -> bool {
    matches!(
        role,
        FileRole::Static | FileRole::Prerender | FileRole::Config
    )
}
fn retained(entry: &FileEntry) -> bool {
    matches!(entry, FileEntry::Symlink { .. })
        || matches!(entry, FileEntry::File { role, .. } if public(role))
}

#[cfg(test)]
#[path = "source_bundle_v2_static_tests.rs"]
mod tests;
