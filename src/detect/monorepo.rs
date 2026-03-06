//! Monorepo detection: pnpm-workspace.yaml, turbo.json, nx.json,
//! and package.json workspaces.

use std::collections::HashSet;

use super::fs::Fs;
use super::package_json::PackageJson;
use super::types::{
    MonorepoInfo, MonorepoPackage, MonorepoTool, PackageManagerInfo, PackageManagerType,
};

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

    // Collect negation patterns for filtering
    let negations: Vec<&str> = patterns
        .iter()
        .filter(|p| p.starts_with('!'))
        .map(|p| p[1..].trim_end_matches('/'))
        .collect();

    for pattern in patterns {
        expand_pattern(fs, pattern, &mut packages, &mut seen);
    }

    // Filter out packages matching negation patterns
    if !negations.is_empty() {
        packages.retain(|pkg| !negations.iter().any(|neg| pkg.path == *neg));
    }

    // Sort by path for deterministic output
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
    // Negation patterns are handled by resolve_packages (filtering)
    if pattern.starts_with('!') {
        return;
    }

    // Split into prefix and glob part
    if let Some(star_idx) = pattern.find('*') {
        let prefix = &pattern[..star_idx];
        let prefix = prefix.trim_end_matches('/');

        // List entries in the prefix directory
        let dir = if prefix.is_empty() { "." } else { prefix };
        let entries = fs.list_dir(dir);

        for entry in entries {
            let path = if prefix.is_empty() {
                entry.clone()
            } else {
                format!("{prefix}/{entry}")
            };

            // Deduplicate by path (overlapping patterns)
            if !seen.insert(path.clone()) {
                continue;
            }

            // Check if it's a directory with a package.json
            if fs.is_dir(&path) && fs.exists(&format!("{path}/package.json")) {
                let name = read_package_name(fs, &path);
                packages.push(MonorepoPackage { name, path });
            }
        }
    } else {
        // Exact path (e.g., "packages/core")
        if seen.insert(pattern.to_string())
            && fs.is_dir(pattern)
            && fs.exists(&format!("{pattern}/package.json"))
        {
            let name = read_package_name(fs, pattern);
            packages.push(MonorepoPackage {
                name,
                path: pattern.to_string(),
            });
        }
    }
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
pub fn resolve_app(info: &MonorepoInfo, app: &str) -> Option<String> {
    // 1. Exact package name match
    for pkg in &info.packages {
        if pkg.name.as_deref() == Some(app) {
            return Some(pkg.path.clone());
        }
    }

    // 2. Directory basename match
    for pkg in &info.packages {
        let basename = pkg.path.rsplit('/').next().unwrap_or(&pkg.path);
        if basename == app {
            return Some(pkg.path.clone());
        }
    }

    // 3. Exact path match
    for pkg in &info.packages {
        if pkg.path == app {
            return Some(pkg.path.clone());
        }
    }

    None
}
