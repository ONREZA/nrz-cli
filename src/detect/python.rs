//! Conservative zero-config detection for CPython process applications.

use std::path::Path;

use super::fs::Fs;
use super::types::{
    BuildInfo, ComputeType, DetectionMetadata, DetectionResult, PackageManagerInfo,
    PackageManagerType, RuntimeInfo, RuntimeType,
};

pub const PYTHON_RUNTIME_VERSION: &str = "3.14";
pub const PYTHON_INTERPRETER: &str = "python3.14";
pub const PYTHON_SITE_PACKAGES_ROOT: &str = ".onreza/python/3.14/site-packages";
pub const PYTHON_ENTRY_CANDIDATES: &[&str] = &[
    "main.py",
    "app.py",
    "server.py",
    "src/main.py",
    "src/app.py",
    "src/server.py",
];

pub fn dependency_manifest(fs: &dyn Fs) -> Option<&'static str> {
    ["requirements.txt", "pyproject.toml", "setup.py"]
        .into_iter()
        .find(|path| fs.exists(path) && !fs.is_dir(path))
}

pub fn has_entry(fs: &dyn Fs) -> bool {
    PYTHON_ENTRY_CANDIDATES
        .iter()
        .any(|path| fs.exists(path) && !fs.is_dir(path))
}

pub fn detect_python(fs: &dyn Fs) -> Option<DetectionResult> {
    let entry_point = PYTHON_ENTRY_CANDIDATES
        .iter()
        .find(|path| fs.exists(path) && !fs.is_dir(path))
        .copied()?;
    if dependency_manifest(fs).is_none() && fs.exists("package.json") {
        return None;
    }
    Some(python_detection(fs, Some(entry_point), false))
}

pub fn detect_configured_python(fs: &dyn Fs) -> DetectionResult {
    let entry_point = PYTHON_ENTRY_CANDIDATES
        .iter()
        .find(|path| fs.exists(path) && !fs.is_dir(path))
        .copied();
    python_detection(fs, entry_point, true)
}

fn python_detection(
    fs: &dyn Fs,
    entry_point: Option<&'static str>,
    configured: bool,
) -> DetectionResult {
    let dependency_file = dependency_manifest(fs);
    let package_manager = Some(PackageManagerInfo {
        pm_type: PackageManagerType::Pip,
        version: None,
        lockfile: dependency_file.map(str::to_string),
    });

    DetectionResult {
        framework: "python".to_string(),
        name: "Python".to_string(),
        version: None,
        suggested_compute: ComputeType::Process,
        metadata: DetectionMetadata {
            uses_typescript: None,
            config_files: dependency_file.into_iter().map(str::to_string).collect(),
            runtime: RuntimeInfo {
                runtime_type: RuntimeType::Python,
                version: Some(PYTHON_RUNTIME_VERSION.to_string()),
            },
            package_manager,
            build_info: Some(BuildInfo {
                build_command: None,
                install_command: dependency_file.map(install_command),
                output_dir: Some(".".to_string()),
                entry_point: entry_point.map(str::to_string),
            }),
            monorepo: None,
            ssr_analysis: None,
            structure: Vec::new(),
        },
        reason: if configured {
            "Configured Python runtime".to_string()
        } else {
            format!(
                "Detected Python process entry {}",
                entry_point.expect("autodetection requires a conventional entry")
            )
        },
    }
}

pub fn install_command(manifest: &str) -> String {
    if manifest == "requirements.txt" {
        format!(
            "{PYTHON_INTERPRETER} -m pip install --disable-pip-version-check --no-compile --target {PYTHON_SITE_PACKAGES_ROOT} --requirement requirements.txt"
        )
    } else {
        format!(
            "{PYTHON_INTERPRETER} -m pip install --disable-pip-version-check --no-compile --target {PYTHON_SITE_PACKAGES_ROOT} ."
        )
    }
}

pub fn resolve_entry_point(output_dir: &Path) -> Option<String> {
    PYTHON_ENTRY_CANDIDATES
        .iter()
        .find(|path| output_dir.join(path).is_file())
        .map(|path| (*path).to_string())
}
