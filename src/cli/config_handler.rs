use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use super::config::{ConfigArgs, ConfigCommand, ConfigExplainArgs};
use crate::output;
use crate::project_context::SelectedAppSource;
use crate::project_settings::ProjectSettingsFetch;
use nrz::config::{EffectiveConfigExplanation, EffectiveProjectConfig, ProjectConfig};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigExplainOutput {
    root_dir: String,
    project_dir: String,
    selected_app: Option<SelectedAppOutput>,
    server_settings: ServerSettingsOutput,
    effective: EffectiveConfigExplanation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedAppOutput {
    requested: String,
    path: String,
    source: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServerSettingsOutput {
    applied: bool,
    project_id: Option<String>,
    source: String,
}

pub async fn run(
    args: ConfigArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    match args.command {
        ConfigCommand::Explain(args) => explain(args, json, token, workspace, config).await,
    }
}

async fn explain(
    args: ConfigExplainArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let root_dir = Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;
    let context = crate::project_context::resolve(&root_dir, config, args.app.as_deref())?;
    let selected_app = context.selected_app.clone();
    let mut effective =
        EffectiveProjectConfig::from_project_config(context.project_dir.clone(), context.config);
    effective.apply_project_id_override(args.project_id.as_deref())?;
    if selected_app
        .as_ref()
        .is_some_and(|app| matches!(app.source, SelectedAppSource::Cli))
    {
        effective.apply_deploy_app_cli_override(
            selected_app.as_ref().map(|app| app.requested.as_str()),
        )?;
    }
    let server_settings =
        apply_server_settings_if_needed(&mut effective, args.local, token, workspace).await?;
    let explanation = effective.explain();
    let output = ConfigExplainOutput {
        root_dir: context.root_dir.display().to_string(),
        project_dir: context.project_dir.display().to_string(),
        selected_app: selected_app.map(|app| SelectedAppOutput {
            requested: app.requested,
            path: app.path,
            source: app.source.as_str().to_string(),
        }),
        server_settings,
        effective: explanation,
    };

    if json {
        output::json_output(&output);
    } else {
        print_human_explanation(&output);
    }

    Ok(())
}

async fn apply_server_settings_if_needed(
    effective: &mut EffectiveProjectConfig,
    local_only: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<ServerSettingsOutput> {
    let project_id = effective.project_id().map(str::to_string);

    if local_only {
        return Ok(ServerSettingsOutput {
            applied: false,
            project_id,
            source: "local-only".to_string(),
        });
    }

    let Some(project_id) = project_id else {
        return Ok(ServerSettingsOutput {
            applied: false,
            project_id: None,
            source: "no-project-id".to_string(),
        });
    };

    let tok = crate::auth::resolve_token(token, workspace)?;
    let client = crate::api::ApiClient::authenticated(&tok)?;
    match crate::project_settings::fetch_for_effective_config(&client, &project_id).await? {
        ProjectSettingsFetch::Applied(settings) => {
            effective.apply_server_settings(Some(&settings));

            Ok(ServerSettingsOutput {
                applied: true,
                project_id: Some(project_id),
                source: "server".to_string(),
            })
        }
        ProjectSettingsFetch::TransientFailure { .. } => Ok(ServerSettingsOutput {
            applied: false,
            project_id: Some(project_id),
            source: "server-unavailable".to_string(),
        }),
    }
}

fn print_human_explanation(output: &ConfigExplainOutput) {
    eprintln!("Effective config");
    eprintln!("  Root dir: {}", output.root_dir);
    eprintln!("  Project dir: {}", output.project_dir);
    if let Some(app) = &output.selected_app {
        eprintln!("  App: {} ({}, {})", app.requested, app.path, app.source);
    }
    eprintln!(
        "  Server settings: {} ({})",
        if output.server_settings.applied {
            "applied"
        } else {
            "not applied"
        },
        output.server_settings.source
    );
    print_value("Project ID", &output.effective.project_id);
    print_value("Framework", &output.effective.framework);
    print_value("Install command", &output.effective.install_command);
    print_value("Build command", &output.effective.build_command);
    print_value("Output directory", &output.effective.output_directory);
    eprintln!(
        "  Output dirs: {} ({})",
        output.effective.output_dirs.values.join(", "),
        output.effective.output_dirs.source
    );
    print_value("Deploy compute", &output.effective.deploy_compute);
    print_value("Deploy entry", &output.effective.deploy_entry);
    print_value("Deploy app", &output.effective.deploy_app);
}

fn print_value(label: &str, value: &nrz::config::EffectiveConfigValue) {
    eprintln!(
        "  {label}: {} ({})",
        value.value.as_deref().unwrap_or("-"),
        value.source
    );
}
