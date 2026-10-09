use std::collections::{HashMap, HashSet};

use crate::normalize_source_path;

// Linux limits a single path lookup to 40 symlink expansions, including revisits.
const MAX_SYMLINK_EXPANSIONS: usize = 40;

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
    entries: HashMap<&'a str, Option<&'a str>>,
    directories: HashSet<&'a str>,
    grammar: PathGrammar,
}

#[derive(Debug, Clone, Copy)]
enum PathGrammar {
    Portable,
    Unix,
}

impl<'a> SourceArchivePathIndex<'a> {
    pub fn from_entries(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    ) -> Result<Self, SourcePathGraphError> {
        Self::from_paths(entries, std::iter::empty(), PathGrammar::Portable)
    }

    /// Index an actual Unix tree, including empty directories and Unix filenames.
    pub fn from_filesystem_entries(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
        directories: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, SourcePathGraphError> {
        Self::from_paths(entries, directories, PathGrammar::Unix)
    }

    fn from_paths(
        entries: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
        directories: impl IntoIterator<Item = &'a str>,
        grammar: PathGrammar,
    ) -> Result<Self, SourcePathGraphError> {
        let mut index = Self {
            entries: HashMap::new(),
            directories: HashSet::from([""]),
            grammar,
        };
        for path in directories {
            index.validate_path(path)?;
            index.directories.insert(path);
            for (offset, _) in path.match_indices('/') {
                index.directories.insert(&path[..offset]);
            }
        }
        for (path, target) in entries {
            index.validate_path(path)?;
            if index.entries.insert(path, target).is_some() {
                return Err(SourcePathGraphError::DuplicatePath(path.to_string()));
            }
            for (offset, _) in path.match_indices('/') {
                index.directories.insert(&path[..offset]);
            }
        }
        for path in &index.directories {
            if let Some(target) = index.entries.get(path) {
                return Err(if target.is_some() {
                    SourcePathGraphError::SymlinkAncestor(path.to_string())
                } else {
                    SourcePathGraphError::FileAncestor(path.to_string())
                });
            }
        }
        Ok(index)
    }

    pub fn validate_symlink(&self, path: &str) -> Result<(), SourcePathGraphError> {
        self.required_paths(path).map(|_| ())
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
        self.walk_symlink(path, None)
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
        self.walk_symlink(path, Some(boundary))
    }

    fn walk_symlink(
        &self,
        path: &str,
        boundary: Option<&str>,
    ) -> Result<(Vec<&'a str>, String), SourcePathGraphError> {
        let Some((&path, &Some(target))) = self.entries.get_key_value(path) else {
            return Err(SourcePathGraphError::MissingTarget(path.to_string()));
        };
        let mut required = vec![path];
        let mut visited = HashSet::from([path]);
        let mut components: Vec<&str> = path
            .rsplit_once('/')
            .map_or_else(Vec::new, |(parent, _)| parent.split('/').collect());
        let mut active = HashSet::from([path]);
        let mut expansions = 1;
        // Each frame retains the caller's unresolved suffix. A completed expansion
        // leaves the active set, allowing a later, independent visit to the alias.
        let mut frames = vec![(path, self.checked_target(path, target)?.split('/'))];
        while let Some((link, remaining)) = frames.last_mut() {
            let Some(component) = remaining.next() else {
                active.remove(link);
                frames.pop();
                continue;
            };
            let current = components.join("/");
            if !self.directories.contains(current.as_str()) {
                // Every component was admitted above; the only existing
                // non-directory at this point is a regular file.
                return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
            }
            match component {
                "" | "." => continue,
                ".." => {
                    if boundary == Some(current.as_str()) {
                        return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
                    }
                    if components.pop().is_none() {
                        return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
                    }
                    let parent = components.join("/");
                    if let Some(&directory) = self.directories.get(parent.as_str())
                        && !directory.is_empty()
                        && visited.insert(directory)
                    {
                        required.push(directory);
                    }
                }
                name => {
                    components.push(name);
                    let current = components.join("/");
                    let node = self
                        .entries
                        .get_key_value(current.as_str())
                        .map(|(&path, _)| path)
                        .or_else(|| self.directories.get(current.as_str()).copied())
                        .ok_or_else(|| SourcePathGraphError::MissingTarget(path.to_string()))?;
                    if visited.insert(node) {
                        required.push(node);
                    }
                    if let Some((&alias, &Some(target))) =
                        self.entries.get_key_value(current.as_str())
                    {
                        expansions += 1;
                        if expansions > MAX_SYMLINK_EXPANSIONS {
                            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
                        }
                        if !active.insert(alias) {
                            return Err(SourcePathGraphError::Cycle(path.to_string()));
                        }
                        components.pop();
                        frames.push((alias, self.checked_target(alias, target)?.split('/')));
                    }
                }
            }
        }
        if components.is_empty() {
            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
        }
        Ok((required, components.join("/")))
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

    fn checked_target(&self, path: &str, target: &'a str) -> Result<&'a str, SourcePathGraphError> {
        if target.is_empty()
            || target.starts_with('/')
            || (matches!(self.grammar, PathGrammar::Portable) && target.contains('\\'))
            || target.contains('\0')
        {
            return Err(SourcePathGraphError::UnsafeTarget(path.to_string()));
        }
        Ok(target)
    }
}

#[cfg(test)]
#[path = "path_graph_tests.rs"]
mod tests;
