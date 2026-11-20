use anyhow::Context;
use nrz_api::{Service200Response3Status, ServiceRequestBody};

use crate::{api::ApiClient, auth, output};

use super::services::{ServicesArgs, ServicesCommand};

pub async fn run(
    args: ServicesArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    match args.command {
        ServicesCommand::Delete {
            service_id,
            project_id,
            force,
        } => remove_service(service_id, project_id, force, json, token, workspace).await,
        command => {
            let token = auth::resolve_token(token, workspace)?;
            let client = ApiClient::authenticated(&token)?;
            let value = match command {
                ServicesCommand::List { project_id } => {
                    serde_json::to_value(client.list_services(project_id).await?)?
                }
                ServicesCommand::CreateSource {
                    project_id,
                    git_url,
                    git_connection_id,
                } => {
                    let source = if let Some(url) = git_url {
                        client
                            .create_git_source(project_id, url, git_connection_id)
                            .await?
                    } else {
                        client.create_cli_source(project_id).await?
                    };
                    serde_json::to_value(source)?
                }
                ServicesCommand::Create(configuration) => {
                    let settings = super::projects_handler::user_build_settings(
                        configuration.build.install_command,
                        configuration.build.build_command,
                        configuration.build.output_directory,
                    );
                    let base = configuration.configuration;
                    let body = ServiceRequestBody {
                        name: base.name,
                        source_id: base.source_id,
                        source_ref: base.source_ref,
                        root_directory: base.root_directory,
                        framework_preset: configuration.framework,
                        package_manager: configuration
                            .build
                            .package_manager
                            .map(|manager| {
                                serde_json::from_value(serde_json::json!(manager.to_uppercase()))
                            })
                            .transpose()?,
                        install_command: settings.install_command,
                        install_command_source: settings.install_command_source,
                        build_command: settings.build_command,
                        build_command_source: settings.build_command_source,
                        output_directory: settings.output_directory,
                        output_directory_source: settings.output_directory_source,
                        ..Default::default()
                    };
                    configure(&client, base.project_id, None, body).await?
                }
                ServicesCommand::Update {
                    service_id,
                    configuration,
                } => {
                    let body = ServiceRequestBody {
                        name: configuration.name,
                        source_id: configuration.source_id,
                        source_ref: configuration.source_ref,
                        root_directory: configuration.root_directory,
                        ..Default::default()
                    };
                    configure(&client, configuration.project_id, Some(service_id), body).await?
                }
                ServicesCommand::Delete { .. } => unreachable!(),
            };
            if json {
                output::json_output(&value);
            } else {
                eprintln!("{}", serde_json::to_string_pretty(&value)?);
            }
            Ok(())
        }
    }
}

async fn configure(
    client: &ApiClient,
    project_id: uuid::Uuid,
    service_id: Option<uuid::Uuid>,
    body: ServiceRequestBody,
) -> anyhow::Result<serde_json::Value> {
    let result = client
        .configure_service(project_id, service_id, body.clone())
        .await?;
    anyhow::ensure!(
        service_id.is_none_or(|id| id == result.service_id)
            && result.source_id == body.source_id
            && body
                .source_ref
                .is_none_or(|value| result.source_ref == value)
            && body
                .root_directory
                .is_none_or(|value| result.root_directory == value),
        "platform returned a different Service or Source configuration"
    );
    Ok(serde_json::to_value(result)?)
}

async fn remove_service(
    service_id: String,
    project_id: String,
    force: bool,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
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
            Service200Response3Status::Removing => format!(
                "Service removal accepted for {}. Runtime withdrawal is pending; Product databases and KV data are preserved.",
                output::terminal_line(&service_id),
            ),
            Service200Response3Status::Deleted => format!(
                "Deleted service {}. Product databases and KV data are preserved.",
                output::terminal_line(&service_id),
            ),
        };
        output::success(false, message, output::Phase::Projects);
    }
    Ok(())
}
