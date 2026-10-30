use anyhow::Context;
use nrz_source_bundle::{ApplicationRuntimeDeclaration, ApplicationRuntimeFamily};

use crate::detect::types::{RuntimeInfo, RuntimeType};
use crate::output;

fn selected_platform_target() -> anyhow::Result<String> {
    std::env::var("ONREZA_RUNTIME_VERSION").map_err(|_| {
        output::coded_error(
            "APPLICATION_RUNTIME_INVALID",
            "trusted runtime version is missing",
        )
    })
}

fn node_build_target(version: &str) -> anyhow::Result<String> {
    match version.parse::<nrz_contract::RuntimeProfile>() {
        Ok(nrz_contract::RuntimeProfile::Node(major)) => Ok(format!("node-{major}")),
        _ => Err(output::coded_error(
            "APPLICATION_RUNTIME_INVALID",
            "invalid admitted Node version syntax",
        )),
    }
}

fn retained_primary_target<'a>(
    manifest: Option<&'a crate::build::manifest::Manifest>,
    declaration: Option<&ApplicationRuntimeDeclaration>,
) -> Option<&'a str> {
    let manifest = manifest?;
    let primary = if let Some(declaration) = declaration {
        manifest
            .layers
            .get(super::select_application_runtime_primary(manifest, declaration).ok()??)?
    } else if super::compute_layer_count(manifest) == 1 {
        manifest
            .layers
            .iter()
            .find(|layer| layer.target == crate::build::manifest::LayerTarget::Compute)?
    } else {
        return None;
    };
    primary.runtime.as_ref()?.build_runtime_version.as_deref()
}

fn needs_local_bun_target(
    runtime: &RuntimeInfo,
    manifest: &crate::build::manifest::Manifest,
    declaration: Option<&ApplicationRuntimeDeclaration>,
) -> bool {
    if retained_primary_target(Some(manifest), declaration).is_some() {
        return false;
    }
    if let Some(declaration) = declaration {
        if declaration.entry.is_none()
            && manifest.layers.iter().any(|layer| {
                layer.runtime.as_ref().is_some_and(|runtime| {
                    runtime.build_runtime_version.is_some()
                        && runtime
                            .application_runtime
                            .as_ref()
                            .is_some_and(|intent| intent.family == declaration.family)
                })
            })
        {
            return false;
        }
        return declaration.family == ApplicationRuntimeFamily::Bun
            && super::compute_layer_count(manifest) > 0;
    }
    if super::compute_layer_count(manifest) != 1 {
        return false;
    }
    let intent = manifest
        .layers
        .iter()
        .find(|layer| layer.target == crate::build::manifest::LayerTarget::Compute)
        .and_then(|layer| layer.runtime.as_ref())
        .and_then(|runtime| runtime.application_runtime.as_ref());
    intent.map_or_else(
        || {
            matches!(
                runtime.runtime_type,
                RuntimeType::Node | RuntimeType::Bun | RuntimeType::Static
            )
        },
        |intent| intent.family == ApplicationRuntimeFamily::Bun,
    )
}

async fn unresolved_local_bun_target(
    runtime: &RuntimeInfo,
    manifest: &crate::build::manifest::Manifest,
    declaration: Option<&ApplicationRuntimeDeclaration>,
    selected_target: Option<&str>,
    platform_runner: bool,
    environment: &[(String, String)],
) -> anyhow::Result<Option<String>> {
    if !platform_runner
        && selected_target.is_none()
        && needs_local_bun_target(runtime, manifest, declaration)
    {
        Ok(Some(measured_bun_target(environment).await?))
    } else {
        Ok(None)
    }
}

