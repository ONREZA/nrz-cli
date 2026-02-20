//! Package manager detection.

use std::path::Path;

use super::package_json::PackageJson;
use super::types::{PackageManagerInfo, PackageManagerType};

/// Detect package manager from package.json and lock files.
pub fn detect_package_manager(
    project_dir: &Path,
    pkg: Option<&PackageJson>,
) -> Option<PackageManagerInfo> {
    // 1. Check packageManager field in package.json (e.g. "pnpm@9.0.0")
    if let Some(pkg) = pkg
        && let Some(ref pm_field) = pkg.package_manager
        && let Some(info) = parse_package_manager_field(pm_field)
    {
        return Some(info);
    }

    // 2. Detect from lock files
    if let Some(info) = detect_from_lockfile(project_dir) {
        return Some(info);
    }

    // 3. Default to npm if package.json exists
    if pkg.is_some() {
        return Some(PackageManagerInfo {
            pm_type: PackageManagerType::Npm,
            version: None,
            lockfile: None,
        });
    }

    None
}

/// Parse the `packageManager` field (e.g. "pnpm@9.0.0" → Pnpm + version).
fn parse_package_manager_field(field: &str) -> Option<PackageManagerInfo> {
    let (name, version) = if let Some(idx) = field.find('@') {
        let name = &field[..idx];
        let ver = &field[idx + 1..];
        (
            name,
            if ver.is_empty() {
                None
            } else {
                Some(ver.to_string())
            },
        )
    } else {
        (field.trim(), None)
    };

    let pm_type = match name {
        "npm" => PackageManagerType::Npm,
        "yarn" => PackageManagerType::Yarn,
        "pnpm" => PackageManagerType::Pnpm,
        "bun" => PackageManagerType::Bun,
        _ => return None,
    };

    Some(PackageManagerInfo {
        pm_type,
        version,
        lockfile: None,
    })
}

/// Detect from lock file presence.
fn detect_from_lockfile(project_dir: &Path) -> Option<PackageManagerInfo> {
    let lockfiles: &[(&str, PackageManagerType)] = &[
        ("bun.lock", PackageManagerType::Bun),
        ("bun.lockb", PackageManagerType::Bun),
        ("pnpm-lock.yaml", PackageManagerType::Pnpm),
        ("yarn.lock", PackageManagerType::Yarn),
        ("package-lock.json", PackageManagerType::Npm),
    ];

    for (file, pm_type) in lockfiles {
        if project_dir.join(file).exists() {
            return Some(PackageManagerInfo {
                pm_type: *pm_type,
                version: None,
                lockfile: Some(file.to_string()),
            });
        }
    }

    None
}

/// Get the install command for a package manager type.
pub fn install_command(pm: PackageManagerType) -> &'static str {
    match pm {
        PackageManagerType::Npm => "npm install",
        PackageManagerType::Yarn => "yarn install",
        PackageManagerType::Pnpm => "pnpm install",
        PackageManagerType::Bun => "bun install",
    }
}

/// Get the build command for a package manager type with a given script name.
pub fn build_command(pm: PackageManagerType, script: &str) -> String {
    if script.is_empty() {
        return String::new();
    }
    match pm {
        PackageManagerType::Npm => format!("npm run {script}"),
        PackageManagerType::Yarn => format!("yarn {script}"),
        PackageManagerType::Pnpm => format!("pnpm {script}"),
        PackageManagerType::Bun => format!("bun run {script}"),
    }
}
