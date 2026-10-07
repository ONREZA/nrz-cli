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
    declaration: Option<&ApplicationRuntimeDeclaration>,
    admitted_node_version: Option<&str>,
    legacy_manifest_selection: bool,
) -> anyhow::Result<Option<String>> {
    if let Some(declaration) = declaration {
        let target = match declaration.family {
            ApplicationRuntimeFamily::Python => format!(
                "python-{}",
                declaration
                    .python_version
                    .context("frozen Python serving minor is missing")?
                    .version()
            ),
            ApplicationRuntimeFamily::Executable => {
                nrz_runtime_artifact::NATIVE_EXECUTION_TARGET.into()
            }
            ApplicationRuntimeFamily::Node => match admitted_node_version {
                Some(version) => node_build_target(version)?,
                None => return Ok(None),
            },
            ApplicationRuntimeFamily::Bun => pinned_bun_build_target()?,
        };
        declaration
            .validate_target(Some(&target))
            .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", error))?;
        return Ok(Some(target));
    }
    // A sole legacy JavaScript layer is the primary authored serving owner.
    // Standalone Node builds remain unresolved until independent admission.
    if legacy_manifest_selection
        && let Some(manifest) = manifest
        && manifest.layers.len() == 1
        && manifest.layers[0].target == crate::build::manifest::LayerTarget::Compute
        && matches!(
            runtime.runtime_type,
            RuntimeType::Node | RuntimeType::Bun | RuntimeType::Static
        )
        && let Some(intent) = manifest.layers[0]
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.application_runtime.as_ref())
    {
        match intent.family {
            ApplicationRuntimeFamily::Node => {
                return admitted_node_version.map(node_build_target).transpose();
            }
            ApplicationRuntimeFamily::Bun => return pinned_bun_build_target().map(Some),
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
        RuntimeType::Node | RuntimeType::Bun | RuntimeType::Static
    ) {
        let target = if platform_runner {
            selected_platform_target()?
        } else {
            runtime
                .version
                .clone()
                .unwrap_or(pinned_bun_build_target()?)
        };
        if !target.starts_with("node-") && !target.starts_with("bun-") {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "an untyped JavaScript COMPUTE layer requires a JavaScript dependency ABI target",
            ));
        }
        nrz_runtime_artifact::source_layer_launch_for_target(None, Some(&target)).map_err(
            |error| output::coded_error("APPLICATION_RUNTIME_INVALID", error.to_string()),
        )?;
        return Ok(Some(target));
    }
    Err(output::coded_error(
        "APPLICATION_RUNTIME_INVALID",
        "a COMPUTE layer requires its own application runtime and frozen target when its build toolchain does not select a server",
    ))
}

pub(crate) async fn validate_application_runtime_before_build(
    context: &nrz_source_bundle::SourceBuildContext,
    effective: &nrz::config::EffectiveProjectConfig,
    platform_runner: bool,
    execute_tools: bool,
    environment: &[(String, String)],
) -> anyhow::Result<Option<String>> {
    use nrz_source_bundle::BuildToolchainFamily;
    context
        .validate()
        .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", error))?;
    if platform_runner {
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
        if let Ok(major) = std::env::var("ONREZA_BUILD_NODE_MAJOR") {
            let selected = node_build_target(
                effective
                    .node_version()
                    .context("ancillary Node requires the frozen Node version")?,
            )?;
            if selected.strip_prefix("node-") != Some(major.as_str()) {
                return Err(output::coded_error(
                    "APPLICATION_RUNTIME_INVALID",
                    "ancillary Node major differs from the frozen Node version",
                ));
            }
        }
        let expected = match context.build_toolchain.family {
            BuildToolchainFamily::Python => format!(
                "python-{}",
                context
                    .build_toolchain
                    .resolved_python_minor()
                    .context("frozen Python build minor is missing")?
                    .version()
            ),
            BuildToolchainFamily::Node => node_build_target(
                effective
                    .node_version()
                    .context("frozen Node version is missing")?,
            )?,
            BuildToolchainFamily::Bun => pinned_bun_build_target()?,
            BuildToolchainFamily::Native => nrz_runtime_artifact::NATIVE_EXECUTION_TARGET.into(),
        };
        if compiler != expected {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "compiler target differs from the frozen source build toolchain",
            ));
        }
    }
    if platform_runner || execute_tools {
        let compiler_target = match context.build_toolchain.family {
            BuildToolchainFamily::Node => effective
                .node_version()
                .map(node_build_target)
                .transpose()?,
            BuildToolchainFamily::Bun => Some(pinned_bun_build_target()?),
            _ => None,
        };
        match context.build_toolchain.family {
            BuildToolchainFamily::Node => {
                qualify_javascript_binary("node", compiler_target.as_deref(), environment).await?
            }
            BuildToolchainFamily::Bun => {
                qualify_javascript_binary("bun", compiler_target.as_deref(), environment).await?
            }
            _ => {}
        }
        if let Ok(major) = std::env::var("ONREZA_BUILD_NODE_MAJOR") {
            qualify_javascript_binary("node", Some(&format!("node-{major}")), environment).await?;
        } else if platform_runner && context.build_toolchain.family == BuildToolchainFamily::Bun {
            return Err(output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "frozen Bun compiler requires its ancillary Node major",
            ));
        }
    }
    let Some(declaration) = context.application_runtime.as_ref() else {
        return Ok(None);
    };
    let target = match declaration.family {
        ApplicationRuntimeFamily::Python => format!(
            "python-{}",
            declaration
                .python_version
                .context("frozen Python serving minor is missing")?
                .version()
        ),
        ApplicationRuntimeFamily::Executable => {
            nrz_runtime_artifact::NATIVE_EXECUTION_TARGET.into()
        }
        ApplicationRuntimeFamily::Node => node_build_target(effective.node_version().context(
            "explicit Node runtime requires the project's selected Node version before building",
        )?)?,
        ApplicationRuntimeFamily::Bun => pinned_bun_build_target()?,
    };
    declaration
        .validate_target(Some(&target))
        .map_err(|error| output::coded_error("APPLICATION_RUNTIME_INVALID", error))?;
    Ok(Some(target))
}

async fn qualify_javascript_binary(
    binary: &str,
    target: Option<&str>,
    environment: &[(String, String)],
) -> anyhow::Result<()> {
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
                let major = actual
                    .strip_prefix('v')
                    .and_then(|version| version.split('.').next());
                target.map_or_else(
                    || matches!(major, Some("22" | "24" | "26")),
                    |target| major == target.strip_prefix("node-"),
                )
            }
            "bun" => Some(actual) == target.and_then(|target| target.strip_prefix("bun-")),
            _ => false,
        };
    if !matches {
        return Err(output::coded_error(
            "APPLICATION_RUNTIME_INVALID",
            format!(
                "local {binary} does not match selected {}; select the correct executable before building",
                target.unwrap_or("supported Node major")
            ),
        ));
    }
    Ok(())
}
