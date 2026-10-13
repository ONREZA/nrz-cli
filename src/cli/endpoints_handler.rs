use super::endpoints::{EndpointCommand, EndpointsArgs, Exposure};
use crate::{api::ApiClient, auth, output};
use anyhow::Context;
use nrz_api::*;

pub async fn run(
    args: EndpointsArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let value = match args.command {
        EndpointCommand::List => serde_json::to_value(client.endpoint_list(&args.scope).await?)?,
        EndpointCommand::Export {
            layer,
            ports,
            expected_generation,
            revoke,
        } => serde_json::to_value(
            client
                .endpoint_export(
                    &args.scope,
                    layer,
                    ExportRequestBody {
                        ports,
                        expected_generation,
                        revoke: Some(revoke),
                    },
                )
                .await?,
        )?,
        EndpointCommand::Import {
            layer,
            ports,
            expected_generation,
            revoke,
            export_id,
            export_generation,
        } => serde_json::to_value(
            client
                .endpoint_import(
                    &args.scope,
                    layer,
                    ImportRequestBody {
                        ports,
                        expected_generation,
                        revoke: Some(revoke),
                        export_id,
                        export_generation,
                    },
                )
                .await?,
        )?,
        EndpointCommand::Network {
            layer,
            file,
            expected_generation,
        } => {
            let network: Endpoint200ResponseEndpointNetwork = serde_json::from_slice(
                &std::fs::read(&file).with_context(|| format!("cannot read {}", file.display()))?,
            )
            .context("invalid complete endpoint network JSON")?;
            let checked: nrz_source_bundle::ComputeNetwork =
                serde_json::from_value(serde_json::to_value(&network)?)?;
            checked.validate().map_err(anyhow::Error::msg)?;
            serde_json::to_value(
                client
                    .endpoint_network(
                        &args.scope,
                        layer,
                        NetworkRequestBody {
                            expected_generation,
                            network,
                            public_route_preview: None,
                        },
                    )
                    .await?,
            )?
        }
        EndpointCommand::Exposure {
            layer,
            exposure,
            expected_generation,
            public_route_preview,
        } => {
            let preview: Option<Vec<NetworkRequestBodyPublicRoutePreview>> = public_route_preview
                .map(|file| -> anyhow::Result<_> {
                    Ok(serde_json::from_slice(
                        &std::fs::read(&file)
                            .with_context(|| format!("cannot read {}", file.display()))?,
                    )?)
                })
                .transpose()?;
            anyhow::ensure!(
                !matches!(exposure, Exposure::Public)
                    || preview.as_ref().is_some_and(|p| !p.is_empty()),
                "PUBLIC exposure requires --public-route-preview with explicit future routes"
            );
            let exposure = match exposure {
                Exposure::Public => Endpoint200ResponseEndpointNetworkExposure::Public,
                Exposure::Private => Endpoint200ResponseEndpointNetworkExposure::Private,
            };
            serde_json::to_value(
                client
                    .endpoint_exposure(
                        &args.scope,
                        layer,
                        ExposureRequestBody {
                            expected_generation,
                            exposure,
                            public_route_preview: preview,
                        },
                    )
                    .await?,
            )?
        }
    };
    if json {
        output::json_output(&value)
    } else {
        eprintln!("{}", serde_json::to_string_pretty(&value)?)
    }
    Ok(())
}
