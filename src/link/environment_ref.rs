use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentType {
    Production,
    Preview,
    Development,
}

impl std::fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Production => write!(f, "production"),
            Self::Preview => write!(f, "preview"),
            Self::Development => write!(f, "development"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentRef {
    pub environment_id: String,
    pub environment_type: EnvironmentType,
}

#[derive(Debug, Deserialize)]
pub struct EnvironmentsResponse {
    pub environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    #[serde(rename = "type")]
    pub env_type: String,
}

pub fn load(project_dir: &Path) -> anyhow::Result<Option<EnvironmentRef>> {
    let path = project_dir.join(".onreza/environment.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let eref = serde_json::from_str(&content)
                .with_context(|| format!("corrupt environment link file: {}", path.display()))?;
            Ok(Some(eref))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

pub fn save(project_dir: &Path, env_ref: &EnvironmentRef) -> anyhow::Result<()> {
    let dir = project_dir.join(".onreza");
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let path = dir.join("environment.json");
    let json = serde_json::to_string_pretty(env_ref)?;
    std::fs::write(&path, &json).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

/// Resolve environment ID and type from explicit flag, saved ref, or interactive selection.
pub async fn resolve_environment_id(
    explicit: Option<&str>,
    project_id: &str,
    client: &ApiClient,
    json: bool,
) -> anyhow::Result<(String, EnvironmentType)> {
    // 1. Explicit --env flag (can be type like "production" or direct ID)
    if let Some(env) = explicit {
        return resolve_env_value(env, project_id, client).await;
    }

    // 2. Saved .onreza/environment.json
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    if let Some(eref) = load(&cwd)? {
        return Ok((eref.environment_id, eref.environment_type));
    }

    // 3. Interactive selection (only in terminal mode)
    if json {
        bail!("--env is required in non-interactive mode (--json)");
    }

    if !std::io::stdin().is_terminal() {
        bail!("--env is required in non-interactive mode");
    }

    let selected = select_environment_interactive(project_id, client).await?;

    // Save for next time
    save(&cwd, &selected)?;

    let env_type = selected.environment_type;
    Ok((selected.environment_id, env_type))
}

/// Resolve an --env value: if it looks like a type name (production/preview/development),
/// find the matching environment; otherwise treat as a direct ID.
async fn resolve_env_value(
    env: &str,
    project_id: &str,
    client: &ApiClient,
) -> anyhow::Result<(String, EnvironmentType)> {
    let lower = env.to_lowercase();
    let env_type = match lower.as_str() {
        "production" => Some(EnvironmentType::Production),
        "preview" => Some(EnvironmentType::Preview),
        "development" => Some(EnvironmentType::Development),
        _ => None,
    };

    if let Some(et) = env_type {
        let resp: EnvironmentsResponse = client
            .get(&format!("/v1/environments/{project_id}"))
            .await
            .context("failed to fetch environments")?;

        let id = resp
            .environments
            .iter()
            .find(|e| e.env_type.eq_ignore_ascii_case(&lower))
            .map(|e| e.id.clone())
            .ok_or_else(|| anyhow::anyhow!("no {lower} environment found for this project"))?;

        Ok((id, et))
    } else {
        // Direct environment ID — fetch type from API
        let resp: EnvironmentsResponse = client
            .get(&format!("/v1/environments/{project_id}"))
            .await
            .context("failed to fetch environments")?;

        let found = resp
            .environments
            .iter()
            .find(|e| e.id == env)
            .ok_or_else(|| anyhow::anyhow!("environment '{env}' not found for this project"))?;

        let et = match found.env_type.to_lowercase().as_str() {
            "production" => EnvironmentType::Production,
            "preview" => EnvironmentType::Preview,
            "development" => EnvironmentType::Development,
            other => bail!("unknown environment type '{other}' from API"),
        };

        Ok((env.to_string(), et))
    }
}

async fn select_environment_interactive(
    project_id: &str,
    client: &ApiClient,
) -> anyhow::Result<EnvironmentRef> {
    let resp: EnvironmentsResponse = client
        .get(&format!("/v1/environments/{project_id}"))
        .await
        .context("failed to fetch environments")?;

    if resp.environments.is_empty() {
        bail!("no environments found for this project");
    }

    eprintln!();
    for (i, env) in resp.environments.iter().enumerate() {
        eprintln!(
            "  {} {}",
            console::style(format!("{}.", i + 1)).dim(),
            env.env_type,
        );
    }
    eprintln!();

    let choice = prompt_choice(resp.environments.len())?;
    let env = &resp.environments[choice - 1];

    let env_type = match env.env_type.to_lowercase().as_str() {
        "production" => EnvironmentType::Production,
        "preview" => EnvironmentType::Preview,
        "development" => EnvironmentType::Development,
        other => bail!("unknown environment type '{other}' from API"),
    };

    Ok(EnvironmentRef {
        environment_id: env.id.clone(),
        environment_type: env_type,
    })
}

fn prompt_choice(max: usize) -> anyhow::Result<usize> {
    loop {
        eprint!(
            "  {} ",
            console::style(format!("Select environment (1-{max}):")).bold(),
        );
        std::io::stderr().flush()?;

        let mut line = String::new();
        let bytes_read = std::io::stdin().lock().read_line(&mut line)?;
        if bytes_read == 0 {
            bail!("unexpected end of input while selecting environment");
        }
        let trimmed = line.trim();

        if let Ok(n) = trimmed.parse::<usize>()
            && n >= 1
            && n <= max
        {
            return Ok(n);
        }
        eprintln!("  Invalid choice. Enter a number between 1 and {max}.");
    }
}
