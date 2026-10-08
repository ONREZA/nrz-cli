use std::path::Path;

use clap::Parser as _;

use crate::cli::DeployArgs;
use crate::context::CommandContext;

pub(super) fn deploy_context(
    project: &Path,
    config: &crate::config::ProjectConfig,
    flags: &[&str],
) -> (CommandContext, DeployArgs) {
    let command = CommandContext::resolve_platform_root(project, config, true).unwrap();
    let args = DeployArgs::try_parse_from(
        ["deploy", project.to_str().unwrap(), "--dry"]
            .into_iter()
            .chain(flags.iter().copied()),
    )
    .unwrap();
    (command, args)
}

pub(super) fn write_project_config(project: &Path, config: &crate::config::ProjectConfig) {
    std::fs::write(
        project.join("onreza.toml"),
        toml::to_string(config).unwrap(),
    )
    .unwrap();
}

pub(super) async fn build_plan(
    args: &DeployArgs,
    command: &CommandContext,
    execution_env: &[(String, String)],
) -> anyhow::Result<super::plan::DeployPlan> {
    super::plan::build(super::plan::DeployPlanRequest {
        args,
        command,
        explicit_compute: None,
        build_logs: None,
        execution_env,
        target_production: None,
        platform_runner: false,
    })
    .await
}
