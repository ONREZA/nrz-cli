use std::collections::{HashMap, HashSet};

use crate::normalize_source_path;

// Linux limits a single path lookup to 40 symlink expansions, including revisits.
pub(crate) const MAX_SYMLINK_EXPANSIONS: usize = 40;

type ResolutionFrame<'a> = (usize, std::str::Split<'a, char>);
pub(crate) const SYMLINK_VALIDATION_SCRATCH_BYTES: usize =
    MAX_SYMLINK_EXPANSIONS * std::mem::size_of::<ResolutionFrame<'static>>();

#[derive(Debug, thiserror::Error)]
pub enum PathWorkError {
    #[error("path verification work budget exhausted")]
    Exhausted,
    #[error(transparent)]
    Invalid(#[from] SourcePathGraphError),
}

impl PathWorkError {
    fn unbudgeted(self) -> SourcePathGraphError {
        match self {
            Self::Invalid(error) => error,
            Self::Exhausted => unreachable!("unbudgeted path operation cannot exhaust fuel"),
        }
    }
}

/// One caller-owned allowance shared by index construction and every lookup.
#[derive(Debug)]
pub struct PathWorkBudget {
    remaining: Option<u64>,
}

impl PathWorkBudget {
    pub fn new(limit: u64) -> Self {
        Self {
            remaining: Some(limit),
        }
    }

    pub fn unlimited() -> Self {
        Self { remaining: None }
    }

