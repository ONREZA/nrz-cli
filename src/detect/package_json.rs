//! Parsing of package.json for framework detection.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Minimal package.json structure for detection purposes.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PackageJson {
    pub name: Option<String>,
    pub main: Option<String>,
    pub module: Option<String>,

    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,

    #[serde(default)]
    pub scripts: HashMap<String, String>,

    pub package_manager: Option<String>,

    #[serde(default)]
    pub workspaces: Workspaces,
}

/// Workspaces can be either an array or an object with `packages` field.
#[derive(Debug, Default, Deserialize)]
#[serde(untagged)]
pub enum Workspaces {
    #[default]
    None,
    Array(Vec<String>),
    Object {
        #[serde(default)]
        packages: Vec<String>,
    },
}

impl Workspaces {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Array(v) => v.is_empty(),
            Self::Object { packages } => packages.is_empty(),
        }
    }

    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::None => Vec::new(),
            Self::Array(v) => v.clone(),
            Self::Object { packages } => packages.clone(),
        }
    }
}

impl PackageJson {
    /// Load package.json from an abstract filesystem (for remote detection).
    pub fn load_from_fs(fs: &dyn super::fs::Fs) -> Option<Self> {
        let content = fs.read_file("package.json")?;
        match serde_json::from_str(&content) {
            Ok(pkg) => Some(pkg),
            Err(e) => {
                tracing::warn!("could not parse package.json: {e}");
                None
            }
        }
    }

    /// Read and parse package.json from a directory.
    ///
    /// Returns `None` for *any* failure (file not found, IO error, parse error),
    /// emitting a `tracing::warn!` for the non-NotFound cases. Callers that must
    /// distinguish "file absent" from "file present but unreadable" should use
    /// `load_strict` — silencing an IO or parse error here has bitten callers
    /// (e.g. `detect_workers_runtime_target` missing Oxygen/CF signals when the
    /// package.json was corrupted).
    pub fn load(project_dir: &Path) -> Option<Self> {
        match Self::load_strict(project_dir) {
            Ok(opt) => opt,
            Err(e) => {
                tracing::warn!(
                    "could not load {}/package.json: {e:#}",
                    project_dir.display()
                );
                None
            }
        }
    }

    /// Strict variant of `load`: returns `Ok(None)` only when the file doesn't
    /// exist. IO errors and parse errors are propagated so callers that treat
    /// package.json as a *signal source* can surface the failure instead of
    /// silently degrading to "no signal present".
    pub fn load_strict(project_dir: &Path) -> anyhow::Result<Option<Self>> {
        use anyhow::Context as _;

        let path = project_dir.join("package.json");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(anyhow::Error::new(e))
                    .with_context(|| format!("failed to read {}", path.display()));
            }
        };
        let pkg = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Some(pkg))
    }

    /// Check if a package exists in dependencies or devDependencies.
    pub fn has_dependency(&self, name: &str) -> bool {
        self.dependencies.contains_key(name) || self.dev_dependencies.contains_key(name)
    }

    /// Get the version of a dependency (from deps or devDeps).
    pub fn dependency_version(&self, name: &str) -> Option<&str> {
        self.dependencies
            .get(name)
            .or_else(|| self.dev_dependencies.get(name))
            .map(|s| s.as_str())
    }

    /// Check if workspaces are defined (monorepo indicator).
    pub fn is_monorepo(&self) -> bool {
        !self.workspaces.is_empty()
    }
}