pub(crate) async fn manifest_build_runtime_target(
    runtime: &RuntimeInfo,
    effective: &crate::config::EffectiveProjectConfig,
    manifest: &crate::build::manifest::Manifest,
    declaration: Option<&ApplicationRuntimeDeclaration>,
    environment: &[(String, String)],
) -> anyhow::Result<Option<String>> {
    let platform_runner = effective.platform_source_build_context().is_some();
    let local_bun_target = unresolved_local_bun_target(
        runtime,
        manifest,
        declaration,
        effective.application_runtime_target(),
        platform_runner,
        environment,
    )
    .await?;
    let fresh_platform_target = if platform_runner
        && declaration.is_some()
        && effective.primary_runtime_rebuilt()
        && effective.application_runtime_target().is_none()
    {
        Some(selected_platform_target()?)
    } else {
        None
    };
    let implicit_platform_bun_target = if platform_runner
        && declaration.is_none()
        && effective.primary_runtime_rebuilt()
        && effective.application_runtime_target().is_none()
        && matches!(runtime.runtime_type, RuntimeType::Bun | RuntimeType::Static)
        && needs_local_bun_target(runtime, manifest, None)
        && manifest.layers.iter().any(|layer| {
            layer.target == crate::build::manifest::LayerTarget::Compute
                && layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .is_none()
        }) {
        Some(
            if effective
                .platform_source_build_context()
                .is_some_and(|context| {
                    context.build_toolchain.family == nrz_source_bundle::BuildToolchainFamily::Bun
                })
            {
                // Pre-build qualification already checked this exact compiler target.
                std::env::var("ONREZA_BUILD_RUNTIME_VERSION")
                    .context("trusted Bun compiler target is missing")?
            } else {
                measured_bun_target(environment).await?
            },
        )
    } else {
        None
    };
    canonical_build_runtime_target(
        runtime,
        platform_runner,
        effective
            .application_runtime_target()
            .filter(|_| {
                effective.primary_runtime_rebuilt()
                    || ((!platform_runner
                        || declaration.is_some_and(|declaration| {
                            declaration.family == ApplicationRuntimeFamily::Bun
                        }))
                        && needs_local_bun_target(runtime, manifest, declaration))
            })
            .or(fresh_platform_target.as_deref())
            .or(implicit_platform_bun_target.as_deref())
            .or(local_bun_target.as_deref()),
        Some(manifest),
        declaration,
        if effective.primary_runtime_rebuilt() {
            effective.node_version()
        } else {
            effective
                .requested_node_version()
                .or_else(|| effective.node_version())
        },
        effective.config().build.toolchain.is_none()
            && effective.config().build.python_version.is_none(),
    )
}

