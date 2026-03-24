//! Prisma ORM detection.

use super::package_json::PackageJson;
use super::package_manager;
use super::types::{PackageManagerType, ToolInfo, ToolSlug};

/// Detect Prisma usage from package.json dependencies.
///
/// Returns `Some(ToolInfo)` with a `prisma generate` pre-build command
/// when `@prisma/client` or `prisma` is found in dependencies.
pub fn detect_prisma(pkg: &PackageJson, pm_type: PackageManagerType) -> Option<ToolInfo> {
    let has_client = pkg.dependencies.contains_key("@prisma/client");
    let has_cli =
        pkg.dev_dependencies.contains_key("prisma") || pkg.dependencies.contains_key("prisma");

    if !has_client && !has_cli {
        return None;
    }

    let version = pkg
        .dependency_version("@prisma/client")
        .or_else(|| pkg.dependency_version("prisma"))
        .map(String::from);

    let runner = package_manager::runner_command(pm_type);

    Some(ToolInfo {
        slug: ToolSlug::Prisma,
        name: "Prisma ORM".to_string(),
        version,
        pre_build_command: format!("{runner} prisma generate"),
    })
}
