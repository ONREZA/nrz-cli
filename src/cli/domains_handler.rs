use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::link::project_ref;
use crate::output;
use nrz::config::ProjectConfig;

use super::domains::{DomainsArgs, DomainsCommand};

#[derive(Debug, Deserialize, Serialize)]
struct DomainsListResponse {
    domains: Vec<Domain>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Domain {
    id: String,
    domain: String,
    environment_id: String,
    dns_status: String,
    tls_status: String,
    dns_validated_at: Option<String>,
    tls_issued_at: Option<String>,
    tls_expires_at: Option<String>,
    dns_error: Option<String>,
    tls_error: Option<String>,
    target_cname: Option<String>,
    redirect_from_www: bool,
    created_at: String,
    environment: DomainEnvironment,
}

#[derive(Debug, Deserialize, Serialize)]
struct DomainEnvironment {
    #[serde(rename = "type")]
    env_type: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct EnvironmentsResponse {
    environments: Vec<Environment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Environment {
    id: String,
    #[serde(rename = "type")]
    env_type: String,
}

#[derive(Debug, Serialize)]
struct AddDomainBody {
    domain: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddDomainResponse {
    domain: AddedDomain,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddedDomain {
    id: String,
    domain: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyDomainResponse {
    domain: String,
    verified: bool,
    dns_status: String,
    #[serde(default)]
    is_apex: Option<bool>,
}

pub async fn run(
    args: DomainsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;

    let client = ApiClient::authenticated(&tok)?;
    let project_id = project_ref::resolve_project_id(args.project_id.as_deref(), config)?;

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
    } else if resp.domains.is_empty() {
        eprintln!("  No custom domains found.");
        return Ok(());
    } else {
        eprintln!();
        eprintln!(
            "  {:<40} {:<12} {:<10} {}",
            console::style("Domain").bold(),
            console::style("DNS").bold(),
            console::style("TLS").bold(),
            console::style("Environment").bold(),
        );
        eprintln!("  {}", "-".repeat(75));

        for d in &resp.domains {
            let dns = format_status(&d.dns_status);
            let tls = format_status(&d.tls_status);
            let env_name = &d.environment.name;
            eprintln!("  {:<40} {:<12} {:<10} {}", d.domain, dns, tls, env_name);
        }
        eprintln!();
    }

    Ok(())
}

fn format_status(status: &str) -> String {
    match status.to_uppercase().as_str() {
        "VALIDATED" | "ISSUED" => console::style(status.to_lowercase()).green().to_string(),
        "PENDING" => console::style(status.to_lowercase()).yellow().to_string(),
        "FAILED" => console::style(status.to_lowercase()).red().to_string(),
        _ => status.to_lowercase(),
    }
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
        output::json_output(&serde_json::json!({
            "id": resp.domain.id,
            "domain": resp.domain.domain,
            "message": resp.message,
        }));
    } else {
        output::success(
            false,
            format!(
                "Added domain {}",
                console::style(&resp.domain.domain).bold(),
            ),
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
    } else if resp.verified {
        output::success(false, "Domain verified.");
    } else {
        output::warn(
            false,
            "Domain verification pending. Check your DNS records.",
        );
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
