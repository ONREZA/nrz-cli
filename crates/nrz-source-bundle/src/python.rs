//! Closed Python minor identities shared by source and execution contracts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
pub struct PythonToolchainVersions {
    pub uv: String,
    pub poetry: String,
    #[serde(rename = "poetryExport")]
    pub poetry_export: String,
    pub packaging: String,
}

#[derive(Deserialize)]
struct PythonCatalog {
    default: String,
    versions: BTreeMap<String, String>,
}
#[derive(Deserialize)]
struct RuntimeToolchainCatalog {
    python: PythonCatalog,
    installers: PythonToolchainVersions,
}

fn runtime_toolchains() -> &'static RuntimeToolchainCatalog {
    static CATALOG: LazyLock<RuntimeToolchainCatalog> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../assets/runtime-toolchains.json"))
            .expect("generated runtime toolchain catalog must be valid")
    });
    &CATALOG
}

pub fn python_toolchain_versions() -> &'static PythonToolchainVersions {
    &runtime_toolchains().installers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum PythonMinor {
    #[serde(rename = "3.12")]
    Python312,
    #[serde(rename = "3.13")]
    Python313,
    #[serde(rename = "3.14")]
    Python314,
}

impl Default for PythonMinor {
    fn default() -> Self {
        Self::from_version(&runtime_toolchains().python.default)
            .expect("generated default Python minor must be qualified")
    }
}

impl PythonMinor {
    pub const ALL: [Self; 3] = [Self::Python312, Self::Python313, Self::Python314];

    pub const fn version(self) -> &'static str {
        match self {
            Self::Python312 => "3.12",
            Self::Python313 => "3.13",
            Self::Python314 => "3.14",
        }
    }
    /// Exact patch is a generated projection of the platform catalog, never a
    /// floating resolver request or a second independent version policy.
    pub fn exact_version(self) -> &'static str {
        runtime_toolchains()
            .python
            .versions
            .get(self.version())
            .expect("supported Python minor must have an exact patch pin")
            .as_str()
    }
    pub const fn target(self) -> &'static str {
        match self {
            Self::Python312 => "python-3.12",
            Self::Python313 => "python-3.13",
            Self::Python314 => "python-3.14",
        }
    }
    pub const fn site_packages_root(self) -> &'static str {
        match self {
            Self::Python312 => ".onreza/python/3.12/site-packages",
            Self::Python313 => ".onreza/python/3.13/site-packages",
            Self::Python314 => ".onreza/python/3.14/site-packages",
        }
    }
    pub const fn platform_interpreter(self) -> &'static str {
        match self {
            Self::Python312 => "/usr/local/bin/python3.12",
            Self::Python313 => "/usr/local/bin/python3.13",
            Self::Python314 => "/usr/local/bin/python3.14",
        }
    }
    pub const fn profile_name(self) -> &'static str {
        match self {
            Self::Python312 => "CPYTHON_3_12",
            Self::Python313 => "CPYTHON_3_13",
            Self::Python314 => "CPYTHON_3_14",
        }
    }
    pub fn from_profile_name(profile: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|minor| minor.profile_name() == profile)
    }
    pub fn from_version(version: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|minor| minor.version() == version)
    }
    pub fn from_target(target: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|minor| minor.target() == target)
    }
    pub fn for_dependency_path(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|minor| {
            path == minor.site_packages_root()
                || path
                    .strip_prefix(minor.site_packages_root())
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    }
}
