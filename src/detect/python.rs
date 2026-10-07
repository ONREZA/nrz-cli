//! Python project evidence and deployment dependency inputs.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, bail};

use super::fs::Fs;
use super::types::{
    BuildInfo, ComputeType, DetectionMetadata, DetectionResult, PackageManagerInfo,
    PackageManagerType, RuntimeInfo, RuntimeType,
};

pub const PYTHON_ENTRY_CANDIDATES: &[&str] = &[
    "main.py",
    "app.py",
    "server.py",
    "src/main.py",
    "src/app.py",
    "src/server.py",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PythonDependencyKind {
    Requirements,
    Project,
    Uv,
    Poetry,
    Setup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonDependencyPlan {
    pub kind: PythonDependencyKind,
    pub manifest: &'static str,
    pub project_name: Option<String>,
    pub requires_python: Option<String>,
    pub poetry_requires_python: Option<String>,
    pub install_project: bool,
}

pub fn dependency_manifest(fs: &dyn Fs) -> Option<&'static str> {
    if fs.exists("uv.lock") || fs.exists("poetry.lock") {
        return Some("pyproject.toml");
    }
    ["requirements.txt", "pyproject.toml", "setup.py"]
        .into_iter()
        .find(|path| fs.exists(path) && !fs.is_dir(path))
}

/// Lockfile workflows take precedence over incidental exported requirements.
/// Their owner validates freshness; this reader never resolves or rewrites locks.
pub fn dependency_plan(fs: &dyn Fs) -> anyhow::Result<Option<PythonDependencyPlan>> {
    if fs.exists("uv.lock") && fs.exists("poetry.lock") {
        bail!("uv.lock and poetry.lock both exist; choose one Python dependency owner");
    }
    let Some(manifest) = dependency_manifest(fs) else {
        return Ok(None);
    };
    let pyproject = fs
        .read_file("pyproject.toml")
        .map(|text| toml::from_str::<toml::Value>(&text))
        .transpose()
        .context("invalid pyproject.toml")?;
    if (fs.exists("uv.lock") || fs.exists("poetry.lock")) && pyproject.is_none() {
        bail!("Python lockfile requires a readable pyproject.toml");
    }
    let project = pyproject.as_ref().and_then(|value| value.get("project"));
    let poetry = pyproject
        .as_ref()
        .and_then(|value| value.get("tool"))
        .and_then(|value| value.get("poetry"));
    let project_name = project
        .and_then(|project| project.get("name"))
        .or_else(|| poetry.and_then(|poetry| poetry.get("name")))
        .and_then(toml::Value::as_str)
        .map(str::to_string);
    let requires_python = project
        .and_then(|project| project.get("requires-python"))
        .and_then(toml::Value::as_str)
        .map(str::to_string);
    let poetry_requires_python = poetry
        .and_then(|poetry| poetry.get("dependencies"))
        .and_then(|dependencies| dependencies.get("python"))
        .and_then(toml::Value::as_str)
        .map(str::to_string);
    let kind = if fs.exists("poetry.lock") {
        PythonDependencyKind::Poetry
    } else if fs.exists("uv.lock") {
        PythonDependencyKind::Uv
    } else if poetry.is_some() {
        bail!("Poetry project requires poetry.lock; run poetry lock before deployment");
    } else {
        match manifest {
            "requirements.txt" => PythonDependencyKind::Requirements,
            "setup.py" => PythonDependencyKind::Setup,
            _ => PythonDependencyKind::Project,
        }
    };
    let poetry_package_disabled = poetry
        .and_then(|poetry| poetry.get("package-mode"))
        .and_then(toml::Value::as_bool)
        == Some(false);
    let uv_package_disabled = pyproject
        .as_ref()
        .and_then(|value| value.get("tool"))
        .and_then(|value| value.get("uv"))
        .and_then(|value| value.get("package"))
        .and_then(toml::Value::as_bool)
        == Some(false);
    let package_disabled = match kind {
        PythonDependencyKind::Poetry => poetry_package_disabled,
        PythonDependencyKind::Uv => uv_package_disabled,
        _ => poetry_package_disabled || uv_package_disabled,
    };
    let install_project = !package_disabled
        && (kind == PythonDependencyKind::Setup
            || fs.exists("setup.py") && !fs.is_dir("setup.py")
            || poetry.is_some()
            || pyproject
                .as_ref()
                .is_some_and(|value| value.get("build-system").is_some())
            || fs.list_dir("src").into_iter().any(|path| {
                fs.is_dir(&format!("src/{path}")) && fs.exists(&format!("src/{path}/__init__.py"))
            }));
    Ok(Some(PythonDependencyPlan {
        kind,
        manifest,
        project_name,
        requires_python,
        poetry_requires_python: (kind == PythonDependencyKind::Poetry)
            .then_some(poetry_requires_python)
            .flatten(),
        install_project,
    }))
}

/// Installation evidence is broader than confidently named dependencies:
/// local paths, archives and unnamed URLs also require a materialized stage.
pub fn requires_dependency_stage(fs: &dyn Fs) -> anyhow::Result<bool> {
    let plan = dependency_plan(fs)?;
    if plan.as_ref().is_some_and(|plan| plan.install_project) {
        return Ok(true);
    }
    if plan
        .as_ref()
        .is_some_and(|plan| plan.kind == PythonDependencyKind::Requirements)
    {
        return Ok(fs.read_file("requirements.txt").is_some_and(|text| {
            text.replace("\\\r\n", "")
                .replace("\\\n", "")
                .lines()
                .any(|line| {
                    let line = line.trim();
                    !line.is_empty()
                        && !line.starts_with('#')
                        && (!line.starts_with('-')
                            || line.starts_with("-r")
                            || line.starts_with("--requirement")
                            || line.starts_with("-e")
                            || line.starts_with("--editable"))
                })
        }));
    }
    if plan.is_some_and(|plan| plan.manifest == "pyproject.toml")
        && let Some(text) = fs.read_file("pyproject.toml")
    {
        let value: toml::Value = toml::from_str(&text).context("invalid pyproject.toml")?;
        if let Some(dependencies) = value
            .get("project")
            .and_then(|project| project.get("dependencies"))
            .and_then(toml::Value::as_array)
        {
            return Ok(dependencies
                .iter()
                .filter_map(toml::Value::as_str)
                .any(|requirement| !requirement.trim().is_empty()));
        }
    }
    Ok(!dependency_names(fs)?.is_empty())
}

pub fn dependency_names(fs: &dyn Fs) -> anyhow::Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    let manifest = dependency_manifest(fs);
    if let Some(text) = (manifest == Some("requirements.txt"))
        .then(|| fs.read_file("requirements.txt"))
        .flatten()
    {
        for requirement in text.lines() {
            if let Some(name) = requirement_name(requirement) {
                names.insert(name);
            }
        }
    }
    if let Some(text) = (manifest == Some("pyproject.toml"))
        .then(|| fs.read_file("pyproject.toml"))
        .flatten()
    {
        let value: toml::Value = toml::from_str(&text).context("invalid pyproject.toml")?;
        if let Some(dependencies) = value
            .get("project")
            .and_then(|value| value.get("dependencies"))
            .and_then(toml::Value::as_array)
        {
            for requirement in dependencies.iter().filter_map(toml::Value::as_str) {
                if let Some(name) = requirement_name(requirement) {
                    names.insert(name);
                }
            }
        }
        // Static PEP 621 requirements own the published runtime dependencies.
        // Legacy Poetry tables also contain optional or source-only metadata.
        if value
            .get("project")
            .and_then(|project| project.get("dependencies"))
            .is_none()
            && let Some(dependencies) = value
                .get("tool")
                .and_then(|value| value.get("poetry"))
                .and_then(|value| value.get("dependencies"))
                .and_then(toml::Value::as_table)
        {
            for (name, requirement) in dependencies {
                if name != "python"
                    && requirement.get("optional").and_then(toml::Value::as_bool) != Some(true)
                {
                    names.insert(normalize_package_name(name));
                }
            }
        }
    }
    Ok(names)
}

