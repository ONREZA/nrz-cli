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

fn pinned_bun_build_target() -> anyhow::Result<String> {
    let toolchain: toml::Value = toml::from_str(include_str!("../../mise.toml"))?;
    let version = toolchain
        .get("tools")
        .and_then(|tools| tools.get("bun"))
        .and_then(toml::Value::as_str)
        .context("CLI toolchain has no pinned Bun version")?;
    Ok(format!("bun-{version}"))
}

fn node_build_target(version: &str) -> anyhow::Result<String> {
    match version {
        "NODE_22" => Ok("node-22".into()),
        "NODE_24" => Ok("node-24".into()),
        "NODE_26" => Ok("node-26".into()),
        _ => Err(output::coded_error(
            "APPLICATION_RUNTIME_INVALID",
            "invalid admitted Node version",
        )),
    }
}

pub(crate) fn canonical_build_runtime_target(
    runtime: &RuntimeInfo,
    platform_runner: bool,
    manifest: Option<&crate::build::manifest::Manifest>,
    admitted_node_version: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let authored_node = manifest.is_some_and(|manifest| {
        manifest.layers.iter().any(|layer| {
            layer.target == crate::build::manifest::LayerTarget::Compute
                && layer
                    .runtime
                    .as_ref()
                    .and_then(|runtime| runtime.application_runtime.as_ref())
                    .is_some_and(|intent| intent.family == ApplicationRuntimeFamily::Node)
        })
    });
    if authored_node && !platform_runner && runtime.runtime_type != RuntimeType::Python {
        // Standalone builds preserve the author's declaration. Direct publication
        // binds the independently admitted snapshot before reaching this boundary.
        return admitted_node_version.map(node_build_target).transpose();
    }
    match runtime.runtime_type {
        RuntimeType::Python => {
            let version = crate::detect::python::PYTHON_RUNTIME_VERSION;
            if runtime.version.as_deref() != Some(version) {
                return Err(output::coded_error(
                    "APPLICATION_RUNTIME_INVALID",
                    "Python build declaration differs from the supported selected toolchain",
                ));
            }
            let target = format!("python-{version}");
            if platform_runner && selected_platform_target()? != target {
                return Err(output::coded_error(
                    "APPLICATION_RUNTIME_INVALID",
                    "Python source build declaration differs from the frozen Builder runtime target",
                ));
            }
            Ok(Some(target))
        }
        _ if manifest.is_some_and(|manifest| {
            manifest
                .layers
                .iter()
                .any(|layer| layer.target == crate::build::manifest::LayerTarget::Compute)
        }) || matches!(runtime.runtime_type, RuntimeType::Node | RuntimeType::Bun) =>
        {
            let target = if platform_runner {
                selected_platform_target()?
            } else if let Some(target) = &runtime.version {
                target.clone()
            } else {
                pinned_bun_build_target()?
            };
            nrz_runtime_artifact::source_layer_launch_for_target(None, Some(&target)).map_err(
                |error| output::coded_error("APPLICATION_RUNTIME_INVALID", error.to_string()),
            )?;
            Ok(Some(target))
        }
        _ => Ok(runtime.version.clone()),
    }
}

pub(crate) async fn validate_application_runtime_before_build(
    declaration: Option<&ApplicationRuntimeDeclaration>,
    runtime: &RuntimeInfo,
    effective: &nrz::config::EffectiveProjectConfig,
    platform_runner: bool,
    environment: &[(String, String)],
) -> anyhow::Result<Option<String>> {
    if platform_runner {
        let frozen = effective.platform_application_runtime().ok_or_else(|| {
            output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "runner-context-v5 missing frozen application runtime",
            )
        })?;
        if frozen.as_ref() != declaration {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "source application runtime differs from the frozen deployment snapshot",
            ));
        }
    }
    let Some(declaration) = declaration else {
        if runtime.runtime_type == RuntimeType::Python {
            return canonical_build_runtime_target(
                runtime,
                platform_runner,
                None,
                effective.node_version(),
            );
        }
        return Ok(None);
    };
    let target = if platform_runner {
        selected_platform_target()?
    } else {
        match declaration.family {
            ApplicationRuntimeFamily::Node => node_build_target(effective.node_version().ok_or_else(|| {
                output::coded_error("APPLICATION_RUNTIME_INVALID", "explicit Node runtime requires the project's selected Node version before building")
            })?)?,
            ApplicationRuntimeFamily::Bun => pinned_bun_build_target()?,
        }
    };
    declaration
        .intent()
        .validate_target(Some(&target))
        .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", error))?;
    let binary = match declaration.family {
        ApplicationRuntimeFamily::Node => "node",
        ApplicationRuntimeFamily::Bun => "bun",
    };
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
        && match declaration.family {
            ApplicationRuntimeFamily::Node => {
                actual
                    .strip_prefix('v')
                    .and_then(|version| version.split('.').next())
                    == target.strip_prefix("node-")
            }
            ApplicationRuntimeFamily::Bun => Some(actual) == target.strip_prefix("bun-"),
        };
    if !matches {
        return Err(output::coded_error(
            "APPLICATION_RUNTIME_INVALID",
            format!(
                "local {binary} does not match selected {target}; select the correct executable before building"
            ),
        ));
    }
    Ok(Some(target))
}
