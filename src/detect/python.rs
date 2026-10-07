//! Python project evidence and deployment dependency inputs.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, bail};
use pep508_rs::pep440_rs::{Operator, VersionSpecifiers};
use pep508_rs::{
    MarkerEnvironment, MarkerEnvironmentBuilder, MarkerTree, MarkerTreeKind, MarkerValueString,
    Requirement,
};

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
            || project_name.is_some()
                && fs.list_dir("src").into_iter().any(|path| {
                    fs.is_dir(&format!("src/{path}"))
                        && fs.exists(&format!("src/{path}/__init__.py"))
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
    requires_dependency_stage_with_environment(fs, None)
}

/// Serving admission evaluates only values fixed by the managed Linux runtime.
/// Detection itself keeps its conservative dependency and framework evidence.
pub fn requires_dependency_stage_for_target(
    fs: &dyn Fs,
    minor: nrz_source_bundle::PythonMinor,
) -> anyhow::Result<bool> {
    if !requires_dependency_stage(fs)? {
        return Ok(false);
    }
    let environment = MarkerEnvironment::try_from(MarkerEnvironmentBuilder {
        implementation_name: "cpython",
        implementation_version: minor.exact_version(),
        os_name: "posix",
        platform_machine: "x86_64",
        platform_python_implementation: "CPython",
        // These values are never evaluated: the AST guard treats them as unknown.
        platform_release: "",
        platform_system: "Linux",
        platform_version: "",
        python_full_version: minor.exact_version(),
        python_version: minor.version(),
        sys_platform: "linux",
    })
    .context("invalid frozen Python marker environment")?;
    requires_dependency_stage_with_environment(fs, Some(&environment))
}

fn requires_dependency_stage_with_environment(
    fs: &dyn Fs,
    environment: Option<&MarkerEnvironment>,
) -> anyhow::Result<bool> {
    let plan = dependency_plan(fs)?;
    if plan.as_ref().is_some_and(|plan| plan.install_project) {
        return Ok(true);
    }
    if plan
        .as_ref()
        .is_some_and(|plan| plan.kind == PythonDependencyKind::Requirements)
    {
        return Ok(fs.read_file("requirements.txt").is_some_and(|text| {
            unfold_requirements(&text).lines().any(|line| {
                let line = line.trim();
                !line.is_empty()
                    && !line.starts_with('#')
                    && (!line.starts_with('-')
                        || line.starts_with("-r")
                        || line.starts_with("--requirement")
                        || line.starts_with("-e")
                        || line.starts_with("--editable"))
                    && environment
                        .is_none_or(|environment| requirement_applies_to_target(line, environment))
            })
        }));
    }
    if plan
        .as_ref()
        .is_some_and(|plan| plan.manifest == "pyproject.toml")
        && let Some(text) = fs.read_file("pyproject.toml")
    {
        let value: toml::Value = toml::from_str(&text).context("invalid pyproject.toml")?;
        let project = value.get("project");
        let dependencies = value
            .get("project")
            .and_then(|project| project.get("dependencies"))
            .and_then(toml::Value::as_array);
        if let Some(environment) = environment {
            let dynamic = project
                .and_then(|project| project.get("dynamic"))
                .and_then(toml::Value::as_array)
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| field.as_str() == Some("dependencies"))
                });
            if let Some(dependencies) = dependencies {
                return Ok(dynamic
                    || dependencies.iter().any(|requirement| {
                        requirement.as_str().is_none_or(|requirement| {
                            requirement_applies_to_target(requirement, environment)
                        })
                    }));
            }
            if plan
                .as_ref()
                .is_some_and(|plan| plan.kind == PythonDependencyKind::Poetry)
                && let Some(dependencies) = value
                    .get("tool")
                    .and_then(|tool| tool.get("poetry"))
                    .and_then(|poetry| poetry.get("dependencies"))
                    .and_then(toml::Value::as_table)
            {
                return Ok(dependencies.iter().any(|(name, requirement)| {
                    name != "python"
                        && poetry_requirement_applies_to_target(requirement, environment)
                }));
            }
        }
        if dependencies.is_some_and(|dependencies| {
            dependencies
                .iter()
                .filter_map(toml::Value::as_str)
                .any(|requirement| !requirement.trim().is_empty())
        }) {
            return Ok(true);
        }
    }
    if !dependency_names(fs)?.is_empty() {
        return Ok(true);
    }
    // Backend-provided dependencies require a stage even when own-package
    // installation is disabled. Detection cannot prove their graph is empty.
    Ok(plan.is_some_and(|plan| plan.manifest == "pyproject.toml")
        && !framework_evidence_complete(fs)?)
}

