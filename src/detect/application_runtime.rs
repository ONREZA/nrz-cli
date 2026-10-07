//! Resolve application intent before install/build; never execute package scripts.

use anyhow::{Context, bail};
use nrz_source_bundle::{
    ApplicationRuntimeDeclaration, ApplicationRuntimeFamily, BuildToolchainDeclaration,
    BuildToolchainFamily, SourceBuildContext,
};
use serde::Deserialize;

use super::fs::Fs;

#[derive(Debug, Default, Deserialize)]
struct RuntimeProject {
    #[serde(default)]
    project: RuntimeFramework,
    #[serde(default)]
    deploy: RuntimeDeploy,
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeFramework {
    framework: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RuntimeDeploy {
    #[serde(skip)]
    framework_override: Option<String>,
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
    let mut config = read_runtime_config(fs)?;
    config
        .framework_override
        .get_or_insert_with(|| framework.to_string());
    resolve_runtime(fs, framework, config)
}

fn read_runtime_config(fs: &dyn Fs) -> anyhow::Result<RuntimeDeploy> {
    let config = fs
        .read_file("onreza.toml")
        .map(|text| toml::from_str::<RuntimeProject>(&text))
        .transpose()
        .context("invalid application runtime declaration in onreza.toml")?
        .unwrap_or_default();
    Ok(RuntimeDeploy {
        framework_override: config.project.framework,
        ..config.deploy
    })
}

#[allow(dead_code)] // Shared detect module is also compiled by the public library.
pub(crate) fn resolve_and_bind_detection(
    fs: &dyn Fs,
    detection: &mut super::types::DetectionResult,
) -> anyhow::Result<()> {
    let config = fs
        .read_file("onreza.toml")
        .map(|text| toml::from_str::<crate::config::ProjectConfig>(&text))
        .transpose()
        .context("invalid source build context in onreza.toml")?
        .unwrap_or_default();
    resolve_and_bind_source_build_context(
        fs,
        detection,
        &config,
        config.project.framework.as_deref(),
        None,
    )?;
    Ok(())
}

/// Bind frozen launch intent to fresh framework/build-output inference.
#[allow(dead_code)] // CLI-only binding; keep it out of the public library API.
pub(crate) fn bind_application_runtime(
    fs: &dyn Fs,
    detection: &mut super::types::DetectionResult,
    declaration: Option<ApplicationRuntimeDeclaration>,
    configured_entry: Option<&str>,
    framework_override: Option<&str>,
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
                if !super::python::is_python_framework(&detection.framework)
                    && framework_override.is_none()
                {
                    *detection = super::python::detect_configured_python(fs);
                } else if let Some(slug) =
                    framework_override.filter(|slug| super::python::is_python_framework(slug))
                {
                    *detection = super::python::detect_configured_python_framework(fs, slug);
                }
                let minor = declaration.python_version.unwrap_or_default();
                detection.suggested_compute = super::types::ComputeType::Process;
                detection.metadata.runtime.runtime_type = super::types::RuntimeType::Python;
                detection.metadata.runtime.version = Some(minor.version().into());
                if detection
                    .metadata
                    .package_manager
                    .as_ref()
                    .is_some_and(|manager| manager.pm_type == super::types::PackageManagerType::Pip)
                    && let Some(build) = detection.metadata.build_info.as_mut()
                    && let Some(manifest) = super::python::dependency_manifest(fs)
                {
                    build.install_command =
                        Some(super::python::install_command_for_minor(manifest, minor));
                }
            }
            ApplicationRuntimeFamily::Executable => {
                detection.suggested_compute = super::types::ComputeType::Process;
            }
            _ => {}
        }
    }
    let build_toolchain = detection
        .metadata
        .source_build_context
        .as_ref()
        .map(|context| context.build_toolchain.clone())
        .unwrap_or_else(|| default_build_toolchain(detection, declaration.as_ref()));
    detection.metadata.source_build_context = Some(SourceBuildContext {
        schema_version: 1,
        build_toolchain,
        application_runtime: declaration,
    });
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
            framework_override: Some(framework.to_string()),
            ..Default::default()
        },
    )
}

