use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::api::ApiClient;
use crate::auth;
use crate::execution_context;
use crate::output;
use nrz::config;
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
    dns_mode: String,
    managed_dns_zone: Option<DomainManagedDnsZone>,
    redirect_from_www: bool,
    created_at: String,
    environment: DomainEnvironment,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DomainManagedDnsZone {
    id: String,
    zone_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DomainEnvironment {
    #[serde(rename = "type")]
    env_type: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AttachHostnameBody {
    domain: String,
    project_id: String,
    environment_id: String,
    redirect_from_www: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachHostnameResponse {
    hostname: AddedHostname,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddedHostname {
    id: String,
    domain: String,
    dns_mode: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyWorkspaceZoneResponse {
    delegation: Option<DelegationStatus>,
    requeued: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct DelegationStatus {
    delegated: bool,
    expected: Vec<String>,
    actual: Vec<String>,
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
    let project_id = config::resolve_project_id(args.project_id.as_deref(), config)?;

    match args.command {
        DomainsCommand::List => list(&client, &project_id, json).await,
        DomainsCommand::Add {
            domain,
            environment,
        } => add(&client, &project_id, &domain, environment.as_deref(), json).await,
        DomainsCommand::Remove { domain_id } => {
            remove(&client, &project_id, &domain_id, json).await
        }
        DomainsCommand::Verify { domain_id } => {
            verify(&client, &project_id, &domain_id, json).await
        }
    }
}

async fn list(client: &ApiClient, project_id: &str, json: bool) -> anyhow::Result<()> {
    let resp = fetch_project_domains(client, project_id)
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

async fn fetch_project_domains(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<DomainsListResponse> {
    client
        .get(&format!("/v1/workspace-domains?projectIds={}", project_id))
        .await
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
    environment: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let env_id = execution_context::resolve_for_mutation(
        client,
        project_id,
        std::path::Path::new("."),
        environment,
        None,
    )
    .await?
    .environment_id;

    let body = AttachHostnameBody {
        domain: domain.to_string(),
        project_id: project_id.to_string(),
        environment_id: env_id,
        redirect_from_www: false,
    };

    let resp: AttachHostnameResponse = client
        .post("/v1/workspace-domains/hostnames", &body)
        .await
        .context("failed to add domain")?;

    if json {
        output::json_output(&serde_json::json!({
            "id": resp.hostname.id,
            "domain": resp.hostname.domain,
            "dnsMode": resp.hostname.dns_mode,
        }));
    } else {
        output::success(
            false,
            format!(
                "Added domain {}",
                console::style(&resp.hostname.domain).bold(),
            ),
            output::Phase::Domains,
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
    let domain = find_project_domain(client, project_id, domain_id).await?;
    let delete_path = match (domain.managed_dns_zone.as_ref(), domain.dns_mode.as_str()) {
        (Some(zone), _) => workspace_hostname_delete_url(&zone.id, domain_id),
        (None, _) => project_domain_delete_url(project_id, domain_id),
    };

    client
        .delete_empty(&delete_path)
        .await
        .context("failed to remove domain")?;

    if json {
        output::json_output(&serde_json::json!({
            "id": domain_id,
            "status": "deleted",
        }));
    } else {
        output::success(false, "Domain removed.", output::Phase::Domains);
    }

    Ok(())
}

async fn verify(
    client: &ApiClient,
    project_id: &str,
    domain_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let domain = find_project_domain(client, project_id, domain_id).await?;
    let zone_id = domain
        .managed_dns_zone
        .as_ref()
        .map(|zone| zone.id.clone())
        .ok_or_else(|| anyhow::anyhow!("domain is not attached to a workspace domain"))?;

    let resp: VerifyWorkspaceZoneResponse = client
        .post_empty(&workspace_zone_verify_url(&zone_id))
        .await
        .context("failed to verify domain")?;

    if json {
        output::json_output(&serde_json::json!({
            "id": domain.id,
            "domain": domain.domain,
            "zoneId": zone_id,
            "requeued": resp.requeued,
            "delegation": resp.delegation,
        }));
    } else if resp
        .delegation
        .as_ref()
        .is_some_and(|status| status.delegated)
    {
        output::success(false, "Domain delegation verified.", output::Phase::Domains);
    } else {
        output::warn(
            false,
            "Domain verification queued. Check your DNS records.",
            output::Phase::Domains,
        );
    }

    Ok(())
}

async fn find_project_domain(
    client: &ApiClient,
    project_id: &str,
    domain_id: &str,
) -> anyhow::Result<Domain> {
    fetch_project_domains(client, project_id)
        .await
        .context("failed to fetch domains")?
        .domains
        .into_iter()
        .find(|domain| domain.id == domain_id)
        .ok_or_else(|| anyhow::anyhow!("domain not found in project: {domain_id}"))
}

pub(crate) fn workspace_hostname_delete_url(zone_id: &str, binding_id: &str) -> String {
    format!("/v1/workspace-domains/domains/{zone_id}/hostnames/{binding_id}")
}

pub(crate) fn project_domain_delete_url(project_id: &str, domain_id: &str) -> String {
    format!("/v1/domains/{project_id}/platform-subdomains/{domain_id}")
}

pub(crate) fn workspace_zone_verify_url(zone_id: &str) -> String {
    format!("/v1/workspace-domains/domains/{zone_id}/verify")
}