fn requirement_name(requirement: &str) -> Option<String> {
    let requirement = requirement.trim();
    if !requirement.starts_with(|character: char| character.is_ascii_alphanumeric()) {
        return None;
    }
    let name: String = requirement
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        .collect();
    let remainder = &requirement[name.len()..];
    let suffix = remainder.trim_start();
    let lowercase = name.to_ascii_lowercase();
    let archive = [".whl", ".zip", ".tar.gz", ".tar.bz2", ".tar.xz", ".tgz"]
        .iter()
        .any(|extension| lowercase.ends_with(extension));
    (!name.is_empty()
        && !archive
        && (suffix.is_empty()
            || suffix.starts_with(['[', '<', '>', '=', '!', '~', '@', ';'])
            || remainder.starts_with(char::is_whitespace) && suffix.starts_with('#')))
    .then(|| normalize_package_name(&name))
}

fn normalize_package_name(name: &str) -> String {
    name.to_ascii_lowercase().replace(['_', '.'], "-")
}

/// Marker evaluation and recursive requirements acquisition belong to the
/// installer. Detection must not invent a server from incomplete evidence.
pub(crate) fn framework_evidence_complete(fs: &dyn Fs) -> anyhow::Result<bool> {
    let conditional_framework = |requirement: &str| {
        let end = requirement
            .char_indices()
            .find_map(|(offset, ch)| {
                (ch == '#' && requirement[..offset].ends_with(char::is_whitespace))
                    .then_some(offset)
            })
            .unwrap_or(requirement.len());
        let requirement = &requirement[..end];
        requirement.contains(';')
            && requirement_name(requirement)
                .is_some_and(|name| is_python_framework(&name) && name != "python")
    };
    match dependency_manifest(fs) {
        Some("requirements.txt") => Ok(fs.read_file("requirements.txt").is_none_or(|text| {
            !text.lines().any(|line| {
                let line = line.trim();
                line.starts_with("-r")
                    || line.starts_with("--requirement")
                    || conditional_framework(line)
            })
        })),
        Some("pyproject.toml") => {
            let Some(text) = fs.read_file("pyproject.toml") else {
                return Ok(true);
            };
            let value: toml::Value = toml::from_str(&text)?;
            if let Some(dependencies) = value
                .get("project")
                .and_then(|p| p.get("dependencies"))
                .and_then(toml::Value::as_array)
            {
                return Ok(!dependencies
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .any(conditional_framework));
            }
            Ok(value
                .get("tool")
                .and_then(|p| p.get("poetry"))
                .and_then(|p| p.get("dependencies"))
                .and_then(toml::Value::as_table)
                .is_none_or(|deps| {
                    !deps.iter().any(|(name, requirement)| {
                        is_python_framework(&normalize_package_name(name))
                            && name != "python"
                            && requirement.get("optional").and_then(toml::Value::as_bool)
                                != Some(true)
                            && (requirement.is_array()
                                || ["markers", "python", "platform"]
                                    .iter()
                                    .any(|key| requirement.get(*key).is_some()))
                    })
                }))
        }
        _ => Ok(true),
    }
}