    pub fn charge(&mut self, units: u64) -> Result<(), PathWorkError> {
        if let Some(remaining) = &mut self.remaining {
            *remaining = remaining
                .checked_sub(units)
                .ok_or(PathWorkError::Exhausted)?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourcePathGraphError {
    #[error("Invalid archive path: {0}")]
    InvalidPath(String),
    #[error("duplicate archive path: {0}")]
    DuplicatePath(String),
    #[error("Regular file contains nested archive entry: {0}")]
    FileAncestor(String),
    #[error("Symlink contains nested archive entry: {0}")]
    SymlinkAncestor(String),
    #[error("Unsafe symlink target: {0}")]
    UnsafeTarget(String),
    #[error("Symlink target is not included in archive: {0}")]
    MissingTarget(String),
    #[error("Symlink target contains a cycle: {0}")]
    Cycle(String),
}

/// The paths actually restored by an archive, including implicit parent directories.
#[derive(Debug)]
pub struct SourceArchivePathIndex<'a> {
    nodes: Vec<PathNode<'a>>,
    children: HashMap<(usize, &'a str), usize>,
    grammar: PathGrammar,
}

#[derive(Debug)]
struct PathNode<'a> {
    parent: Option<usize>,
    path: &'a str,
    target: Option<&'a str>,
    entry: bool,
    directory: bool,
}

pub(crate) const PATH_INDEX_NODE_BYTES: usize = std::mem::size_of::<PathNode<'static>>();
pub(crate) const PATH_INDEX_CHILD_BYTES: usize =
    std::mem::size_of::<((usize, &'static str), usize)>();

#[derive(Debug, Clone, Copy)]
enum PathGrammar {
    Portable,
    Unix,
}

impl<'a> SourceArchivePathIndex<'a> {
    /// Conservative borrowed-index and lookup scratch allowance, independent of
    /// the manifest strings it borrows. Includes simultaneous growth allocations.
    pub fn working_memory_bytes(max_nodes: u64, max_path_bytes: u64) -> Option<u64> {
        if PATH_INDEX_NODE_BYTES > 64 || PATH_INDEX_CHILD_BYTES > 32 {
            return None;
        }
        max_nodes
            .checked_add(1)?
            .checked_mul(512)?
            .checked_add(max_path_bytes.checked_mul(4)?)?
            .checked_add(SYMLINK_VALIDATION_SCRATCH_BYTES as u64)
    }

    pub fn from_entries(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    ) -> Result<Self, SourcePathGraphError> {
        Self::from_paths(
            entries,
            std::iter::empty(),
            PathGrammar::Portable,
            &mut PathWorkBudget::unlimited(),
        )
        .map_err(PathWorkError::unbudgeted)
    }

    pub fn from_entries_with_budget(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
        budget: &mut PathWorkBudget,
    ) -> Result<Self, PathWorkError> {
        Self::from_paths(entries, std::iter::empty(), PathGrammar::Portable, budget)
    }

    /// Index an actual Unix tree, including empty directories and Unix filenames.
    pub fn from_filesystem_entries(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
        directories: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, SourcePathGraphError> {
        Self::from_paths(
            entries,
            directories,
            PathGrammar::Unix,
            &mut PathWorkBudget::unlimited(),
        )
        .map_err(PathWorkError::unbudgeted)
    }

    fn from_paths(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
        directories: impl IntoIterator<Item = &'a str>,
        grammar: PathGrammar,
        budget: &mut PathWorkBudget,
    ) -> Result<Self, PathWorkError> {
        budget.charge(1)?;
        let mut index = Self {
            nodes: vec![PathNode {
                parent: None,
                path: "",
                target: None,
                entry: false,
                directory: true,
            }],
            children: HashMap::new(),
            grammar,
        };
        let mut indexed_key_bytes = 0;
        for path in directories {
            budget.charge((path.len() as u64).saturating_mul(5).saturating_add(1))?;
            index.validate_path(path)?;
            let node = index.insert_path(path, budget, &mut indexed_key_bytes)?;
            index.nodes[node].directory = true;
        }
        for (path, target) in entries {
            budget.charge((path.len() as u64).saturating_mul(5).saturating_add(1))?;
            index.validate_path(path)?;
            let node = index.insert_path(path, budget, &mut indexed_key_bytes)?;
            if index.nodes[node].entry {
                return Err(SourcePathGraphError::DuplicatePath(path.to_string()).into());
            }
            index.nodes[node].entry = true;
            index.nodes[node].target = target;
        }
        budget.charge(index.nodes.len() as u64)?;
        for node in &index.nodes {
            if node.directory && node.entry {
                return Err(if node.target.is_some() {
                    SourcePathGraphError::SymlinkAncestor(node.path.to_string())
                } else {
                    SourcePathGraphError::FileAncestor(node.path.to_string())
                }
                .into());
            }
        }
        Ok(index)
    }

    fn insert_path(
        &mut self,
        path: &'a str,
        budget: &mut PathWorkBudget,
        indexed_key_bytes: &mut u64,
    ) -> Result<usize, PathWorkError> {
        let mut parent = 0;
        let mut offset = 0;
        for component in path.split('/') {
            budget.charge((component.len() as u64).saturating_mul(2).saturating_add(1))?;
            if self.children.len() == self.children.capacity() {
                // Rehashing revisits existing key bytes, including long names.
                budget.charge(indexed_key_bytes.saturating_add(self.children.len() as u64))?;
                self.children.reserve(1);
            }
            if self.nodes.len() == self.nodes.capacity() {
                budget.charge(self.nodes.len() as u64)?;
                self.nodes.reserve(1);
            }
            self.nodes[parent].directory = true;
            offset += component.len();
            // Hash each component once; borrowed prefixes are only return values.
            parent = *self.children.entry((parent, component)).or_insert_with(|| {
                *indexed_key_bytes = indexed_key_bytes.saturating_add(component.len() as u64);
                let node = self.nodes.len();
                self.nodes.push(PathNode {
                    parent: Some(parent),
                    path: &path[..offset],
                    target: None,
                    entry: false,
                    directory: false,
                });
                node
            });
            offset += 1;
        }
        Ok(parent)
    }

    fn find_path(&self, path: &str) -> Option<usize> {
        let mut node = 0;
        for component in path.split('/') {
            node = *self.children.get(&(node, component))?;
        }
        Some(node)
    }

    pub fn validate_symlink(&self, path: &str) -> Result<(), SourcePathGraphError> {
        self.validate_symlink_with_budget(path, &mut PathWorkBudget::unlimited())
            .map_err(PathWorkError::unbudgeted)
    }

    pub(crate) fn validate_symlink_with_budget(
        &self,
        path: &str,
        budget: &mut PathWorkBudget,
    ) -> Result<(), PathWorkError> {
        self.walk_symlink(path, None, false, budget).map(|_| ())
    }

    /// Paths visited while resolving a declared symlink, in first-visit order.
    /// Includes intermediate directories even when a later `..` leaves them.
    /// The archive root is implicit and is never returned.
    pub fn required_paths(&self, path: &str) -> Result<Vec<&'a str>, SourcePathGraphError> {
        self.resolve_symlink(path).map(|(required, _)| required)
    }

    /// The physical archive path reached after following aliases before `..`.
    pub fn resolved_path(&self, path: &str) -> Result<String, SourcePathGraphError> {
        self.resolve_symlink(path).map(|(_, resolved)| resolved)
    }

    /// Resolve once when a caller needs both traversal custody and the endpoint.
    pub fn resolve_symlink(
        &self,
        path: &str,
    ) -> Result<(Vec<&'a str>, String), SourcePathGraphError> {
        self.resolve_symlink_with_budget(path, &mut PathWorkBudget::unlimited())
            .map_err(PathWorkError::unbudgeted)
    }

    /// Resolve while charging the same allowance used to construct the index.
    pub fn resolve_symlink_with_budget(
        &self,
        path: &str,
        budget: &mut PathWorkBudget,
    ) -> Result<(Vec<&'a str>, String), PathWorkError> {
        self.walk_symlink(path, None, true, budget)
            .map(|(required, resolved, _)| (required, resolved.to_string()))
    }

    /// Preparation needs the endpoint and actual expansions without custody-path allocations.
    pub(crate) fn resolved_endpoint_with_budget(
        &self,
        path: &str,
        budget: &mut PathWorkBudget,
    ) -> Result<(&'a str, usize), PathWorkError> {
        self.walk_symlink(path, None, false, budget)
            .map(|(_, resolved, expansions)| (resolved, expansions))
    }

    /// Resolve within a caller's physical namespace. Leaving `boundary` is an
    /// error even if a later component would reenter an indexed directory.
    pub fn resolve_symlink_within(
        &self,
        path: &str,
        boundary: &str,
    ) -> Result<(Vec<&'a str>, String), SourcePathGraphError> {
        if !path
            .strip_prefix(boundary)
            .is_some_and(|suffix| suffix.starts_with('/'))
        {
            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
        }
        let boundary = self
            .find_path(boundary)
            .ok_or_else(|| SourcePathGraphError::UnsafeTarget(path.to_string()))?;
        self.walk_symlink(path, Some(boundary), true, &mut PathWorkBudget::unlimited())
            .map(|(required, resolved, _)| (required, resolved.to_string()))
            .map_err(PathWorkError::unbudgeted)
    }

    fn walk_symlink(
        &self,
        path: &str,
        boundary: Option<usize>,
        collect_required: bool,
        budget: &mut PathWorkBudget,
    ) -> Result<(Vec<&'a str>, &'a str, usize), PathWorkError> {
        // Pay for splitting, component hashing and equality before find_path.
        budget.charge((path.len() as u64).saturating_mul(3).saturating_add(1))?;
        let Some(node) = self
            .find_path(path)
            .filter(|node| self.nodes[*node].target.is_some())
        else {
            return Err(SourcePathGraphError::MissingTarget(path.to_string()).into());
        };
        let path = self.nodes[node].path;
        let target = self.nodes[node]
            .target
            .expect("symlink target checked above");
        let mut required = Vec::new();
        let mut visited = HashSet::new();
        if collect_required {
            required.push(path);
            visited.insert(node);
        }
        let mut current = self.nodes[node]
            .parent
            .expect("archive entries have a parent");
        let mut expansions = 1;
        // Frames are also the active links; popping permits independent revisits.
        let mut frames: Vec<ResolutionFrame<'a>> = Vec::with_capacity(MAX_SYMLINK_EXPANSIONS);
        frames.push((node, self.checked_target(path, target, budget)?.split('/')));
        while let Some((_, remaining)) = frames.last_mut() {
            budget.charge(1)?;
            let Some(component) = remaining.next() else {
                frames.pop();
                continue;
            };
            if !self.nodes[current].directory {
                // Every component was admitted above; the only existing
                // non-directory at this point is a regular file.
                return Err(SourcePathGraphError::UnsafeTarget(path.to_string()).into());
            }
            match component {
                "" | "." => continue,
                ".." => {
                    if boundary == Some(current) {
                        return Err(SourcePathGraphError::UnsafeTarget(path.to_string()).into());
                    }
                    current = self.nodes[current]
                        .parent
                        .ok_or_else(|| SourcePathGraphError::UnsafeTarget(path.to_string()))?;
                    if collect_required && current != 0 && visited.insert(current) {
                        required.push(self.nodes[current].path);
                    }
                }
                name => {
                    budget.charge((name.len() as u64).saturating_mul(2).saturating_add(1))?;
                    let node = *self
                        .children
                        .get(&(current, name))
                        .ok_or_else(|| SourcePathGraphError::MissingTarget(path.to_string()))?;
                    if collect_required && visited.insert(node) {
                        required.push(self.nodes[node].path);
                    }
                    if let Some(target) = self.nodes[node].target {
                        expansions += 1;
                        if expansions > MAX_SYMLINK_EXPANSIONS {
                            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()).into());
                        }
                        budget.charge(frames.len() as u64)?;
                        if frames.iter().any(|(active, _)| *active == node) {
                            return Err(SourcePathGraphError::Cycle(path.to_string()).into());
                        }
                        frames.push((
                            node,
                            self.checked_target(self.nodes[node].path, target, budget)?
                                .split('/'),
                        ));
                    } else {
                        current = node;
                    }
                }
            }
        }
        if current == 0 {
            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()).into());
        }
        Ok((required, self.nodes[current].path, expansions))
    }

    fn validate_path(&self, path: &str) -> Result<(), SourcePathGraphError> {
        match self.grammar {
            PathGrammar::Portable => normalize_source_path(path)
                .map(|_| ())
                .map_err(SourcePathGraphError::InvalidPath),
            PathGrammar::Unix => {
                if path.contains('\0')
                    || path
                        .split('/')
                        .any(|part| part.is_empty() || part == "." || part == "..")
                {
                    return Err(SourcePathGraphError::InvalidPath(path.to_string()));
                }
                Ok(())
            }
        }
    }

    fn checked_target(
        &self,
        path: &str,
        target: &'a str,
        budget: &mut PathWorkBudget,
    ) -> Result<&'a str, PathWorkError> {
        // Includes both validation scans and the later lazy component scan.
        budget.charge((target.len() as u64).saturating_mul(3).saturating_add(1))?;
        if target.is_empty()
            || target.starts_with('/')
            || (matches!(self.grammar, PathGrammar::Portable) && target.contains('\\'))
            || target.contains('\0')
        {
            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()).into());
        }
        Ok(target)
    }
}

#[cfg(test)]
#[path = "path_graph_tests.rs"]
mod tests;
