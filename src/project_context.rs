use std::path::{Path, PathBuf};

use anyhow::Context;
use nrz::config::ProjectConfig;

use crate::output;

#[derive(Debug, Clone)]
pub(crate) struct ProjectContext {
    pub root_dir: PathBuf,
    pub project_dir: PathBuf,
    pub config: ProjectConfig,
    pub selected_app: Option<SelectedApp>,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectedApp {
    pub requested: String,
    pub path: String,
    pub source: SelectedAppSource,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SelectedAppSource {
    Cli,
    Config,
}

impl SelectedAppSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Config => "onreza.toml",
        }
    }
}

pub(crate) fn resolve(
    root_dir: &Path,
    root_config: &ProjectConfig,
    app_override: Option<&str>,
) -> anyhow::Result<ProjectContext> {
    let root_dir = root_dir
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", root_dir.display()))?;
    let selected_app = app_override
        .map(|app| (app, SelectedAppSource::Cli))
        .or_else(|| {
            root_config
                .deploy_app()
                .map(|app| (app, SelectedAppSource::Config))
        });

    let Some((app_name, app_source)) = selected_app else {
        return Ok(ProjectContext {
            root_dir: root_dir.clone(),
            project_dir: root_dir,
            config: root_config.clone(),
            selected_app: None,
        });
    };

    let app_path = resolve_monorepo_app_path(&root_dir, app_name)?;
    let project_dir = root_dir.join(&app_path).canonicalize().with_context(|| {
        format!(
            "failed to resolve app path {} in {}",
            app_path,
            root_dir.display()
        )
    })?;
    if !project_dir.starts_with(&root_dir) {
        return Err(output::coded_error(
            "MONOREPO_APP_NOT_FOUND",
            format!(
                "resolved app directory escapes the monorepo root: {}",
                project_dir.display()
            ),
        ));
    }
    let app_config = nrz::config::load(&project_dir)?;
    let config = root_config.merge_child_for_selected_app(app_config, app_name);

    Ok(ProjectContext {
        root_dir,
        project_dir,
        config,
        selected_app: Some(SelectedApp {
            requested: app_name.to_string(),
            path: app_path,
            source: app_source,
        }),
    })
}

fn resolve_monorepo_app_path(root_dir: &Path, app_name: &str) -> anyhow::Result<String> {
    let mono_fs = crate::detect::fs::LocalFs::new(root_dir);
    let mono_pkg = crate::detect::package_json::PackageJson::load_from_fs(&mono_fs);
    let mono_pm =
        crate::detect::package_manager::detect_package_manager(&mono_fs, mono_pkg.as_ref());
    let Some(info) =
        crate::detect::monorepo::detect_monorepo(&mono_fs, mono_pkg.as_ref(), mono_pm.as_ref())
    else {
        return Err(output::coded_error(
            "MONOREPO_APP_NOT_FOUND",
            format!(
                "--app or [deploy] app was specified but no monorepo detected in {}",
                root_dir.display()
            ),
        ));
    };

    let app_path = match crate::detect::monorepo::resolve_app(&info, app_name) {
        Ok(Some(path)) => path,
        Err(paths) => {
            return Err(output::coded_error(
                "MONOREPO_APP_AMBIGUOUS",
                format!(
                    "app \"{app_name}\" matches multiple monorepo packages: {}. \
                     Pass an exact package name or relative path.",
                    paths.join(", ")
                ),
            ));
        }
        Ok(None) => {
            let available: Vec<String> = info
                .packages
                .iter()
                .map(|p| p.name.as_deref().unwrap_or(&p.path).to_string())
                .collect();
            return Err(output::coded_error(
                "MONOREPO_APP_NOT_FOUND",
                format!(
                    "app \"{app_name}\" not found in monorepo workspaces.\n\
                     Available packages: {}",
                    if available.is_empty() {
                        "(none resolved)".to_string()
                    } else {
                        available.join(", ")
                    }
                ),
            ));
        }
    };

    let resolved = root_dir.join(&app_path);
    if !resolved.is_dir() {
        return Err(output::coded_error(
            "MONOREPO_APP_NOT_FOUND",
            format!(
                "resolved app directory does not exist: {}",
                resolved.display()
            ),
        ));
    }

    Ok(app_path)
}
