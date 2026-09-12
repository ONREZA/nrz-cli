use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use nrz_api::functions::{MAX_FUNCTION_SOURCE_FILE_BYTES, MAX_FUNCTIONS_PER_PUBLISH};
const ENTRY_SUFFIXES: &[&str] = &[
    ".nrz-fn.ts",
    ".nrz-fn.tsx",
    ".nrz-fn.js",
    ".nrz-fn.jsx",
    ".nrz-fn.mjs",
];

/// Directory names that are never part of function discovery.
const DENIED_DIR_NAMES: &[&str] = &[
    ".git",
    ".onreza",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    "vendor",
];

/// All ONREZA Functions discovered under the project root.
#[derive(Debug, Default)]
pub struct CollectedFunctions {
    pub functions: Vec<CollectedFunction>,
}

impl CollectedFunctions {
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    pub fn source_file_count(&self) -> usize {
        self.functions
            .iter()
            .map(|function| function.sources.len())
            .sum()
    }
}

/// A single function entry ready for native-runtime preflight and publishing.
#[derive(Debug)]
pub struct CollectedFunction {
    /// Filename-derived candidate; native preflight resolves and validates the declared identity.
    pub name: String,
    /// Entrypoint path relative to the project root.
    pub entrypoint: String,
    /// One-file source set keyed by `entrypoint`.
    pub sources: BTreeMap<String, String>,
    pub(crate) inspected: Option<nrz_api::FunctionPublishSpec>,
}

/// Discover ONREZA Functions under the project root by branded file suffix.
pub fn collect(project_dir: &Path) -> anyhow::Result<CollectedFunctions> {
    if !project_dir.is_dir() {
        return Ok(CollectedFunctions::default());
    }

    let mut entries = Vec::new();
    walk_entries(project_dir, &mut entries)?;
    entries.sort();

    if entries.len() > MAX_FUNCTIONS_PER_PUBLISH {
        bail!("ONREZA Functions discovery found more than {MAX_FUNCTIONS_PER_PUBLISH} entry files");
    }

    let mut functions = Vec::with_capacity(entries.len());
    for path in entries {
        let relative = relative_path(project_dir, &path);
        let mut content = String::new();
        std::fs::File::open(&path)?
            .take(MAX_FUNCTION_SOURCE_FILE_BYTES + 1)
            .read_to_string(&mut content)
            .with_context(|| format!("function source '{relative}' is not valid UTF-8 text"))?;
        if content.len() as u64 > MAX_FUNCTION_SOURCE_FILE_BYTES {
            bail!("function source '{relative}' exceeds {MAX_FUNCTION_SOURCE_FILE_BYTES} bytes");
        }
        let name = name_from_entrypoint(&relative).to_string();

        let mut sources = BTreeMap::new();
        sources.insert(relative.clone(), content);
        functions.push(CollectedFunction {
            name,
            entrypoint: relative,
            sources,
            inspected: None,
        });
    }

    Ok(CollectedFunctions { functions })
}

fn walk_entries(dir: &Path, entries: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let dir_entries =
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?;
    for entry in dir_entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            if is_denied_dir_name(&name) || name.starts_with('.') {
                continue;
            }
            walk_entries(&path, entries)?;
            continue;
        }

        if file_type.is_file() && ENTRY_SUFFIXES.iter().any(|suffix| name.ends_with(suffix)) {
            entries.push(path);
        }
    }
    Ok(())
}

fn is_denied_dir_name(name: &str) -> bool {
    DENIED_DIR_NAMES.contains(&name)
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("walked path is under source root")
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn name_from_entrypoint(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    ENTRY_SUFFIXES
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(name)
}
