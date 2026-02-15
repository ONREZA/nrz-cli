use serde::Serialize;

use crate::auth::config;
use crate::output;

use super::workspace::{WorkspaceArgs, WorkspaceCommand};

#[derive(Serialize)]
struct WorkspaceListEntry {
    slug: String,
    name: String,
    default: bool,
}

pub async fn run(args: WorkspaceArgs, json: bool) -> anyhow::Result<()> {
    match args.command {
        WorkspaceCommand::List => list(json),
        WorkspaceCommand::Switch { slug } => switch(&slug, json),
    }
}

fn list(json: bool) -> anyhow::Result<()> {
    let cfg = config::load();

    if json {
        let entries: Vec<WorkspaceListEntry> = cfg
            .workspaces
            .iter()
            .map(|(slug, info)| WorkspaceListEntry {
                slug: slug.clone(),
                name: info.name.clone(),
                default: cfg.default_workspace.as_deref() == Some(slug.as_str()),
            })
            .collect();
        output::json_output(&entries);
    } else {
        if cfg.workspaces.is_empty() {
            eprintln!("  No workspaces found. Run `nrz login` first.");
            return Ok(());
        }

        eprintln!();
        for (slug, info) in &cfg.workspaces {
            let is_default = cfg.default_workspace.as_deref() == Some(slug.as_str());
            let marker = if is_default { " (default)" } else { "" };
            eprintln!(
                "  {} {}{}",
                console::style(slug).bold(),
                info.name,
                console::style(marker).dim(),
            );
        }
        eprintln!();
    }

    Ok(())
}

fn switch(slug: &str, json: bool) -> anyhow::Result<()> {
    let mut cfg = config::load();

    if !cfg.workspaces.contains_key(slug) {
        anyhow::bail!(
            "workspace '{slug}' not found. Run `nrz workspace list` to see available workspaces."
        );
    }

    cfg.default_workspace = Some(slug.to_string());
    config::save(&cfg)?;

    if json {
        output::json_output(&serde_json::json!({
            "default_workspace": slug,
            "status": "ok",
        }));
    } else {
        output::success(
            false,
            format!("Switched to workspace {}", console::style(slug).bold(),),
        );
    }

    Ok(())
}
