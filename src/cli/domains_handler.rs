use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::link::project_ref;
use crate::output;

use super::domains::{DomainsArgs, DomainsCommand};

#[derive(Debug, Deserialize, Serialize)]
struct DomainsListResponse {
    domains: Vec<Domain>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Domain {
    id: String,
    domain: String,
    verified: Option<bool>,
    #[serde(rename = "environmentId")]
    environment_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnvironmentsResponse {
    environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
struct Environment {
    id: String,
    #[serde(rename = "type")]
    env_type: String,
}

#[derive(Debug, Serialize)]
struct AddDomainBody {
    domain: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AddDomainResponse {
    id: String,
    domain: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerifyDomainResponse {
    id: String,
    domain: Option<String>,
    verified: Option<bool>,
}

pub async fn run(
    args: DomainsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;
    let project_id = project_ref::resolve_project_id(args.project_id.as_deref())?;

    match args.command {
        DomainsCommand::List => list(&client, &project_id, json).await,
        DomainsCommand::Add {
            domain,
            environment_id,
        } => {
            add(
                &client,
                &project_id,
                &domain,
                environment_id.as_deref(),
                json,
            )
            .await
        }
        DomainsCommand::Remove { domain_id } => {
            remove(&client, &project_id, &domain_id, json).await
        }
        DomainsCommand::Verify { domain_id } => {
            verify(&client, &project_id, &domain_id, json).await
        }
    }
}

async fn list(client: &ApiClient, project_id: &str, json: bool) -> anyhow::Result<()> {
    let resp: DomainsListResponse = client
        .get(&format!("/v1/domains/{}", project_id))
        .await
        .context("failed to fetch domains")?;

    if json {
        output::json_output(&resp);
    } else {
        if resp.domains.is_empty() {
            eprintln!("  No custom domains found.");
            return Ok(());
        }

        eprintln!();
        eprintln!(
            "  {:<40} {:<12} {}",
            console::style("Domain").bold(),
            console::style("Verified").bold(),
            console::style("ID").bold(),
        );
        eprintln!("  {}", "-".repeat(70));

        for d in &resp.domains {
            let verified = if d.verified.unwrap_or(false) {
                console::style("yes").green().to_string()
            } else {
                console::style("no").red().to_string()
            };
            eprintln!("  {:<40} {:<12} {}", d.domain, verified, d.id);
        }
        eprintln!();
    }

    Ok(())
}

async fn add(
    client: &ApiClient,
    project_id: &str,
    domain: &str,
    environment_id: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let env_id = if let Some(id) = environment_id {
        id.to_string()
    } else {
        find_production_environment(client, project_id).await?
    };

    let body = AddDomainBody {
        domain: domain.to_string(),
    };

    let resp: AddDomainResponse = client
        .post(
            &format!("/v1/domains/{}/environments/{}", project_id, env_id),
            &body,
        )
        .await
        .context("failed to add domain")?;

    if json {
        output::json_output(&resp);
    } else {
        output::success(
            false,
            format!("Added domain {}", console::style(&resp.domain).bold(),),
        );
    }

    Ok(())
}

async fn remove(
    client: &ApiClient,
    project_id: &str,
    domain_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    client
        .delete_empty(&format!("/v1/domains/{}/{}", project_id, domain_id))
        .await
        .context("failed to remove domain")?;

    if json {
        output::json_output(&serde_json::json!({
            "id": domain_id,
            "status": "deleted",
        }));
    } else {
        output::success(false, "Domain removed.");
    }

    Ok(())
}

async fn verify(
    client: &ApiClient,
    project_id: &str,
    domain_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let resp: VerifyDomainResponse = client
        .post_empty(&format!("/v1/domains/{}/{}/verify", project_id, domain_id))
        .await
        .context("failed to verify domain")?;

    if json {
        output::json_output(&resp);
    } else {
        let verified = resp.verified.unwrap_or(false);
        if verified {
            output::success(false, "Domain verified.");
        } else {
            output::warn(
                false,
                "Domain verification pending. Check your DNS records.",
            );
        }
    }

    Ok(())
}

async fn find_production_environment(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<String> {
    let resp: EnvironmentsResponse = client
        .get(&format!("/v1/environments/{}", project_id))
        .await
        .context("failed to fetch environments")?;

    resp.environments
        .iter()
        .find(|e| e.env_type.eq_ignore_ascii_case("PRODUCTION"))
        .map(|e| e.id.clone())
        .ok_or_else(|| anyhow::anyhow!("no production environment found"))
}
