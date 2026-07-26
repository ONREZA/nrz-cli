//! Monorepo detection: pnpm-workspace.yaml, turbo.json, nx.json,
//! and package.json workspaces.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::fs::{Fs, LocalFs, MAX_DETECTION_PATH_DEPTH};
use super::package_json::PackageJson;
use super::package_manager::detect_package_manager;
use super::types::{
    MonorepoInfo, MonorepoPackage, MonorepoTool, PackageManagerInfo, PackageManagerType,
};

const MAX_WORKSPACE_EXPANSION_STATES: usize = 100_000;

/// Detect monorepo setup from filesystem and package.json.
///
/// Priority for workspace patterns:
/// 1. `pnpm-workspace.yaml` (canonical for pnpm)
/// 2. `package.json` workspaces (npm/yarn/bun — distinguished by detected package manager)
///
/// Orchestrator upgrade: turbo.json → Turbo, nx.json → Nx.
pub fn detect_monorepo(
    fs: &dyn Fs,
    pkg: Option<&PackageJson>,
    pm: Option<&PackageManagerInfo>,
) -> Option<MonorepoInfo> {
    let (base_tool, workspaces) = detect_workspace_patterns(fs, pkg, pm)?;

    // Upgrade tool if an orchestrator is present.
    // NOTE: This replaces base_tool (e.g. Yarn) with Turbo/Nx, so the
    // underlying workspace manager identity is lost. If consumers need
    // to know the base workspace tool, expose `base_tool` as a separate field.
    let tool = if fs.exists("turbo.json") {
        MonorepoTool::Turbo
    } else if fs.exists("nx.json") {
        MonorepoTool::Nx
    } else {
        base_tool
    };

    let packages = resolve_packages(fs, &workspaces);

    Some(MonorepoInfo {
        tool,
        workspaces,
        packages,
    })
}

/// Find the nearest ancestor workspace that declares `project_dir` as one of
/// its packages. Falls back to the project itself when it is not in a
/// workspace.
pub fn discover_workspace_root(project_dir: &Path) -> PathBuf {
    let canonical_project = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());

    for candidate in canonical_project.ancestors() {
        let relative = match canonical_project.strip_prefix(candidate) {
            Ok(path) => path_to_workspace_string(path),
            Err(_) => continue,
        };
        let fs = LocalFs::new(candidate);
        let package_json = PackageJson::load_from_fs(&fs);
        let package_manager = detect_package_manager(&fs, package_json.as_ref());
        let Some(monorepo) = detect_monorepo(&fs, package_json.as_ref(), package_manager.as_ref())
        else {
            continue;
        };

        if relative.is_empty()
            || monorepo
                .packages
                .iter()
                .any(|package| package.path == relative)
        {
            return candidate.to_path_buf();
        }
    }

    canonical_project
}

/// Detect workspace patterns from pnpm-workspace.yaml or package.json.
fn detect_workspace_patterns(
    fs: &dyn Fs,
    pkg: Option<&PackageJson>,
    pm: Option<&PackageManagerInfo>,
) -> Option<(MonorepoTool, Vec<String>)> {
    // 1. pnpm-workspace.yaml takes priority
    if let Some(content) = fs.read_file("pnpm-workspace.yaml") {
        let patterns = parse_pnpm_workspace(&content);
        if !patterns.is_empty() {
            return Some((MonorepoTool::Pnpm, patterns));
        }
        tracing::warn!(
            "pnpm-workspace.yaml exists but no workspace patterns were extracted; \
             check the file format (flow-style YAML and tabs are not supported)"
        );
    }

    // 2. package.json workspaces — tool depends on the package manager
    if let Some(pkg) = pkg
        && pkg.is_monorepo()
    {
        let tool = match pm.map(|p| p.pm_type) {
            Some(PackageManagerType::Yarn) => MonorepoTool::Yarn,
            Some(PackageManagerType::Pnpm) => MonorepoTool::Pnpm,
            Some(PackageManagerType::Bun) => MonorepoTool::Bun,
            _ => MonorepoTool::Npm,
        };
        return Some((tool, pkg.workspaces.to_vec()));
    }

    None
}

/// Parse pnpm-workspace.yaml to extract workspace patterns.
///
/// Handles the standard format:
/// ```yaml
/// packages:
///   - 'apps/*'
///   - 'packages/*'
/// ```
pub fn parse_pnpm_workspace(content: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut in_packages = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Detect `packages:` key
        if trimmed == "packages:" {
            in_packages = true;
            continue;
        }

        // If we hit another top-level key, stop collecting
        if !trimmed.starts_with('-') && !trimmed.starts_with(' ') && in_packages {
            break;
        }

        // Collect list items under `packages:`
        if in_packages && trimmed.starts_with('-') {
            let value = trimmed
                .trim_start_matches('-')
                .trim()
                .trim_matches('\'')
                .trim_matches('"');
            if !value.is_empty() {
                patterns.push(value.to_string());
            }
        }
    }

    patterns
}

/// Resolve workspace patterns to actual packages by expanding globs
/// and reading package.json in each matched directory.
fn resolve_packages(fs: &dyn Fs, patterns: &[String]) -> Vec<MonorepoPackage> {
    let mut packages = Vec::new();
    let mut seen = HashSet::new();

    for pattern in patterns {
        expand_pattern(fs, pattern, &mut packages, &mut seen);
    }

    let mut excluded = HashSet::new();
    for pattern in patterns
        .iter()
        .filter_map(|pattern| pattern.strip_prefix('!'))
    {
        let mut excluded_packages = Vec::new();
        let mut excluded_seen = HashSet::new();
        expand_pattern(fs, pattern, &mut excluded_packages, &mut excluded_seen);
        excluded.extend(excluded_packages.into_iter().map(|package| package.path));
    }
    packages.retain(|package| !excluded.contains(&package.path));

    packages.sort_by(|a, b| a.path.cmp(&b.path));
    packages
}

