use std::path::{Path, PathBuf};

use anyhow::Context as _;
use nrz::config::{EffectiveProjectConfig, ProjectConfig};

use crate::project_context::{ProjectContext, SelectedApp};

#[derive(Debug, Clone)]
pub(crate) struct CommandContext {
    pub(crate) root_dir: PathBuf,
    pub(crate) project_dir: PathBuf,
    pub(crate) config: ProjectConfig,
    pub(crate) selected_app: Option<SelectedApp>,
    pub(crate) effective: EffectiveProjectConfig,
    pub(crate) json: bool,
}

impl CommandContext {
    pub(crate) fn resolve(
        dir: impl AsRef<Path>,
        root_config: &ProjectConfig,
        app_override: Option<&str>,
        json: bool,
    ) -> anyhow::Result<Self> {
        let root_dir = dir
            .as_ref()
            .canonicalize()
            .with_context(|| format!("project directory not found: {}", dir.as_ref().display()))?;
        let ProjectContext {
            root_dir,
            project_dir,
            config,
            selected_app,
        } = crate::project_context::resolve(&root_dir, root_config, app_override)?;
        let mut effective =
            EffectiveProjectConfig::from_project_config(project_dir.clone(), config.clone());
        if let Some(app) = selected_app.as_ref() {
            effective.apply_deploy_app_cli_override(
                matches!(app.source, crate::project_context::SelectedAppSource::Cli)
                    .then_some(app.requested.as_str()),
            )?;
        }

        Ok(Self {
            root_dir,
            project_dir,
            config,
            selected_app,
            effective,
            json,
        })
    }

    pub(crate) fn apply_project_id_override(
        &mut self,
        project_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.effective.apply_project_id_override(project_id)
    }

    pub(crate) fn apply_server_settings(
        &mut self,
        settings: Option<&nrz::config::ProjectBuildSettings>,
    ) {
        self.effective.apply_server_settings(settings);
    }
}
