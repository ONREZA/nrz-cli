pub mod config;
mod device_flow;
pub mod workspace;

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod workspace_tests;

use anyhow::Context;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::output;

#[derive(Debug, Deserialize)]
struct UserInfo {
    #[allow(dead_code)]
    id: String,
    email: String,
    name: Option<String>,
    username: Option<String>,
}

#[derive(Serialize)]
struct LoginOutput {
    workspace_slug: String,
    workspace_name: String,
}

#[derive(Serialize)]
struct WhoamiOutput {
    email: String,
    name: Option<String>,
    username: Option<String>,
    workspace_slug: String,
    workspace_name: String,
}

#[derive(Serialize)]
struct StatusOutput {
    status: String,
}

/// Resolve token from explicit arg, workspace config, or legacy credentials.
pub fn resolve_token(token: Option<&str>, ws: Option<&str>) -> anyhow::Result<String> {
    let ctx = workspace::resolve_workspace_context(token, ws)?;
    Ok(ctx.token)
}

pub async fn login(json: bool, token: Option<&str>) -> anyhow::Result<()> {
    // If token already provided via --token/NRZ_TOKEN, save it directly
    if let Some(tok) = token {
        let client = ApiClient::authenticated(tok)?;
        let user: UserInfo = client
            .get("/v1/user")
            .await
            .context("invalid token — failed to fetch user info")?;

        let mut cfg = config::load();
        cfg.add_workspace(
            "personal",
            tok.to_string(),
            user.name.clone().unwrap_or_default(),
        );
        config::save(&cfg)?;

        if json {
            output::json_output(&serde_json::json!({
                "email": user.email,
                "name": user.name,
            }));
        } else {
            output::success(
                false,
                format!("Token saved. Logged in as {}", user.email),
                output::Phase::Auth,
            );
        }
        return Ok(());
    }

    let client = ApiClient::anonymous()?;
    let device = device_flow::request_device_code(&client).await?;

    if json {
        // JSON mode: print device code info, then poll silently
        output::json_output(&serde_json::json!({
            "user_code": device.user_code,
            "verification_uri": device.verification_uri,
            "verification_uri_complete": device.verification_uri_complete,
            "expires_in": device.expires_in,
            "status": "awaiting_authorization",
        }));
    } else {
        eprintln!();
        eprintln!(
            "  Your code: {}",
            console::style(&device.user_code).bold().cyan(),
        );
        eprintln!();

        if open::that(&device.verification_uri_complete).is_err() {
            eprintln!(
                "  Open this URL in your browser:\n  {}",
                console::style(&device.verification_uri_complete).underlined(),
            );
        } else {
            eprintln!("  Browser opened. Waiting for authorization...");
        }
        eprintln!();
    }

    // Poll for token
    let spinner = if !json {
        let s = ProgressBar::new_spinner();
        s.set_style(
            ProgressStyle::with_template("  {spinner} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        s.set_message("Waiting for authorization...");
        s.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(s)
    } else {
        None
    };

    let token_resp = device_flow::poll_for_token(
        &client,
        &device.device_code,
        device.interval,
        device.expires_in,
    )
    .await;

    if let Some(s) = &spinner {
        s.finish_and_clear();
    }

    match token_resp? {
        device_flow::TokenResponse::Success {
            access_token,
            workspace_slug,
            workspace_name,
            ..
        } => {
            let slug = if workspace_slug.is_empty() {
                "personal".to_string()
            } else {
                workspace_slug.clone()
            };

            let mut cfg = config::load();
            cfg.add_workspace(&slug, access_token, workspace_name.clone());
            config::save(&cfg)?;

            if json {
                output::json_output(&LoginOutput {
                    workspace_slug,
                    workspace_name,
                });
            } else {
                output::success(
                    false,
                    format!(
                        "Logged in to workspace: {} ({slug})",
                        console::style(&workspace_name).bold(),
                    ),
                    output::Phase::Auth,
                );
            }
        }
        device_flow::TokenResponse::Error { error } => {
            anyhow::bail!("authorization failed: {error}");
        }
    }

    Ok(())
}

pub async fn whoami(json: bool, token: Option<&str>, ws: Option<&str>) -> anyhow::Result<()> {
    let ctx = workspace::resolve_workspace_context(token, ws)?;

    let client = ApiClient::authenticated(&ctx.token)?;
    let user: UserInfo = client
        .get("/v1/user")
        .await
        .context("failed to fetch user info")?;

    let cfg = config::load();
    let workspace_name = cfg
        .workspaces
        .get(&ctx.workspace_slug)
        .map(|w| w.name.clone())
        .unwrap_or_default();

    if json {
        output::json_output(&WhoamiOutput {
            email: user.email,
            name: user.name,
            username: user.username,
            workspace_slug: ctx.workspace_slug,
            workspace_name,
        });
    } else {
        eprintln!("  {} {}", console::style("Email:").dim(), user.email);
        if let Some(name) = &user.name {
            eprintln!("  {} {}", console::style("Name:").dim(), name);
        }
        if let Some(username) = &user.username {
            eprintln!("  {} {}", console::style("Username:").dim(), username);
        }
        if !workspace_name.is_empty() {
            eprintln!(
                "  {} {} ({})",
                console::style("Workspace:").dim(),
                workspace_name,
                ctx.workspace_slug,
            );
        }
    }

    Ok(())
}

pub async fn logout(json: bool, ws: Option<&str>, all: bool) -> anyhow::Result<()> {
    let mut cfg = config::load();

    if all {
        cfg.workspaces.clear();
        cfg.default_workspace = None;
    } else if let Some(slug) = ws {
        cfg.remove_workspace(slug);
    } else if let Some(slug) = cfg.default_workspace.clone() {
        cfg.remove_workspace(&slug);
    } else if cfg.workspaces.len() == 1 {
        cfg.workspaces.clear();
        cfg.default_workspace = None;
    } else {
        anyhow::bail!("multiple workspaces found. Use --workspace <slug> or --all.");
    }

    config::save(&cfg)?;

    if json {
        output::json_output(&StatusOutput {
            status: "ok".into(),
        });
    } else {
        output::success(false, "Logged out.", output::Phase::Auth);
    }
    Ok(())
}