/// Expand a single workspace pattern (e.g., "apps/*") into concrete packages.
fn expand_pattern(
    fs: &dyn Fs,
    pattern: &str,
    packages: &mut Vec<MonorepoPackage>,
    seen: &mut HashSet<String>,
) {
    if pattern.starts_with('!') {
        return;
    }
    let Some(pattern) = normalize_workspace_pattern(pattern) else {
        return;
    };
    let segments = pattern.split('/').collect::<Vec<_>>();
    let mut visited = HashSet::new();
    expand_pattern_segments(fs, &segments, 0, "", packages, seen, &mut visited);
}

fn expand_pattern_segments(
    fs: &dyn Fs,
    segments: &[&str],
    segment_index: usize,
    current: &str,
    packages: &mut Vec<MonorepoPackage>,
    seen: &mut HashSet<String>,
    visited: &mut HashSet<(String, usize)>,
) {
    if visited.len() >= MAX_WORKSPACE_EXPANSION_STATES
        || current
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            > MAX_DETECTION_PATH_DEPTH
        || !visited.insert((current.to_string(), segment_index))
    {
        return;
    }

    if segment_index == segments.len() {
        if !current.is_empty()
            && seen.insert(current.to_string())
            && fs.is_dir(current)
            && fs.exists(&format!("{current}/package.json"))
        {
            packages.push(MonorepoPackage {
                name: read_package_name(fs, current),
                path: current.to_string(),
            });
        }
        return;
    }

    let segment = segments[segment_index];
    if segment == "**" {
        expand_pattern_segments(
            fs,
            segments,
            segment_index + 1,
            current,
            packages,
            seen,
            visited,
        );
        for entry in fs.list_dir(current) {
            if is_ignored_workspace_directory(&entry) {
                continue;
            }
            let path = join_workspace_path(current, &entry);
            if fs.is_dir(&path) {
                expand_pattern_segments(
                    fs,
                    segments,
                    segment_index,
                    &path,
                    packages,
                    seen,
                    visited,
                );
            }
        }
        return;
    }

    for entry in fs.list_dir(current) {
        if !workspace_segment_matches(&entry, segment) {
            continue;
        }
        let path = join_workspace_path(current, &entry);
        if fs.is_dir(&path) {
            expand_pattern_segments(
                fs,
                segments,
                segment_index + 1,
                &path,
                packages,
                seen,
                visited,
            );
        }
    }
}

fn normalize_workspace_pattern(pattern: &str) -> Option<String> {
    let normalized = pattern.trim().replace('\\', "/");
    if normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':') {
        return None;
    }
    let normalized = normalized
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if normalized.is_empty()
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return None;
    }
    Some(normalized)
}

fn workspace_segment_matches(value: &str, pattern: &str) -> bool {
    let value = value.chars().collect::<Vec<_>>();
    let pattern = pattern.chars().collect::<Vec<_>>();
    let mut matches = vec![vec![false; pattern.len() + 1]; value.len() + 1];
    matches[0][0] = true;
    for pattern_index in 0..pattern.len() {
        if pattern[pattern_index] == '*' {
            matches[0][pattern_index + 1] = matches[0][pattern_index];
        }
    }
    for value_index in 0..=value.len() {
        for pattern_index in 0..pattern.len() {
            matches[value_index][pattern_index + 1] = match pattern[pattern_index] {
                '*' => {
                    matches[value_index][pattern_index]
                        || (value_index > 0 && matches[value_index - 1][pattern_index + 1])
                }
                '?' => value_index > 0 && matches[value_index - 1][pattern_index],
                literal => {
                    value_index > 0
                        && value[value_index - 1] == literal
                        && matches[value_index - 1][pattern_index]
                }
            };
        }
    }
    matches[value.len()][pattern.len()]
}

fn join_workspace_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{parent}/{child}")
    }
}

fn is_ignored_workspace_directory(name: &str) -> bool {
    matches!(name, ".git" | "node_modules")
}

fn path_to_workspace_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Read the `name` field from a package.json in the given directory.
fn read_package_name(fs: &dyn Fs, dir: &str) -> Option<String> {
    let path = format!("{dir}/package.json");
    let content = fs.read_file(&path)?;
    match serde_json::from_str::<PackageJson>(&content) {
        Ok(pkg) => pkg.name,
        Err(e) => {
            tracing::warn!(
                path = %path,
                error = %e,
                "failed to parse workspace package.json, package name will be unavailable"
            );
            None
        }
    }
}

/// Resolve an `--app` argument to a workspace directory path.
///
/// Matches by:
/// 1. Package name (exact match from package.json `name`)
/// 2. Directory basename (last segment of the path)
/// 3. Relative path (exact path match)
pub fn resolve_app(info: &MonorepoInfo, app: &str) -> Result<Option<String>, Vec<String>> {
    // 1. Exact package name match
    let matches = info
        .packages
        .iter()
        .filter(|package| package.name.as_deref() == Some(app))
        .map(|package| package.path.clone())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [path] => return Ok(Some(path.clone())),
        [] => {}
        _ => return Err(matches),
    }

    // 2. Directory basename match
    let matches = info
        .packages
        .iter()
        .filter(|package| package.path.rsplit('/').next().unwrap_or(&package.path) == app)
        .map(|package| package.path.clone())
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [path] => return Ok(Some(path.clone())),
        [] => {}
        _ => return Err(matches),
    }

    // 3. Exact path match
    for pkg in &info.packages {
        if pkg.path == app {
            return Ok(Some(pkg.path.clone()));
        }
    }

    Ok(None)
}
