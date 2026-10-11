use anyhow::Context;
use nrz_api::Service200ResponseStatus;

use crate::{api::ApiClient, auth, output};

use super::services::{ServicesArgs, ServicesCommand};

pub async fn run(
    args: ServicesArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let ServicesCommand::Delete {
        service_id,
        project_id,
        force,
    } = args.command;
    super::remote::confirm_deletion("service", &service_id, force, json)?;
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let result = client
        .remove_service(&project_id, &service_id)
        .await
        .with_context(|| {
            format!("failed to remove service {service_id} from product {project_id}")
        })?;
    if result.product_id != project_id.parse::<uuid::Uuid>()?
        || result.service_id != service_id.parse::<uuid::Uuid>()?
    {
        anyhow::bail!("platform returned a different Product or Service for the removal request");
    }
    if json {
        output::json_output(&result);
    } else {
        let message = match result.status {
            Service200ResponseStatus::Removing => format!(
                "Service removal accepted for {}. Runtime withdrawal is pending; Product databases and KV data are preserved.",
                output::terminal_line(&service_id),
            ),
            Service200ResponseStatus::Deleted => format!(
                "Deleted service {}. Product databases and KV data are preserved.",
                output::terminal_line(&service_id),
            ),
        };
        output::success(false, message, output::Phase::Projects);
    }
    Ok(())
}