pub fn framework(fs: &dyn Fs) -> anyhow::Result<&'static str> {
    if fs.exists("manage.py") && !fs.is_dir("manage.py") {
        return Ok("django");
    }
    if !framework_evidence_complete(fs)? {
        bail!(
            "Python framework inference is incomplete because dependencies use conditional framework requirements or requirements includes; declare deploy.entry, module, application with server, or project.framework explicitly"
        );
    }
    let dependencies = dependency_names(fs)?;
    // FastAPI itself depends on Starlette: the specific framework wins.
    if dependencies.contains("fastapi") {
        Ok("fastapi")
    } else if dependencies.contains("django") || fs.exists("manage.py") {
        Ok("django")
    } else if dependencies.contains("flask") {
        Ok("flask")
    } else if dependencies.contains("starlette") {
        Ok("starlette")
    } else {
        Ok("python")
    }
}

pub fn is_python_framework(framework: &str) -> bool {
    matches!(
        framework,
        "python" | "fastapi" | "starlette" | "django" | "flask"
    )
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
        .copied();
    let has_manifest = dependency_manifest(fs).is_some();
    if fs.exists("package.json") && (entry_point.is_none() || !has_manifest) {
        return None;
    }
    let pyproject_project = fs
        .read_file("pyproject.toml")
        .and_then(|text| toml::from_str::<toml::Value>(&text).ok())
        .is_some_and(|value| {
            value.get("project").is_some()
                || value.get("build-system").is_some()
                || value
                    .get("tool")
                    .and_then(|value| value.get("poetry"))
                    .is_some()
        });
    if entry_point.is_none()
        && !fs.exists("requirements.txt")
        && !fs.exists("setup.py")
        && !fs.exists("manage.py")
        && !pyproject_project
    {
        return None;
    }
    Some(python_detection(fs, entry_point, false))
}