pub(crate) fn canonical_build_runtime_target(
    runtime: &RuntimeInfo,
    platform_runner: bool,
    selected_application_target: Option<&str>,
    manifest: Option<&crate::build::manifest::Manifest>,
    declaration: Option<&ApplicationRuntimeDeclaration>,
    _selected_node_version: Option<&str>,
    legacy_manifest_selection: bool,
) -> anyhow::Result<Option<String>> {
    if let Some(declaration) = declaration {
        if (platform_runner
            || (declaration.family == ApplicationRuntimeFamily::Bun
                && retained_primary_target(manifest, Some(declaration)).is_none()))
            && selected_application_target.is_none()
            && manifest.is_some_and(|manifest| {
                manifest.layers.iter().any(|layer| {
                    layer.target == crate::build::manifest::LayerTarget::Compute
                        && layer.runtime.as_ref().is_some_and(|runtime| {
                            runtime.application_runtime.is_some()
                                && runtime.build_runtime_version.is_some()
                        })
                })
            })
        {
            // No new build facts: the strict binder validates each retained
            // owner against the original declaration and its own target.
            return Ok(None);
        }
        let retained_target = retained_primary_target(manifest, Some(declaration));
        let target = if let Some(target) = selected_application_target {
            target.to_string()
        } else if let Some(target) = retained_target {
            target.to_string()
        } else {
            match declaration.family {
                ApplicationRuntimeFamily::Python => "python".into(),
                ApplicationRuntimeFamily::Executable => {
                    nrz_runtime_artifact::NATIVE_EXECUTION_TARGET.into()
                }
                ApplicationRuntimeFamily::Node => "node".into(),
                ApplicationRuntimeFamily::Bun => selected_application_target
                    .or(runtime.version.as_deref())
                    .context("local Bun application target is missing")?
                    .to_string(),
            }
        };
        declaration
            .validate_target(Some(&target))
            .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", error))?;
        return Ok(Some(target));
    }
    // A sole legacy JavaScript COMPUTE owner can select its serving target.
    // Typed frozen owners retain their own targets: compiler metadata and
    // admitted Node versions do not declare a primary serving runtime.
    // Standalone Node builds remain unresolved until independent admission.
    if legacy_manifest_selection
        && let Some(manifest) = manifest
        && super::compute_layer_count(manifest) == 1
        && let Some(layer) = manifest
            .layers
            .iter()
            .find(|layer| layer.target == crate::build::manifest::LayerTarget::Compute)
        && layer
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.build_runtime_version.as_ref())
            .is_none()
        && matches!(
            runtime.runtime_type,
            RuntimeType::Node | RuntimeType::Bun | RuntimeType::Static
        )
        && let Some(intent) = layer
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.application_runtime.as_ref())
    {
        match intent.family {
            ApplicationRuntimeFamily::Node => {
                return Ok(Some("node".into()));
            }
            ApplicationRuntimeFamily::Bun => {
                return selected_application_target
                    .or(runtime.version.as_deref())
                    .map(str::to_owned)
                    .map(Some)
                    .context("local Bun application target is missing");
            }
            _ => {}
        }
    }
    // Typed layers own their target even when the primary serves STATIC.
    let untyped_compute = manifest.is_some_and(|manifest| {
        manifest.layers.iter().any(|layer| {
            layer.target == crate::build::manifest::LayerTarget::Compute
                && layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .is_none()
        })
    });
    if !untyped_compute {
        return Ok(None);
    }
    if matches!(
        runtime.runtime_type,
        RuntimeType::Node | RuntimeType::Bun | RuntimeType::Static | RuntimeType::Python
    ) {
        let target = selected_application_target
            .map(str::to_owned)
            .or_else(|| retained_primary_target(manifest, None).map(str::to_owned))
            .or_else(|| match runtime.runtime_type {
                RuntimeType::Node => Some("node".into()),
                RuntimeType::Python => Some("python".into()),
                _ => None,
            });
        if let Some(target) = target.as_deref() {
            let config = (runtime.runtime_type == RuntimeType::Python)
                .then(|| serde_json::json!({"runtimeFamily":"PYTHON"}));
            nrz_runtime_artifact::source_layer_launch_for_target(config.as_ref(), Some(target))
                .map_err(|error| {
                    output::coded_error("APPLICATION_RUNTIME_INVALID", error.to_string())
                })?;
        }
        return Ok(target);
    }
    Err(output::coded_error(
        "APPLICATION_RUNTIME_INVALID",
        "a COMPUTE layer requires its own application runtime and frozen target when its build toolchain does not select a server",
    ))
}

#[derive(Debug)]
pub(crate) struct ResolvedBuildRuntime {
    pub context: nrz_source_bundle::SourceBuildContext,
    pub application_target: Option<String>,
    pub node_version: Option<String>,
    pub python_interpreter: Option<std::path::PathBuf>,
}

fn resolve_platform_runtime(
    context: &nrz_source_bundle::SourceBuildContext,
    compiler: &str,
    application: &str,
    ancillary_node_major: Option<&str>,
) -> anyhow::Result<ResolvedBuildRuntime> {
    use nrz_source_bundle::{ApplicationRuntimeIntent, BuildToolchainFamily, PythonMinor};
    let mut resolved = context.clone();
    let compiler_family = match context.build_toolchain.family {
        BuildToolchainFamily::Node => ApplicationRuntimeFamily::Node,
        BuildToolchainFamily::Bun => ApplicationRuntimeFamily::Bun,
        BuildToolchainFamily::Python => ApplicationRuntimeFamily::Python,
        BuildToolchainFamily::Native => ApplicationRuntimeFamily::Executable,
    };
    ApplicationRuntimeIntent {
        family: compiler_family,
        args: vec![],
    }
    .validate_target(Some(compiler))
    .map_err(anyhow::Error::msg)?;
    if compiler_family == ApplicationRuntimeFamily::Python {
        resolved.build_toolchain.python_version =
            PythonMinor::from_target(compiler).map(PythonMinor::version);
    }
    let application_target = if let Some(declaration) = &resolved.application_runtime {
        declaration
            .intent()
            .validate_target(Some(application))
            .map_err(anyhow::Error::msg)?;
        Some(application.to_string())
    } else {
        None
    };
    let node_target = application_target
        .as_deref()
        .filter(|target| target.starts_with("node-"))
        .or_else(|| compiler.starts_with("node-").then_some(compiler));
    let node_version = if let Some(target) = node_target {
        match nrz_contract::RuntimeProfile::node_build_target(target) {
            Some(nrz_contract::RuntimeProfile::Node(major)) => Some(format!("NODE_{major}")),
            _ => anyhow::bail!("frozen Node compiler target has invalid syntax"),
        }
    } else {
        ancillary_node_major.map(|major| format!("NODE_{major}"))
    };
    if let Some(version) = &node_version {
        node_build_target(version)?;
    }
    resolved.validate().map_err(anyhow::Error::msg)?;
    Ok(ResolvedBuildRuntime {
        context: resolved,
        application_target,
        node_version,
        python_interpreter: None,
    })
}