pub fn resolve_application_runtime_with_project(
    fs: &dyn Fs,
    framework: &str,
    deploy: &crate::config::DeploySection,
    framework_override: Option<&str>,
) -> anyhow::Result<Option<ApplicationRuntimeDeclaration>> {
    resolve_runtime(
        fs,
        framework,
        RuntimeDeploy {
            framework_override: framework_override.map(str::to_string),
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
        }, config.framework_override.as_deref())?.context("Python application entry is ambiguous or absent; declare deploy.entry, module or application")?;
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

fn default_build_toolchain(
    detection: &super::types::DetectionResult,
    serving: Option<&ApplicationRuntimeDeclaration>,
) -> BuildToolchainDeclaration {
    use super::types::{PackageManagerType, RuntimeType};
    let family = if serving
        .is_some_and(|runtime| runtime.family == ApplicationRuntimeFamily::Python)
        || detection.metadata.runtime.runtime_type == RuntimeType::Python
    {
        BuildToolchainFamily::Python
    } else if matches!(
        detection.metadata.runtime.runtime_type,
        RuntimeType::Dart | RuntimeType::Go
    ) || serving
        .is_some_and(|runtime| runtime.family == ApplicationRuntimeFamily::Executable)
    {
        BuildToolchainFamily::Native
    } else {
        match detection
            .metadata
            .package_manager
            .as_ref()
            .map(|manager| manager.pm_type)
        {
            Some(PackageManagerType::Bun) => BuildToolchainFamily::Bun,
            Some(PackageManagerType::Npm | PackageManagerType::Yarn | PackageManagerType::Pnpm) => {
                BuildToolchainFamily::Node
            }
            _ if serving.is_some_and(|runtime| runtime.family == ApplicationRuntimeFamily::Bun)
                || detection.metadata.runtime.runtime_type == RuntimeType::Bun =>
            {
                BuildToolchainFamily::Bun
            }
            _ => BuildToolchainFamily::Node,
        }
    };
    BuildToolchainDeclaration {
        family,
        python_version: (family == BuildToolchainFamily::Python).then(|| {
            serving
                .and_then(|runtime| runtime.python_version)
                .unwrap_or_default()
        }),
    }
}

pub fn resolve_build_toolchain(
    detection: &super::types::DetectionResult,
    config: &crate::config::ProjectConfig,
) -> anyhow::Result<BuildToolchainDeclaration> {
    let serving =
        config
            .deploy
            .selected_runtime_family()
            .map(|family| ApplicationRuntimeDeclaration {
                family,
                python_version: config.deploy.python_version,
                entry: None,
                args: vec![],
            });
    let mut build = default_build_toolchain(detection, serving.as_ref());
    if let Some(family) = config.build.selected_toolchain_family() {
        build.family = family;
        build.python_version = (family == BuildToolchainFamily::Python).then(|| {
            config
                .build
                .python_version
                .or(config.deploy.python_version)
                .unwrap_or_default()
        });
        if family != BuildToolchainFamily::Python && config.build.python_version.is_some() {
            bail!("build.python_version requires the Python build toolchain");
        }
    }
    build.validate().map_err(anyhow::Error::msg)?;
    Ok(build)
}

pub(crate) fn resolve_and_bind_source_build_context(
    fs: &dyn Fs,
    detection: &mut super::types::DetectionResult,
    config: &crate::config::ProjectConfig,
    framework_override: Option<&str>,
    explicit_compute: Option<super::types::ComputeType>,
) -> anyhow::Result<SourceBuildContext> {
    let build_toolchain = resolve_build_toolchain(detection, config)?;
    let configured_compute = config
        .deploy
        .compute
        .as_deref()
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "static" => Ok(super::types::ComputeType::Static),
            "process" => Ok(super::types::ComputeType::Process),
            _ => Err(anyhow::anyhow!(
                "invalid compute type; expected static or process"
            )),
        })
        .transpose()?;
    let process_fields = config.deploy.runtime.is_some()
        || config.deploy.entry.is_some()
        || config.deploy.args.is_some()
        || config.deploy.module.is_some()
        || config.deploy.application.is_some()
        || config.deploy.server.is_some()
        || config.deploy.python_version.is_some();
    let selected_compute = explicit_compute.or(configured_compute);
    let static_serving = selected_compute == Some(super::types::ComputeType::Static)
        || selected_compute.is_none()
            && !process_fields
            && detection.suggested_compute == super::types::ComputeType::Static;
    let serving_framework = serving_framework(config, &detection.framework).to_string();
    let application_runtime = if static_serving {
        if config.deploy.runtime.is_some()
            || config.deploy.entry.is_some()
            || config.deploy.args.is_some()
            || config.deploy.module.is_some()
            || config.deploy.application.is_some()
            || config.deploy.server.is_some()
            || config.deploy.python_version.is_some()
        {
            bail!(
                "STATIC serving conflicts with explicit PROCESS launch fields; use build.toolchain and build.python_version for build-only selectors"
            );
        }
        None
    } else {
        let mut deploy = config.deploy.clone();
        if deploy.python_version.is_none()
            && (deploy.selected_runtime_family() == Some(ApplicationRuntimeFamily::Python)
                || deploy.selected_runtime_family().is_none()
                    && super::python::is_python_framework(&detection.framework))
        {
            deploy.python_version = build_toolchain.resolved_python_minor();
        }
        resolve_application_runtime_with_project(
            fs,
            &serving_framework,
            &deploy,
            framework_override,
        )?
    };
    let context = SourceBuildContext {
        schema_version: 1,
        build_toolchain,
        application_runtime,
    };
    bind_source_build_context(
        fs,
        detection,
        &context,
        config,
        framework_override,
        static_serving.then_some(super::types::ComputeType::Static),
    )?;
    Ok(context)
}