pub fn detect_configured_python(fs: &dyn Fs) -> DetectionResult {
    let entry_point = PYTHON_ENTRY_CANDIDATES
        .iter()
        .find(|path| fs.exists(path) && !fs.is_dir(path))
        .copied();
    python_detection(fs, entry_point, true)
}

pub fn detect_configured_python_framework(fs: &dyn Fs, slug: &str) -> DetectionResult {
    let mut result = detect_configured_python(fs);
    result.framework = slug.to_string();
    result.name = framework_name(slug).to_string();
    result
}

fn framework_name(framework: &str) -> &'static str {
    match framework {
        "fastapi" => "FastAPI",
        "starlette" => "Starlette",
        "django" => "Django",
        "flask" => "Flask",
        _ => "Python",
    }
}

fn python_detection(
    fs: &dyn Fs,
    entry_point: Option<&'static str>,
    configured: bool,
) -> DetectionResult {
    let dependency_file = dependency_manifest(fs);
    let framework = framework(fs).unwrap_or("python");
    let (pm_type, lockfile) = if fs.exists("poetry.lock") {
        (PackageManagerType::Poetry, Some("poetry.lock"))
    } else if fs.exists("uv.lock") {
        (PackageManagerType::Uv, Some("uv.lock"))
    } else {
        (PackageManagerType::Pip, dependency_file)
    };
    DetectionResult {
        framework: framework.to_string(),
        name: framework_name(framework).to_string(),
        version: None,
        suggested_compute: ComputeType::Process,
        metadata: DetectionMetadata {
            source_build_context: None,
            uses_typescript: None,
            config_files: [dependency_file, lockfile]
                .into_iter()
                .flatten()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(str::to_string)
                .collect(),
            runtime: RuntimeInfo {
                runtime_type: RuntimeType::Python,
                version: Some(
                    nrz_source_bundle::PythonMinor::default()
                        .version()
                        .to_string(),
                ),
            },
            package_manager: Some(PackageManagerInfo {
                pm_type,
                version: None,
                lockfile: lockfile.map(str::to_string),
            }),
            build_info: Some(BuildInfo {
                build_command: None,
                install_command: if matches!(
                    pm_type,
                    PackageManagerType::Uv | PackageManagerType::Poetry
                ) {
                    None
                } else {
                    dependency_file.map(install_command)
                },
                output_dir: Some(".".to_string()),
                entry_point: entry_point.map(str::to_string),
            }),
            monorepo: None,
            ssr_analysis: None,
            structure: Vec::new(),
        },
        reason: if configured {
            "Configured Python runtime".to_string()
        } else if let Some(entry_point) = entry_point {
            format!("Detected Python process entry {entry_point}")
        } else {
            "Detected Python project; configure its application launch".to_string()
        },
    }
}

pub fn install_command(manifest: &str) -> String {
    install_command_for_minor(manifest, nrz_source_bundle::PythonMinor::default())
}

pub fn install_command_for_minor(manifest: &str, minor: nrz_source_bundle::PythonMinor) -> String {
    let interpreter = format!("python{}", minor.version());
    let root = minor.site_packages_root();
    if manifest == "requirements.txt" {
        format!(
            "{interpreter} -m pip install --disable-pip-version-check --no-compile --target {root} --requirement requirements.txt"
        )
    } else {
        format!(
            "{interpreter} -m pip install --disable-pip-version-check --no-compile --target {root} ."
        )
    }
}

pub fn resolve_entry_point(output_dir: &Path) -> Option<String> {
    PYTHON_ENTRY_CANDIDATES
        .iter()
        .find(|path| output_dir.join(path).is_file())
        .map(|path| (*path).to_string())
}
