use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use nrz_fn_policy::{SourceSet, analyze_function_entry};

/// Function entry suffixes recognized by discovery. The suffix is intentionally
/// not configurable so discovery never needs to scan generic `*.ts` files.
const FUNCTION_ENTRY_SUFFIXES: &[&str] = &[
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

/// Public functions-publish contract caps. Keep in sync with
/// `FunctionPublishPayloadSchema` and artifact-ingest validation.
const MAX_FUNCTIONS: usize = 1000;
const MAX_FUNCTION_FILE_BYTES: u64 = 128 * 1024;
const MAX_FUNCTION_NAME_LENGTH: usize = 64;

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

/// A single function entry, ready for policy preview and publish payload.
#[derive(Debug)]
pub struct CollectedFunction {
    /// Stable platform function identity declared in config or derived from the branded file name.
    pub name: String,
    /// Entrypoint path relative to the project root.
    pub entrypoint: String,
    /// One-file source set keyed by `entrypoint`.
    pub sources: SourceSet,
}

/// Discover ONREZA Functions under the project root by branded file suffix.
pub fn collect(project_dir: &Path) -> anyhow::Result<CollectedFunctions> {
    if !project_dir.is_dir() {
        return Ok(CollectedFunctions::default());
    }

    let mut entries = Vec::new();
    walk_entries(project_dir, &mut entries)?;
    entries.sort();

    if entries.len() > MAX_FUNCTIONS {
        bail!("ONREZA Functions discovery found more than {MAX_FUNCTIONS} entry files");
    }

    let mut functions = Vec::with_capacity(entries.len());
    let mut seen_names = HashMap::new();
    for path in entries {
        let relative = relative_path(project_dir, &path);
        let size = path.metadata()?.len();
        if size > MAX_FUNCTION_FILE_BYTES {
            bail!("function source '{relative}' exceeds {MAX_FUNCTION_FILE_BYTES} bytes");
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("function source '{relative}' is not valid UTF-8 text"))?;
        let analysis = analyze_function_entry(&relative, &content)
            .with_context(|| format!("invalid ONREZA Function declaration in '{relative}'"))?;
        let name = match analysis.declaration.name.as_deref() {
            Some(name) => name.to_string(),
            None => function_name_from_entry(&relative)?,
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

        let mut sources = SourceSet::new();
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

        if file_type.is_file() && is_function_entry_file(&name) {
            entries.push(path);
        }
    }
    Ok(())
}

fn is_function_entry_file(file_name: &str) -> bool {
    FUNCTION_ENTRY_SUFFIXES
        .iter()
        .any(|suffix| file_name.ends_with(suffix))
}

fn function_name_from_entry(entrypoint: &str) -> anyhow::Result<String> {
    let file_name = Path::new(entrypoint)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("function entry '{entrypoint}' has no file name"))?;
    let Some(name) = strip_entry_suffix(file_name) else {
        bail!("function entry '{entrypoint}' must use *.nrz-fn.ts/js/mjs suffix");
    };

    if name.is_empty() {
        bail!("function entry '{entrypoint}' derives an empty function name");
    }
    if !is_function_name_segment(name) {
        bail!(
            "function entry '{entrypoint}' must use lowercase letters, digits, and '-' before .nrz-fn"
        );
    }
    if name.len() > MAX_FUNCTION_NAME_LENGTH {
        bail!("function name '{name}' exceeds {MAX_FUNCTION_NAME_LENGTH} characters");
    }
    Ok(name.to_string())
}

fn strip_entry_suffix(entrypoint: &str) -> Option<&str> {
    FUNCTION_ENTRY_SUFFIXES
        .iter()
        .find_map(|suffix| entrypoint.strip_suffix(suffix))
}

fn is_function_name_segment(segment: &str) -> bool {
    segment
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !segment.starts_with('-')
        && !segment.ends_with('-')
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
