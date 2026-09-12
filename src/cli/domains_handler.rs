use anyhow::{Context, bail};
use nrz_api::{DomainResponse as DomainsListResponse, DomainResponseItem as Domain};

use crate::api::ApiClient;
use crate::auth;
use crate::execution_context;
use crate::output;
use nrz::config;
use nrz::config::ProjectConfig;

use super::domains::{DomainsArgs, DomainsCommand};

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
            let dns = format_status(&output::terminal_line(&d.dns_status.to_string()));
            let tls = format_status(&output::terminal_line(&d.tls_status.to_string()));
            let domain = output::terminal_line(&d.domain);
            let env_name = output::terminal_line(&d.environment.name);
            eprintln!("  {domain:<40} {dns:<12} {tls:<10} {env_name}");
        }
        eprintln!();
    }

    Ok(())
}

async fn fetch_project_domains(
    client: &ApiClient,
    project_id: &str,
) -> anyhow::Result<DomainsListResponse> {
    client.project_domains(project_id).await
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

    let zones = client
        .domain_zones()
        .await
        .context("failed to list connected domain zones")?;
    let (zone_id, label) = hostname_zone(domain, &zones.domains)?;
    let body = nrz_api::HostnameRequestBody {
        name: label,
        project_id: project_id.parse().context("invalid project ID")?,
        environment_id: env_id.parse().context("invalid environment ID")?,
        redirect_from_www: Some(false),
        replace_record_ids: None,
    };
    let resp = client
        .attach_hostname(zone_id, body)
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
    if let Some(zone) = domain.managed_dns_zone.as_ref() {
        client
            .detach_hostname(zone.id, domain.id)
            .await
            .context("failed to remove domain")?;
    } else if domain.dns_mode == nrz_api::DomainResponseItemDnsMode::PlatformSubdomain {
        client
            .delete_platform_subdomain(project_id, domain.id)
            .await
            .context("failed to remove platform subdomain")?;
    } else {
        bail!("domain is not attached to a workspace zone; reconnect it before removal");
    }

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
        .map(|zone| zone.id)
        .ok_or_else(|| anyhow::anyhow!("domain is not attached to a workspace domain"))?;

    let resp = client
        .verify_domain_zone(zone_id)
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
    let binding_id: uuid::Uuid = domain_id.parse().context("invalid domain binding ID")?;
    fetch_project_domains(client, project_id)
        .await
        .context("failed to fetch domains")?
        .domains
        .into_iter()
        .find(|domain| domain.id == binding_id)
        .ok_or_else(|| anyhow::anyhow!("domain not found in project: {domain_id}"))
}

/// Resolve an FQDN to the most specific connected zone and a relative DNS label.
pub(crate) fn hostname_zone(
    domain: &str,
    zones: &[nrz_api::DomainResponse2Item],
) -> anyhow::Result<(uuid::Uuid, String)> {
    let domain = domain.strip_suffix('.').unwrap_or(domain);
    let url::Host::Domain(domain) = url::Host::parse(domain).context("invalid domain name")? else {
        bail!("a domain name is required, not an IP address");
    };
    let zone = zones
        .iter()
        .filter(|zone| {
            domain == zone.zone_name
                || domain
                    .strip_suffix(&zone.zone_name)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
        .max_by_key(|zone| zone.zone_name.len())
        .context("no connected workspace zone matches this hostname; connect the domain first")?;
    let label = if domain == zone.zone_name {
        "@".to_string()
    } else {
        domain[..domain.len() - zone.zone_name.len() - 1].to_string()
    };
    Ok((zone.id, label))
}
