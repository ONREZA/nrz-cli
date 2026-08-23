use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use nrz_fn_source::{
    MAX_FUNCTION_SOURCE_FILE_BYTES, MAX_FUNCTIONS_PER_PUBLISH, analyze_function_entry,
    function_name_from_entrypoint, is_function_entry_path,
};

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
    /// Stable platform function identity declared in config or derived from the branded file name.
    pub name: String,
    /// Entrypoint path relative to the project root.
    pub entrypoint: String,
    /// One-file source set keyed by `entrypoint`.
    pub sources: BTreeMap<String, String>,
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
    let mut seen_names = HashMap::new();
    for path in entries {
        let relative = relative_path(project_dir, &path);
        let size = path.metadata()?.len();
        if size > MAX_FUNCTION_SOURCE_FILE_BYTES {
            bail!("function source '{relative}' exceeds {MAX_FUNCTION_SOURCE_FILE_BYTES} bytes");
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("function source '{relative}' is not valid UTF-8 text"))?;
        let analysis = analyze_function_entry(&relative, &content)
            .with_context(|| format!("invalid ONREZA Function declaration in '{relative}'"))?;
        let name = match analysis.declaration.name.as_deref() {
            Some(name) => name.to_string(),
            None => function_name_from_entrypoint(&relative)
                .with_context(|| format!("invalid ONREZA Function entrypoint '{relative}'"))?
                .to_string(),
        };
        if let Some(previous_entrypoint) = seen_names.get(&name) {
            bail!(
                "duplicate ONREZA Function name '{name}' in '{relative}' and '{previous_entrypoint}'"
            );
        }
        seen_names.insert(name.clone(), relative.clone());

        if !analysis.imports.is_empty() {
            bail!(
                "function entry '{}' imports '{}'; ONREZA Functions v1 entry files must be self-contained",
                relative,
                analysis.imports.join("', '")
            );
        }
        if analysis.computed_dynamic_import {
            bail!("function entry '{relative}' uses computed dynamic import");
        }

        let mut sources = BTreeMap::new();
        sources.insert(relative.clone(), content);
        functions.push(CollectedFunction {
            name,
            entrypoint: relative,
            sources,
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

        if file_type.is_file() && is_function_entry_path(&name) {
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
