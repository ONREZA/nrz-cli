#[cfg(test)]
#[path = "project_settings_tests.rs"]
mod tests;

use anyhow::Context;
use nrz::config::ProjectBuildSettings;

use crate::api::ApiClient;

pub(crate) enum ProjectSettingsFetch {
    Applied(ProjectBuildSettings),
    TransientFailure { message: String },
}

pub(crate) async fn fetch(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<ProjectBuildSettings> {
    let project = client
        .project(project_id)
        .await
        .context("failed to fetch project settings")?;
    Ok(ProjectBuildSettings {
        framework_preset: project.framework_preset,
        root_directory: project.root_directory,
        git_lfs_enabled: Some(project.git_lfs_enabled),
        package_manager: project.package_manager.to_string(),
        install_command: project.install_command,
        install_command_source: Some(setting_source(project.install_command_source)),
        build_command: project.build_command,
        build_command_source: Some(setting_source(project.build_command_source)),
        output_directory: project.output_directory,
        output_directory_source: Some(setting_source(project.output_directory_source)),
        ignored_build_behavior: Some(ignored_build_behavior(project.ignored_build_behavior)),
        ignored_build_folder: project.ignored_build_folder,
        ignored_build_command: project.ignored_build_command,
    })
}

pub(crate) async fn fetch_for_effective_config(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<ProjectSettingsFetch> {
    match fetch(client, project_id).await {
        Ok(settings) => Ok(ProjectSettingsFetch::Applied(settings)),
        Err(error) if crate::api::classify_api_retry(&error).is_some() => {
            Ok(ProjectSettingsFetch::TransientFailure {
                message: error.to_string(),
            })
        }
        Err(error) => Err(error.context(format!(
            "failed to fetch settings for project '{project_id}'"
        ))),
    }
}

fn setting_source(
    value: nrz_api::ProjectRequestBodyInstallCommandSource,
) -> nrz::config::BuildSettingSource {
    use nrz::config::BuildSettingSource;
    use nrz_api::ProjectRequestBodyInstallCommandSource as ApiSource;
    match value {
        ApiSource::Preset => BuildSettingSource::Preset,
        ApiSource::Detected => BuildSettingSource::Detected,
        ApiSource::User => BuildSettingSource::User,
    }
}

pub(crate) fn ignored_build_behavior(
    value: nrz_api::Project200Response3IgnoredBuildBehavior,
) -> nrz::config::IgnoredBuildBehavior {
    use nrz::config::IgnoredBuildBehavior;
    use nrz_api::Project200Response3IgnoredBuildBehavior as ApiBehavior;
    match value {
        ApiBehavior::Automatic => IgnoredBuildBehavior::Automatic,
        ApiBehavior::OnlyProduction => IgnoredBuildBehavior::OnlyProduction,
        ApiBehavior::OnlyPreview => IgnoredBuildBehavior::OnlyPreview,
        ApiBehavior::OnlyChanges => IgnoredBuildBehavior::OnlyChanges,
        ApiBehavior::ChangesInFolder => IgnoredBuildBehavior::ChangesInFolder,
        ApiBehavior::Never => IgnoredBuildBehavior::Never,
        ApiBehavior::BashScript => IgnoredBuildBehavior::BashScript,
        ApiBehavior::NodeScript => IgnoredBuildBehavior::NodeScript,
        ApiBehavior::Custom => IgnoredBuildBehavior::Custom,
    }
}
