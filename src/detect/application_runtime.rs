//! Resolve application intent before install/build; never execute package scripts.

use anyhow::{Context, bail};
use nrz_source_bundle::{ApplicationRuntimeDeclaration, ApplicationRuntimeFamily};
use serde::Deserialize;

use super::fs::Fs;

#[derive(Debug, Default, Deserialize)]
struct RuntimeProject {
    #[serde(default)]
    deploy: RuntimeDeploy,
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeDeploy {
    python_version: Option<nrz_source_bundle::PythonMinor>,
    runtime: Option<ApplicationRuntimeFamily>,
    #[serde(alias = "entrypoint")]
    entry: Option<String>,
    args: Option<Vec<String>>,
    module: Option<String>,
    application: Option<String>,
    server: Option<String>,
}

#[allow(dead_code)] // Public library API; the CLI uses resolve_and_bind_detection.
pub fn resolve_application_runtime(
    fs: &dyn Fs,
    framework: &str,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    let config = read_runtime_config(fs)?;
    resolve_runtime(fs, framework, config)
}

fn read_runtime_config(fs: &dyn Fs) -> anyhow::Result<RuntimeDeploy> {
    Ok(fs
        .read_file("onreza.toml")
        .map(|text| toml::from_str::<RuntimeProject>(&text))
        .transpose()
        .context("invalid application runtime declaration in onreza.toml")?
        .unwrap_or_default()
        .deploy)
}

#[allow(dead_code)] // Shared detect module is also compiled by the public library.
pub(crate) fn resolve_and_bind_detection(
    fs: &dyn Fs,
    detection: &mut super::types::DetectionResult,
) -> anyhow::Result<()> {
    let config = read_runtime_config(fs)?;
    let configured_entry = config.entry.clone();
    let declaration = resolve_runtime(fs, &detection.framework, config)?;
    bind_application_runtime(detection, declaration, configured_entry.as_deref())
}

/// Bind frozen launch intent to fresh framework/build-output inference.
#[allow(dead_code)] // CLI-only binding; keep it out of the public library API.
pub(crate) fn bind_application_runtime(
    detection: &mut super::types::DetectionResult,
    declaration: Option<ApplicationRuntimeDeclaration>,
    configured_entry: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(entry) = configured_entry {
        normalize_application_entry(entry)?;
    }
    if let Some(declaration) = &declaration {
        declaration.validate().map_err(anyhow::Error::msg)?;
        validate_framework(&detection.framework, declaration.family)?;
    }
    // Generic static fallback must not erase a configured server launch.
    // Known framework exports and authored manifests retain their own contracts.
    if (declaration.is_some() || configured_entry.is_some())
        && matches!(detection.framework.as_str(), "other" | "static-html")
    {
        detection.suggested_compute = super::types::ComputeType::Process;
    }
    if let Some(declaration) = &declaration {
        match declaration.family {
            ApplicationRuntimeFamily::Python => {
                detection.metadata.runtime.runtime_type = super::types::RuntimeType::Python;
                detection.metadata.runtime.version = Some(
                    declaration
                        .python_version
                        .unwrap_or_default()
                        .version()
                        .into(),
                );
            }
            ApplicationRuntimeFamily::Executable => {
                detection.suggested_compute = super::types::ComputeType::Process;
            }
            _ => {}
        }
    }
    detection.metadata.application_runtime = declaration;
    Ok(())
}

#[allow(dead_code)] // Retained public library API; the CLI uses the complete project declaration.
pub fn resolve_application_runtime_with_config(
    fs: &dyn Fs,
    framework: &str,
    runtime: Option<ApplicationRuntimeFamily>,
    entry: Option<String>,
    args: Option<Vec<String>>,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    resolve_runtime(
        fs,
        framework,
        RuntimeDeploy {
            runtime,
            entry,
            args,
            ..Default::default()
        },
    )
}

pub fn resolve_application_runtime_with_project(
    fs: &dyn Fs,
    framework: &str,
    deploy: &crate::config::DeploySection,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    resolve_runtime(
        fs,
        framework,
        RuntimeDeploy {
            runtime: deploy.runtime,
            python_version: deploy.python_version,
            entry: deploy.entry.clone(),
            args: deploy.args.clone(),
            module: deploy.module.clone(),
            application: deploy.application.clone(),
            server: deploy.server.clone(),
        },
    )
}

fn resolve_runtime(
    fs: &dyn Fs,
    framework: &str,
    mut config: RuntimeDeploy,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    if let Some(entry) = &config.entry {
        config.entry = Some(normalize_application_entry(entry)?);
    }
    let python_fields = config.module.is_some()
        || config.application.is_some()
        || config.server.is_some()
        || config.python_version.is_some();
    if super::python::is_python_framework(framework)
        || config.runtime == Some(ApplicationRuntimeFamily::Python)
        || python_fields
    {
        if config
            .runtime
            .is_some_and(|family| family != ApplicationRuntimeFamily::Python)
        {
            bail!("Python launch conflicts with the declared application runtime");
        }
        let args = config.args.as_deref().unwrap_or_default();
        let launch = super::python_launch::resolve_launch_for_framework(fs, super::python_launch::PythonLaunchRequest {
            entry: config.entry.as_deref(), module: config.module.as_deref(), application: config.application.as_deref(),
            server: config.server.as_deref(), args,
        }, Some(framework))?.context("Python application entry is ambiguous or absent; declare deploy.entry, module or application")?;
        let declaration = ApplicationRuntimeDeclaration {
            family: ApplicationRuntimeFamily::Python,
            python_version: Some(config.python_version.unwrap_or_default()),
            entry: Some(launch.entry),
            args: launch.args,
        };
        validate_framework(framework, declaration.family)?;
        declaration.validate().map_err(anyhow::Error::msg)?;
        return Ok(Some(declaration));
    }
    if matches!(
        super::native::native_recipe(framework),
        Some(super::native::NativeRecipe::DartServer | super::native::NativeRecipe::GoServer)
    ) || config.runtime == Some(ApplicationRuntimeFamily::Executable)
    {
        if config
            .runtime
            .is_some_and(|family| family != ApplicationRuntimeFamily::Executable)
        {
            bail!("native application conflicts with the declared runtime");
        }
        let entry = config.entry.or_else(|| {
            super::native::detect_configured_native(fs, framework).and_then(|detection| {
                detection
                    .metadata
                    .build_info
                    .and_then(|build| build.entry_point)
            })
        });
        let declaration = ApplicationRuntimeDeclaration {
            family: ApplicationRuntimeFamily::Executable,
            python_version: None,
            entry,
            args: config.args.unwrap_or_default(),
        };
        validate_framework(framework, declaration.family)?;
        declaration.validate().map_err(anyhow::Error::msg)?;
        return Ok(Some(declaration));
    }
    let config = RuntimeDeploy {
        runtime: config.runtime,
        entry: config
            .entry
            .map(|entry| normalize_application_entry(&entry))
            .transpose()?,
        args: config.args,
        ..Default::default()
    };
    let package = fs
        .read_file("package.json")
        .map(|text| serde_json::from_str::<super::package_json::PackageJson>(&text))
        .transpose()
        .context("invalid package.json while resolving application runtime")?;
    let script = package
        .as_ref()
        .and_then(|package| package.scripts.get("start"));
    if config.runtime.is_some()
        && script
            .and_then(|script| {
                direct_launcher_family(script.split_whitespace().next().unwrap_or(""))
            })
            .is_some_and(|family| Some(family) != config.runtime)
    {
        bail!("[deploy] runtime conflicts with scripts.start launcher");
    }
    let complete = config.runtime.is_some() && config.entry.is_some() && config.args.is_some();
    let start = if complete {
        None
    } else {
        script
            .map(|script| direct_start(script))
            .transpose()?
            .flatten()
    };
    if config.runtime.is_some()
        && start
            .as_ref()
            .is_some_and(|start| Some(start.family) != config.runtime)
    {
        bail!("[deploy] runtime conflicts with scripts.start launcher");
    }
    let family = config
        .runtime
        .or_else(|| start.as_ref().map(|start| start.family));
    let Some(family) = family else {
        if config.args.is_some() {
            bail!("[deploy] args requires an explicit runtime or direct bun/node start");
        }
        return Ok(None);
    };
    validate_framework(framework, family)?;
    let entry = config
        .entry
        .or_else(|| start.as_ref().and_then(|start| start.entry.clone()));
    let args = config
        .args
        .unwrap_or_else(|| start.map_or_else(Vec::new, |start| start.args));
    let declaration = ApplicationRuntimeDeclaration {
        family,
        python_version: None,
        entry,
        args,
    };
    declaration.validate().map_err(anyhow::Error::msg)?;
    Ok(Some(declaration))
}

pub(crate) fn validate_framework(
    framework: &str,
    family: ApplicationRuntimeFamily,
) -> anyhow::Result<()> {
    let python = super::python::is_python_framework(framework);
    let native = matches!(
        super::native::native_recipe(framework),
        Some(super::native::NativeRecipe::DartServer | super::native::NativeRecipe::GoServer)
    );
    if (python && family != ApplicationRuntimeFamily::Python)
        || (native && family != ApplicationRuntimeFamily::Executable)
        || (framework == "elysia" && family != ApplicationRuntimeFamily::Bun)
        || (matches!(framework, "flutter" | "hugo"))
    {
        bail!("application runtime conflicts with framework {framework}");
    }
    Ok(())
}

pub fn normalize_application_entry(entry: &str) -> anyhow::Result<String> {
    if entry.len() > 4096 {
        bail!("application entry exceeds 4096 UTF-8 bytes");
    }
    let entry = entry.trim().replace('\\', "/");
    let entry = entry.strip_prefix("./").unwrap_or(&entry);
    let mut words = entry.split_whitespace();
    let first = words.next().unwrap_or("");
    if entry.contains(':')
        || (words.next().is_some()
            && matches!(
                first,
                "bun" | "node" | "npm" | "pnpm" | "yarn" | "python" | "python3"
            ))
    {
        bail!("application entry must be a relative file path, not a command or URL");
    }
    nrz_source_bundle::normalize_source_path(entry).map_err(anyhow::Error::msg)
}

fn direct_start(script: &str) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    let mut words = script.split_whitespace();
    let Some(executor) = words.next() else {
        return Ok(None);
    };
    let family = match direct_launcher_family(executor) {
        Some(family) => family,
        None => {
            if words.any(|word| matches!(word.trim_matches(['\'', '"']), "bun" | "node")) {
                bail!(
                    "ambiguous scripts.start launcher; declare [deploy] runtime, entry and args explicitly"
                );
            }
            return Ok(None);
        }
    };
    if script.chars().any(|character| {
        matches!(
            character,
            '\'' | '"'
                | '`'
                | '$'
                | '|'
                | '&'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '\n'
                | '\r'
                | '*'
                | '?'
                | '['
                | ']'
                | '\\'
        )
    }) {
        bail!(
            "unsupported scripts.start shell syntax; declare [deploy] runtime, entry and args explicitly"
        );
    }
    let mut entry = words
        .next()
        .context("scripts.start requires an application file")?;
    if family == ApplicationRuntimeFamily::Bun && entry == "run" {
        entry = words
            .next()
            .context("bun run requires an application file")?;
    }
    if entry.starts_with('-') || !super::looks_like_script_path_token(entry) {
        bail!(
            "unsupported scripts.start interpreter flags or script alias; declare [deploy] runtime, entry and args explicitly"
        );
    }
    Ok(Some(ApplicationRuntimeDeclaration {
        family,
        python_version: None,
        entry: Some(normalize_application_entry(entry)?),
        args: words.map(str::to_string).collect(),
    }))
}

fn direct_launcher_family(executor: &str) -> Option<ApplicationRuntimeFamily> {
    match executor.trim_matches(['\'', '"']) {
        "bun" => Some(ApplicationRuntimeFamily::Bun),
        "node" => Some(ApplicationRuntimeFamily::Node),
        _ => None,
    }
}