/// Keep frozen source intent while refreshing build output inference. Generated
/// package metadata cannot replace the pre-build serving or compiler selection.
pub(crate) fn bind_source_build_context(
    fs: &dyn Fs,
    detection: &mut super::types::DetectionResult,
    context: &SourceBuildContext,
    config: &crate::config::ProjectConfig,
    framework_override: Option<&str>,
    explicit_compute: Option<super::types::ComputeType>,
) -> anyhow::Result<()> {
    context.validate().map_err(anyhow::Error::msg)?;
    let build_framework = detection.framework.clone();
    detection.framework = serving_framework(config, &build_framework).into();
    bind_application_runtime(
        fs,
        detection,
        context.application_runtime.clone(),
        config.deploy.entry.as_deref(),
        framework_override,
    )?;
    if config.build.selected_toolchain_family().is_some() {
        detection.framework = build_framework;
    }
    if explicit_compute == Some(super::types::ComputeType::Static)
        || config.deploy.compute.as_deref() == Some("static")
    {
        detection.suggested_compute = super::types::ComputeType::Static;
    }
    if let Some(minor) = context.build_toolchain.resolved_python_minor() {
        let python = super::python::detect_configured_python(fs);
        detection.metadata.package_manager = python.metadata.package_manager;
        detection.metadata.build_info = python.metadata.build_info;
        if context
            .application_runtime
            .as_ref()
            .is_none_or(|runtime| runtime.family != ApplicationRuntimeFamily::Python)
        {
            detection.metadata.runtime.runtime_type = super::types::RuntimeType::Python;
            detection.metadata.runtime.version = Some(minor.version().into());
        }
        if detection
            .metadata
            .package_manager
            .as_ref()
            .is_some_and(|manager| manager.pm_type == super::types::PackageManagerType::Pip)
            && let Some(build) = detection.metadata.build_info.as_mut()
            && let Some(manifest) = super::python::dependency_manifest(fs)
        {
            build.install_command = Some(super::python::install_command_for_minor(manifest, minor));
        }
    }
    detection.metadata.source_build_context = Some(context.clone());
    Ok(())
}

/// Generic language evidence describes a build recipe. Authored framework presets
/// retain their launch contract; an authored converter can replace an inferred preset.
pub(crate) fn serving_framework<'a>(
    config: &crate::config::ProjectConfig,
    detected: &'a str,
) -> &'a str {
    let selected_runtime = config.deploy.selected_runtime_family();
    let converter = config
        .build
        .command
        .as_ref()
        .is_some_and(|command| !command.trim().is_empty())
        && selected_runtime.is_some();
    let detected_serving_family = match detected {
        "python" => Some(ApplicationRuntimeFamily::Python),
        "go" | "dart" => Some(ApplicationRuntimeFamily::Executable),
        _ => None,
    };
    // Confirming the inferred launch family keeps its entry and build recipe,
    // even when the compiler is selected independently.
    if !converter && selected_runtime.is_some() && selected_runtime == detected_serving_family {
        return detected;
    }
    let generic_declaration = selected_runtime.is_some()
        && detected_serving_family.is_some()
        && selected_runtime != detected_serving_family;
    let independent =
        config.build.selected_toolchain_family().is_some() || converter || generic_declaration;
    if independent
        && config.project.framework.is_none()
        && (matches!(detected, "python" | "go" | "dart") || converter)
    {
        "other"
    } else {
        detected
    }
}