pub(crate) async fn validate_application_runtime_before_build(
    context: &nrz_source_bundle::SourceBuildContext,
    effective: &nrz::config::EffectiveProjectConfig,
    platform_runner: bool,
    execute_tools: bool,
    environment: &[(String, String)],
) -> anyhow::Result<ResolvedBuildRuntime> {
    use nrz_source_bundle::BuildToolchainFamily;
    context
        .validate()
        .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", error))?;
    let mut resolved = if platform_runner {
        let frozen = effective
            .platform_source_build_context()
            .context("runner-context-v6 missing frozen source build context")?;
        if frozen != context {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "source build context differs from the frozen deployment snapshot",
            ));
        }
        let compiler = std::env::var("ONREZA_BUILD_RUNTIME_VERSION")
            .context("trusted compiler target is missing")?;
        let expected_family = match context.build_toolchain.family {
            BuildToolchainFamily::Python => "python",
            BuildToolchainFamily::Native => "native",
            BuildToolchainFamily::Node | BuildToolchainFamily::Bun => "javascript",
        };
        if std::env::var("ONREZA_BUILD_RUNTIME_FAMILY").as_deref() != Ok(expected_family) {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "compiler family differs from the frozen source build toolchain",
            ));
        }
        let application = if context.application_runtime.is_some() {
            selected_platform_target()?
        } else {
            String::new()
        };
        resolve_platform_runtime(
            context,
            &compiler,
            &application,
            std::env::var("ONREZA_BUILD_NODE_MAJOR").ok().as_deref(),
        )?
    } else {
        ResolvedBuildRuntime {
            context: context.clone(),
            application_target: None,
            node_version: effective.node_version().map(str::to_owned),
            python_interpreter: None,
        }
    };
    if execute_tools
        && (context.build_toolchain.family == BuildToolchainFamily::Python
            || !platform_runner
                && context
                    .application_runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.family == ApplicationRuntimeFamily::Python))
    {
        let interpreter = super::python_toolchain::available_interpreter(environment).await?;
        if !platform_runner
            && context
                .application_runtime
                .as_ref()
                .is_some_and(|runtime| runtime.family == ApplicationRuntimeFamily::Python)
        {
            resolved.application_target = Some(format!("python-{}", interpreter.version));
        }
        bind_python_compiler(&mut resolved, interpreter);
    }
    let mut measured_bun = None;
    let mut measured_node = None;
    if execute_tools {
        let compiler_target = if platform_runner {
            Some(
                std::env::var("ONREZA_BUILD_RUNTIME_VERSION")
                    .context("trusted compiler target is missing")?,
            )
        } else {
            None
        };
        match context.build_toolchain.family {
            BuildToolchainFamily::Node => {
                measured_node = Some(
                    qualify_javascript_binary("node", compiler_target.as_deref(), environment)
                        .await?,
                );
            }
            BuildToolchainFamily::Bun => {
                measured_bun = Some(
                    qualify_javascript_binary("bun", compiler_target.as_deref(), environment)
                        .await?,
                );
            }
            _ => {}
        }
        if platform_runner {
            if let Ok(major) = std::env::var("ONREZA_BUILD_NODE_MAJOR") {
                let auxiliary_target = format!("node-{major}");
                if context.build_toolchain.family != BuildToolchainFamily::Node
                    || compiler_target.as_deref() != Some(auxiliary_target.as_str())
                {
                    measured_node = Some(
                        qualify_javascript_binary("node", Some(&auxiliary_target), environment)
                            .await?,
                    );
                }
            } else if context.build_toolchain.family == BuildToolchainFamily::Bun {
                return Err(output::coded_error(
                    "APPLICATION_RUNTIME_INVALID",
                    "frozen Bun compiler requires its ancillary Node major",
                ));
            }
        }
    }
    if !platform_runner
        && execute_tools
        && measured_node.is_none()
        && context
            .application_runtime
            .as_ref()
            .is_some_and(|runtime| runtime.family == ApplicationRuntimeFamily::Node)
    {
        measured_node = Some(qualify_javascript_binary("node", None, environment).await?);
    }
    if let Some(actual) = measured_node {
        let major = actual
            .strip_prefix('v')
            .unwrap_or(&actual)
            .split('.')
            .next()
            .context("observed Node version is missing")?;
        resolved.node_version = Some(format!("NODE_{major}"));
    }
    if !platform_runner {
        resolved.application_target = match context
            .application_runtime
            .as_ref()
            .map(|runtime| runtime.family)
        {
            Some(ApplicationRuntimeFamily::Bun) if execute_tools => Some(match measured_bun {
                Some(actual) => format!("bun-{actual}"),
                None => measured_bun_target(environment).await?,
            }),
            Some(ApplicationRuntimeFamily::Bun) => None,
            Some(ApplicationRuntimeFamily::Node) => resolved
                .node_version
                .as_deref()
                .map(node_build_target)
                .transpose()?,
            Some(ApplicationRuntimeFamily::Python) => resolved.application_target.clone(),
            Some(ApplicationRuntimeFamily::Executable) => {
                Some(nrz_runtime_artifact::NATIVE_EXECUTION_TARGET.into())
            }
            None => None,
        };
    }
    Ok(resolved)
}

