use anyhow::Context;
use nrz_source_bundle::{ApplicationRuntimeDeclaration, ApplicationRuntimeFamily};

use crate::output;

pub(crate) async fn validate_application_runtime_before_build(
    declaration: Option<&ApplicationRuntimeDeclaration>,
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
        return Ok(None);
    };
    let target = if platform_runner {
        std::env::var("ONREZA_RUNTIME_VERSION").map_err(|_| {
            output::coded_error(
                "APPLICATION_RUNTIME_INVALID",
                "trusted runtime version is missing",
            )
        })?
    } else {
        match declaration.family {
            ApplicationRuntimeFamily::Node => match effective.node_version() {
                Some("NODE_22") => "node-22",
                Some("NODE_24") => "node-24",
                Some("NODE_26") => "node-26",
                _ => return Err(output::coded_error("APPLICATION_RUNTIME_INVALID", "explicit Node runtime requires the project's selected Node version before building")),
            }.to_string(),
            ApplicationRuntimeFamily::Bun => {
                let toolchain: toml::Value = toml::from_str(include_str!("../../mise.toml"))?;
                let version = toolchain.get("tools").and_then(|tools| tools.get("bun")).and_then(toml::Value::as_str)
                    .context("CLI toolchain has no pinned Bun version")?;
                format!("bun-{version}")
            },
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