fn poetry_requirement_applies_to_target(
    requirement: &toml::Value,
    environment: &MarkerEnvironment,
) -> bool {
    if let Some(alternatives) = requirement.as_array() {
        return alternatives.is_empty()
            || alternatives
                .iter()
                .any(|requirement| poetry_requirement_applies_to_target(requirement, environment));
    }
    if requirement.get("optional").and_then(toml::Value::as_bool) == Some(true) {
        return false;
    }
    let mut applies = true;
    if let Some(platform) = requirement.get("platform") {
        let Some(platform) = platform.as_str().filter(|platform| {
            !platform.is_empty()
                && platform
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'-'))
        }) else {
            return true;
        };
        let marker = format!("sys_platform == '{platform}'");
        let Some(result) = MarkerTree::parse_str::<url::Url>(&marker)
            .ok()
            .and_then(|marker| marker_matches_known_target(&marker, environment))
        else {
            return true;
        };
        applies &= result;
    }
    if let Some(python) = requirement.get("python") {
        let Some(specifiers) = python
            .as_str()
            .and_then(|python| VersionSpecifiers::from_str(python).ok())
        else {
            return true;
        };
        for specifier in specifiers.iter() {
            let version = specifier.version();
            if version.epoch() != 0
                || version.is_pre()
                || version.is_post()
                || version.is_dev()
                || version.is_local()
            {
                return true;
            }
            // Poetry preserves minor equality, but uses the full version for
            // strict lower and inclusive upper bounds and exact exclusions.
            // Unqualified operators remain opaque rather than inventing ranges.
            let key = match specifier.operator() {
                Operator::Equal | Operator::LessThan | Operator::GreaterThanEqual => {
                    if version.release().len() <= 2 {
                        "python_version"
                    } else {
                        "python_full_version"
                    }
                }
                Operator::NotEqual | Operator::LessThanEqual | Operator::GreaterThan => {
                    "python_full_version"
                }
                _ => return true,
            };
            let marker = format!("{key} {} '{version}'", specifier.operator());
            let Some(result) = MarkerTree::parse_str::<url::Url>(&marker)
                .ok()
                .and_then(|marker| marker_matches_known_target(&marker, environment))
            else {
                return true;
            };
            applies &= result;
        }
    }
    if let Some(marker) = requirement.get("markers") {
        let Some(result) = marker
            .as_str()
            .and_then(|marker| MarkerTree::parse_str::<url::Url>(marker).ok())
            .and_then(|marker| marker_matches_known_target(&marker, environment))
        else {
            return true;
        };
        applies &= result;
    }
    applies
}

fn requirement_applies_to_target(requirement: &str, environment: &MarkerEnvironment) -> bool {
    if requirement.trim().is_empty() {
        return false;
    }
    Requirement::<url::Url>::from_str(requirement).map_or(true, |requirement| {
        marker_applies_to_target(&requirement.marker, environment)
    })
}

fn marker_applies_to_target(marker: &MarkerTree, environment: &MarkerEnvironment) -> bool {
    marker_matches_known_target(marker, environment).unwrap_or(true)
}

fn marker_matches_known_target(
    marker: &MarkerTree,
    environment: &MarkerEnvironment,
) -> Option<bool> {
    let unknown_key = |key: &MarkerValueString| {
        matches!(
            key,
            MarkerValueString::PlatformRelease
                | MarkerValueString::PlatformVersion
                | MarkerValueString::PlatformVersionDeprecated
        )
    };
    let mut pending = vec![marker.clone()];
    let mut visited = HashSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(node.clone()) {
            continue;
        }
        match node.kind() {
            MarkerTreeKind::True | MarkerTreeKind::False => {}
            MarkerTreeKind::Version(node) => pending.extend(node.edges().map(|(_, child)| child)),
            MarkerTreeKind::String(node) => {
                if unknown_key(node.key()) {
                    return None;
                }
                pending.extend(node.children().map(|(_, child)| child));
            }
            MarkerTreeKind::In(node) => {
                if unknown_key(node.key()) {
                    return None;
                }
                pending.extend(node.children().map(|(_, child)| child));
            }
            MarkerTreeKind::Contains(node) => {
                if unknown_key(node.key()) {
                    return None;
                }
                pending.extend(node.children().map(|(_, child)| child));
            }
            MarkerTreeKind::Extra(_) => return None,
        }
    }
    Some(marker.evaluate(environment, &[]))
}