fn bind_python_compiler(
    resolved: &mut ResolvedBuildRuntime,
    interpreter: super::python_toolchain::PythonInterpreter,
) {
    if resolved.context.build_toolchain.family == nrz_source_bundle::BuildToolchainFamily::Python {
        resolved.context.build_toolchain.python_version = Some(interpreter.minor.version());
    }
    resolved.python_interpreter = Some(interpreter.path);
}

pub(crate) async fn measured_bun_target(
    environment: &[(String, String)],
) -> anyhow::Result<String> {
    let actual = qualify_javascript_binary("bun", None, environment).await?;
    Ok(format!("bun-{actual}"))
}

async fn qualify_javascript_binary(
    binary: &str,
    target: Option<&str>,
    environment: &[(String, String)],
) -> anyhow::Result<String> {
    let mut command = tokio::process::Command::new(binary);
    command
        .arg("--version")
        .envs(environment.iter().map(|(name, value)| (name, value)))
        .kill_on_drop(true);
    let result = tokio::time::timeout(std::time::Duration::from_secs(8), command.output())
        .await
        .map_err(|_| {
            output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "runtime version probe timed out",
            )
        })?
        .map_err(|_| {
            output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                format!("{binary} is unavailable; install the selected runtime before building"),
            )
        })?;
    let actual = std::str::from_utf8(&result.stdout).unwrap_or("").trim();
    let matches = result.status.success()
        && match binary {
            "node" => {
                let version = actual.strip_prefix('v');
                let observed = version.and_then(|version| {
                    nrz_contract::RuntimeProfile::node_build_target(&format!("node-{version}"))
                });
                matches!(observed, Some(nrz_contract::RuntimeProfile::Node(_)))
                    && target.is_none_or(|target| {
                        matches!(
                            nrz_contract::RuntimeProfile::node_build_target(target),
                            Some(
                                nrz_contract::RuntimeProfile::Node(_)
                                    | nrz_contract::RuntimeProfile::NodeAny
                            )
                        )
                    })
            }
            "bun" => target.map_or_else(
                || {
                    nrz_source_bundle::application_runtime::supported_bun_build_target(&format!(
                        "bun-{actual}"
                    ))
                },
                |target| Some(actual) == target.strip_prefix("bun-"),
            ),
            _ => false,
        };
    if !matches {
        return Err(output::coded_error(
            "APPLICATION_RUNTIME_INVALID",
            if binary == "node" {
                "available node must report its canonical Node version".to_owned()
            } else {
                format!(
                    "local {binary} does not match selected {}; select the correct executable before building",
                    target.unwrap_or("canonical runtime version")
                )
            },
        ));
    }
    Ok(actual.to_string())
}

#[cfg(test)]
#[path = "application_runtime_tests.rs"]
mod tests;
