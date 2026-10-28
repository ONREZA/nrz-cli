//! Canonical Python minor declarations shared by source and execution contracts.

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

pub fn validate_runtime_version_declaration(version: &str) -> Result<(), String> {
    if version.is_empty() || version.len() > 128 || version.contains('\0') {
        return Err(
            "runtime version declaration must be nonempty text within 128 UTF-8 bytes without NUL"
                .into(),
        );
    }
    Ok(())
}

pub fn deserialize_python_version_declaration<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = Option::<String>::deserialize(deserializer)?;
    if let Some(version) = version.as_deref() {
        validate_runtime_version_declaration(version).map_err(serde::de::Error::custom)?;
    }
    Ok(version)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonMinor {
    major: u32,
    minor: u32,
}

impl Serialize for PythonMinor {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.version())
    }
}

impl<'de> Deserialize<'de> for PythonMinor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let version = String::deserialize(deserializer)?;
        Self::from_version(&version).ok_or_else(|| {
            serde::de::Error::custom("pythonVersion must be a canonical numeric major.minor")
        })
    }
}

impl Default for PythonMinor {
    fn default() -> Self {
        Self::from_version(&runtime_toolchains().python.default)
            .expect("generated default Python minor must be canonical")
    }
}

impl PythonMinor {
    #[allow(non_upper_case_globals)]
    pub const Python312: Self = Self {
        major: 3,
        minor: 12,
    };
    #[allow(non_upper_case_globals)]
    pub const Python313: Self = Self {
        major: 3,
        minor: 13,
    };
    #[allow(non_upper_case_globals)]
    pub const Python314: Self = Self {
        major: 3,
        minor: 14,
    };
    pub const ALL: [Self; 3] = [Self::Python312, Self::Python313, Self::Python314];

    pub fn version(self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
    /// Exact patch of an available compiler from the generated platform catalog.
    pub fn exact_version(self) -> &'static str {
        runtime_toolchains()
            .python
            .versions
            .get(&self.version())
            .expect("selected Python compiler must have an exact patch pin")
            .as_str()
    }
    pub fn target(self) -> String {
        format!("python-{}", self.version())
    }
    pub fn site_packages_root(self) -> String {
        format!(".onreza/python/{}/site-packages", self.version())
    }
    pub fn platform_interpreter(self) -> String {
        format!("/usr/local/bin/python{}", self.version())
    }
    pub fn profile_name(self) -> String {
        format!("CPYTHON_{}_{}", self.major, self.minor)
    }
    pub fn from_profile_name(profile: &str) -> Option<Self> {
        Self::parse(profile.strip_prefix("CPYTHON_")?, '_')
    }
    pub fn from_version(version: &str) -> Option<Self> {
        Self::parse(version, '.')
    }
    fn parse(version: &str, separator: char) -> Option<Self> {
        let (major, minor) = version.split_once(separator)?;
        let parse = |part: &str| {
            (!part.is_empty()
                && part.len() <= 9
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0')))
            .then(|| part.parse::<u32>().ok())
            .flatten()
        };
        Some(Self {
            major: parse(major)?,
            minor: parse(minor)?,
        })
    }
    pub fn from_target(target: &str) -> Option<Self> {
        match nrz_contract::RuntimeProfile::python_build_target(target)? {
            nrz_contract::RuntimeProfile::Cpython(major, minor) => Some(Self { major, minor }),
            _ => None,
        }
    }
    pub fn for_dependency_path(path: &str) -> Option<Self> {
        let (version, suffix) = path.strip_prefix(".onreza/python/")?.split_once('/')?;
        (suffix == "site-packages" || suffix.starts_with("site-packages/"))
            .then(|| Self::from_version(version))
            .flatten()
    }
}
