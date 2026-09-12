use super::*;

impl TryFrom<nrz_api::RunnerContext200Response> for RunnerContextResponse {
    type Error = anyhow::Error;
    fn try_from(value: nrz_api::RunnerContext200Response) -> anyhow::Result<Self> {
        Ok(Self {
            context: value.context.into(),
            deployment: RunnerDeploymentContext {
                id: value.deployment.id.to_string(),
                attempt: value
                    .deployment
                    .attempt
                    .try_into()
                    .context("invalid deployment attempt")?,
                status: value.deployment.status.to_string(),
                url: value.deployment.url,
            },
            settings: runner_settings(value.settings)?,
        })
    }
}

impl TryFrom<nrz_api::Admit200Response> for AdmissionResponse {
    type Error = anyhow::Error;
    fn try_from(value: nrz_api::Admit200Response) -> anyhow::Result<Self> {
        Ok(Self {
            context: value.context.into(),
            deployment: AdmissionDeployment {
                id: value.deployment.id.to_string(),
                attempt: value
                    .deployment
                    .attempt
                    .try_into()
                    .context("invalid deployment attempt")?,
                status: value.deployment.status,
                url: value.deployment.url,
            },
        })
    }
}

fn runner_settings(
    value: nrz_api::RunnerContext200ResponseSettings,
) -> anyhow::Result<ProjectBuildSettings> {
    Ok(ProjectBuildSettings {
        framework_preset: value.framework_preset,
        root_directory: value.root_directory,
        git_lfs_enabled: Some(value.git_lfs_enabled),
        package_manager: value.package_manager.to_string(),
        install_command: value.install_command,
        install_command_source: Some(setting_source(&value.install_command_source)?),
        build_command: value.build_command,
        build_command_source: Some(setting_source(&value.build_command_source)?),
        output_directory: value.output_directory,
        output_directory_source: Some(setting_source(&value.output_directory_source)?),
        ignored_build_behavior: Some(crate::project_settings::ignored_build_behavior(
            value.ignored_build_behavior,
        )),
        ignored_build_folder: value.ignored_build_folder,
        ignored_build_command: value.ignored_build_command,
    })
}

fn setting_source(value: &str) -> anyhow::Result<nrz::config::BuildSettingSource> {
    use nrz::config::BuildSettingSource;
    match value {
        "PRESET" => Ok(BuildSettingSource::Preset),
        "DETECTED" => Ok(BuildSettingSource::Detected),
        "USER" => Ok(BuildSettingSource::User),
        _ => bail!("unsupported build setting source in runner context"),
    }
}

pub(super) async fn load_runner_context(
    client: &ApiClient,
    deployment_id: Uuid,
) -> anyhow::Result<RunnerContextResponse> {
    let value = client.runner_context(&deployment_id.to_string()).await?;
    anyhow::ensure!(
        value.deployment.id == deployment_id,
        "runner context belongs to another deployment"
    );
    require_runner_context_protocol(&value.protocol_version)?;
    value.try_into()
}

pub(super) async fn admit(
    client: &ApiClient,
    project_id: &str,
    context: &crate::execution_context::ExecutionContext,
    branch: String,
    commit_sha: String,
) -> anyhow::Result<AdmissionResponse> {
    let value = client
        .admit_deployment(
            project_id,
            nrz_api::AdmitRequestBody {
                protocol_version: crate::execution_context::EXECUTION_CONTEXT_PROTOCOL.to_string(),
                environment_id: context
                    .environment_id
                    .parse()
                    .context("invalid environment ID")?,
                branch,
                commit_sha,
                selection_source: crate::execution_context::wire::selection_source(
                    &context.selection_source,
                )?,
            },
        )
        .await?;
    anyhow::ensure!(
        value.context.project_id == project_id.parse::<Uuid>()?
            && value.context.environment_id == context.environment_id.parse::<Uuid>()?
            && value.context.workspace_id == context.workspace_id.parse::<Uuid>()?,
        "admission response belongs to another workspace, project or environment"
    );
    anyhow::ensure!(
        value.protocol_version == crate::execution_context::EXECUTION_CONTEXT_PROTOCOL,
        "unsupported execution context protocol"
    );
    value.try_into()
}
