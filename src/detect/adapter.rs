//! Detection of @onreza/adapter-* platform adapters.

use super::package_json::PackageJson;
use super::types::AdapterInfo;

/// Adapter package prefix. All adapters follow `@onreza/adapter-{name}` naming.
const ADAPTER_PREFIX: &str = "@onreza/adapter-";

/// Detect if any @onreza/adapter-* package is installed.
///
/// Searches both `dependencies` and `devDependencies` for any package
/// matching the `@onreza/adapter-*` pattern.
pub fn detect_adapter(pkg: &PackageJson) -> Option<AdapterInfo> {
    // Check dependencies first, then devDependencies
    for deps in [&pkg.dependencies, &pkg.dev_dependencies] {
        for (name, version) in deps {
            if name.starts_with(ADAPTER_PREFIX) {
                return Some(AdapterInfo {
                    adapter_package: name.clone(),
                    adapter_version: Some(version.clone()),
                });
            }
        }
    }
    None
}