pub fn dependency_names(fs: &dyn Fs) -> anyhow::Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    let manifest = dependency_manifest(fs);
    if let Some(text) = (manifest == Some("requirements.txt"))
        .then(|| fs.read_file("requirements.txt"))
        .flatten()
    {
        for requirement in unfold_requirements(&text).lines() {
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

/// All requirement evidence uses logical lines. Full-line comments terminate
/// continuations instead of hiding the following requirement, as in pip.
fn unfold_requirements(text: &str) -> String {
    let mut unfolded = String::with_capacity(text.len());
    for line in text.lines() {
        let comment = line.trim_start().starts_with('#');
        if !comment && let Some(prefix) = line.strip_suffix('\\') {
            unfolded.push_str(prefix);
            continue;
        }
        if comment {
            // A comment following a continued requirement must remain a comment.
            unfolded.push(' ');
        }
        unfolded.push_str(line);
        unfolded.push('\n');
    }
    unfolded
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
            !unfold_requirements(&text).lines().any(|line| {
                let line = line.trim();
                line.starts_with("-r")
                    || line.starts_with("--requirement")
                    || conditional_framework(line)
            })
        })),
        Some("setup.py") => Ok(false),
        Some("pyproject.toml") => {
            let Some(text) = fs.read_file("pyproject.toml") else {
                return Ok(true);
            };
            let value: toml::Value = toml::from_str(&text)?;
            let project = value.get("project");
            let dynamic_dependencies = project
                .and_then(|project| project.get("dynamic"))
                .and_then(toml::Value::as_array)
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| field.as_str() == Some("dependencies"))
                });
            if let Some(dependencies) = project
                .and_then(|project| project.get("dependencies"))
                .and_then(toml::Value::as_array)
            {
                return Ok(!dynamic_dependencies
                    && !dependencies
                        .iter()
                        .filter_map(toml::Value::as_str)
                        .any(conditional_framework));
            }
            let plan = dependency_plan(fs)?;
            if plan
                .as_ref()
                .is_some_and(|plan| plan.kind == PythonDependencyKind::Poetry)
                && let Some(dependencies) = value
                    .get("tool")
                    .and_then(|tool| tool.get("poetry"))
                    .and_then(|poetry| poetry.get("dependencies"))
                    .and_then(toml::Value::as_table)
            {
                return Ok(!dependencies.iter().any(|(name, requirement)| {
                    is_python_framework(&normalize_package_name(name))
                        && name != "python"
                        && requirement.get("optional").and_then(toml::Value::as_bool) != Some(true)
                        && (requirement.is_array()
                            || ["markers", "python", "platform"]
                                .iter()
                                .any(|key| requirement.get(*key).is_some()))
                }));
            }
            if dynamic_dependencies {
                return Ok(false);
            }
            // A named PEP 621 project without dynamic dependencies has an
            // authoritative empty runtime graph when dependencies are omitted.
            if project
                .and_then(|project| project.get("name"))
                .and_then(toml::Value::as_str)
                .is_some()
            {
                return Ok(true);
            }
            Ok(value.get("build-system").is_none()
                && !["setup.py", "setup.cfg"]
                    .iter()
                    .any(|file| fs.exists(file) && !fs.is_dir(file))
                && !plan.is_some_and(|plan| plan.install_project))
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
            "Python framework inference is incomplete because dependencies use dynamic package metadata, conditional framework requirements or requirements includes; declare deploy.entry, module, application with server, or project.framework explicitly"
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

pub(crate) fn has_application_entry(fs: &dyn Fs) -> bool {
    has_entry(fs) || super::python_launch::has_declared_console_scripts(fs)
}

pub fn detect_python(fs: &dyn Fs) -> Option<DetectionResult> {
    let entry_point = PYTHON_ENTRY_CANDIDATES
        .iter()
        .find(|path| fs.exists(path) && !fs.is_dir(path))
        .copied();
    let has_manifest = dependency_manifest(fs).is_some();
    if fs.exists("package.json") && (!has_application_entry(fs) || !has_manifest) {
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
