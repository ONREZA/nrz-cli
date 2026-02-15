pub mod credentials;
mod device_flow;

#[cfg(test)]
mod credentials_tests;

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

/// Resolve token from explicit arg or saved credentials.
pub fn resolve_token(token: Option<&str>) -> Option<String> {
    token
        .map(String::from)
        .or_else(|| credentials::load().map(|c| c.access_token))
}

pub async fn login(json: bool, token: Option<&str>) -> anyhow::Result<()> {
    // If token already provided via --token/NRZ_TOKEN, save it directly
    if let Some(tok) = token {
        let client = ApiClient::authenticated(tok)?;
        let user: UserInfo = client
            .get("/v1/user")
            .await
            .context("invalid token — failed to fetch user info")?;

        // We need workspace info — fetch from projects or store minimal
        credentials::save(&credentials::Credentials {
            access_token: tok.to_string(),
            workspace_slug: String::new(),
            workspace_name: String::new(),
        })?;

        if json {
            output::json_output(&serde_json::json!({
                "email": user.email,
                "name": user.name,
            }));
        } else {
            output::success(false, format!("Token saved. Logged in as {}", user.email));
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
            credentials::save(&credentials::Credentials {
                access_token,
                workspace_slug: workspace_slug.clone(),
                workspace_name: workspace_name.clone(),
            })?;

            if json {
                output::json_output(&LoginOutput {
                    workspace_slug,
                    workspace_name,
                });
            } else {
                output::success(
                    false,
                    format!(
                        "Logged in to workspace: {} ({workspace_slug})",
                        console::style(&workspace_name).bold(),
                    ),
                );
            }
        }
        device_flow::TokenResponse::Error { error } => {
            anyhow::bail!("authorization failed: {error}");
        }
    }

    Ok(())
}

pub async fn whoami(json: bool, token: Option<&str>) -> anyhow::Result<()> {
    let tok = resolve_token(token)
        .ok_or_else(|| anyhow::anyhow!("not logged in. Run `nrz login` first."))?;

    let client = ApiClient::authenticated(&tok)?;
    let user: UserInfo = client
        .get("/v1/user")
        .await
        .context("failed to fetch user info")?;

    let creds = credentials::load();
    let workspace_slug = creds
        .as_ref()
        .map(|c| c.workspace_slug.clone())
        .unwrap_or_default();
    let workspace_name = creds
        .as_ref()
        .map(|c| c.workspace_name.clone())
        .unwrap_or_default();

    if json {
        output::json_output(&WhoamiOutput {
            email: user.email,
            name: user.name,
            username: user.username,
            workspace_slug,
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
                "  {} {} ({workspace_slug})",
                console::style("Workspace:").dim(),
                workspace_name,
            );
        }
    }

    Ok(())
}

pub async fn logout(json: bool) -> anyhow::Result<()> {
    credentials::remove()?;
    if json {
        output::json_output(&StatusOutput {
            status: "ok".into(),
        });
    } else {
        output::success(false, "Logged out.");
    }
    Ok(())
}
