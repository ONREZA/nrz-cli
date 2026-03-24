//! Types for framework detection.

use serde::Serialize;

/// Runtime type for a detected framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum RuntimeType {
    Node,
    Bun,
    Deno,
    Python,
    Static,
}

/// Category of a framework preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetCategory {
    React,
    Vue,
    Svelte,
    Static,
    Server,
    Other,
}

/// Package manager type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManagerType {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl PackageManagerType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Yarn => "yarn",
            Self::Pnpm => "pnpm",
            Self::Bun => "bun",
        }
    }
}

impl std::fmt::Display for PackageManagerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for ComputeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => f.write_str("STATIC"),
            Self::Isolate => f.write_str("ISOLATE"),
            Self::Process => f.write_str("PROCESS"),
        }
    }
}

/// Static framework preset definition.
#[allow(dead_code)]
pub struct FrameworkPreset {
    pub slug: &'static str,
    pub name: &'static str,
    pub dependencies: &'static [&'static str],
    pub output_directory: &'static str,
    pub build_script: Option<&'static str>,
    pub category: PresetCategory,
    pub priority: u32,
    pub runtime: RuntimeType,
}

/// SSR analysis result for SSR-capable frameworks.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrAnalysis {
    pub is_static_compatible: bool,
    pub ssr_features: Vec<String>,
}

impl SsrAnalysis {
    pub fn has_ssr_features(&self) -> bool {
        !self.ssr_features.is_empty()
    }

    /// Whether SSR analysis detected `output: 'standalone'` (Next.js).
    pub fn has_standalone_output(&self) -> bool {
        self.ssr_features.iter().any(|f| f.contains("standalone"))
    }
}

/// Detected package manager info.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManagerInfo {
    #[serde(rename = "type")]
    pub pm_type: PackageManagerType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lockfile: Option<String>,
}

/// Build information derived from detection.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<String>,
}

/// Monorepo orchestration tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MonorepoTool {
    /// npm workspaces (package.json only)
    Npm,
    /// yarn workspaces (package.json only)
    Yarn,
    /// pnpm workspaces (pnpm-workspace.yaml)
    Pnpm,
    /// bun workspaces (package.json only)
    Bun,
    /// Turborepo (turbo.json on top of npm/pnpm/yarn/bun workspaces)
    Turbo,
    /// Nx (nx.json on top of workspaces or standalone)
    Nx,
}

impl std::fmt::Display for MonorepoTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Npm => f.write_str("npm workspaces"),
            Self::Yarn => f.write_str("yarn workspaces"),
            Self::Pnpm => f.write_str("pnpm workspaces"),
            Self::Bun => f.write_str("bun workspaces"),
            Self::Turbo => f.write_str("turborepo"),
            Self::Nx => f.write_str("nx"),
        }
    }
}

/// Resolved workspace package in a monorepo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonorepoPackage {
    /// Package name from package.json (may be absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Relative path from the monorepo root.
    pub path: String,
}

/// Monorepo information.
///
/// Presence of `Some(MonorepoInfo)` in metadata means the project is a monorepo.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonorepoInfo {
    pub tool: MonorepoTool,
    pub workspaces: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<MonorepoPackage>,
}

/// Runtime info.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    #[serde(rename = "type")]
    pub runtime_type: RuntimeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Detection metadata — additional info beyond the framework slug.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_typescript: Option<bool>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub config_files: Vec<String>,

    pub runtime: RuntimeInfo,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<PackageManagerInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_info: Option<BuildInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub monorepo: Option<MonorepoInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssr_analysis: Option<SsrAnalysis>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structure: Vec<String>,
}

/// Suggested compute type for deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComputeType {
    /// Pure static files, no server runtime needed.
    Static,
    /// Edge/isolate runtime via @onreza adapter (V8 isolate).
    Isolate,
    /// Full runtime with fs access (standalone Next.js, custom servers, binaries).
    Process,
}

/// The full detection result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub framework: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub suggested_compute: ComputeType,
    pub metadata: DetectionMetadata,
    pub reason: String,
}
