use anyhow::Context;
use base64::{Engine, prelude::BASE64_STANDARD};
use nrz_api::*;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    api::{ApiClient, StructuredApiError},
    auth, output,
};

use super::kv_v2::{BranchesCommand, DataCommand, KvV2Args, KvV2Command, StoresCommand};

const OUTCOME_UNKNOWN_MESSAGE: &str = "KV write outcome is unknown. The request may have taken effect; it was not replayed. Inspect the branch before deciding whether to retry.";

pub async fn run(
    args: KvV2Args,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let value = match args.command {
        KvV2Command::Regions { store } => serde_json::to_value(client.kv2_regions(store).await?)?,
        KvV2Command::Stores { command } => stores(&client, command, json).await?,
        KvV2Command::Branches { store, command } => {
            branches(
                &client,
                store.context("--store is required")?,
                command,
                json,
            )
            .await?
        }
        KvV2Command::Data {
            store,
            branch,
            command,
        } => {
            let body = data_request(command)?;
            let mutation = matches!(
                body,
                RequestRequestBody::Object2(_)
                    | RequestRequestBody::Object3(_)
                    | RequestRequestBody::Operation(_)
            );
            // Mutations are sent once: an unknown outcome must never trigger automatic replay.
            let result = client
                .kv2_request(
                    store.context("--store is required")?,
                    branch.context("--branch is required")?,
                    body,
                )
                .await
                .map_err(|error| {
                    let refused = error
                        .downcast_ref::<StructuredApiError>()
                        .is_some_and(|error| {
                            error.status.is_client_error()
                                && error.status != reqwest::StatusCode::REQUEST_TIMEOUT
                                && !error.code.starts_with("HTTP_")
                        });
                    if mutation && !refused {
                        output::coded_error("OUTCOME_UNKNOWN", OUTCOME_UNKNOWN_MESSAGE)
                    } else {
                        error
                    }
                })?;
            if let Request200Response::Object5(error) = &result {
                let message = if error.code == Request200ResponseObject5Code::OutcomeUnknown {
                    OUTCOME_UNKNOWN_MESSAGE
                } else {
                    "KV operation failed; the request was not replayed."
                };
                return Err(output::coded_error(error.code.to_string(), message));
            }
            serde_json::to_value(result)?
        }
    };
    if json {
        output::json_output(&value);
    } else {
        eprintln!(
            "{}",
            output::terminal_text(&serde_json::to_string_pretty(&value)?)
        );
    }
    Ok(())
}

async fn stores(client: &ApiClient, command: StoresCommand, json: bool) -> anyhow::Result<Value> {
    Ok(match command {
        StoresCommand::List(page) => {
            serde_json::to_value(client.kv2_stores(page.after, page.limit).await?)?
        }
        StoresCommand::Create {
            configuration,
            main_branch_id,
        } => {
            let id = configuration.id.unwrap_or_else(Uuid::now_v7);
            let main_branch_id = main_branch_id.unwrap_or_else(Uuid::now_v7);
            let result = client.kv2_create_store(StoreRequestBody {
                id,
                main_branch_id,
                name: configuration.name,
                region: configuration.region,
                batch_window_ms: Some(configuration.batch_window_ms),
            }).await.with_context(|| format!("store creation request id={id} main-branch-id={main_branch_id}; no automatic retry; reuse both IDs with the identical request if retrying"))?;
            anyhow::ensure!(
                result.store.id == id
                    && result.store.main_branch_id == Some(main_branch_id)
                    && result.branch.id == main_branch_id
                    && result.branch.store_id == id,
                "platform returned different Store or branch IDs for store request id={id} main-branch-id={main_branch_id}"
            );
            serde_json::to_value(result)?
        }
        StoresCommand::Info { id } => serde_json::to_value(client.kv2_store(id).await?)?,
        StoresCommand::Delete { id, force } => {
            super::remote::confirm_deletion("store", &id.to_string(), force, json)?;
            serde_json::to_value(client.kv2_delete_store(id).await?)?
        }
    })
}

async fn branches(
    client: &ApiClient,
    store: Uuid,
    command: BranchesCommand,
    json: bool,
) -> anyhow::Result<Value> {
    Ok(match command {
        BranchesCommand::List(page) => {
            serde_json::to_value(client.kv2_branches(store, page.after, page.limit).await?)?
        }
        BranchesCommand::Create(configuration) => {
            let id = configuration.id.unwrap_or_else(Uuid::now_v7);
            let result = client.kv2_create_branch(store, BranchRequestBody {
                id,
                name: configuration.name,
                region: configuration.region,
                batch_window_ms: Some(configuration.batch_window_ms),
            }).await.with_context(|| format!("branch creation request store={store} id={id}; no automatic retry; reuse the ID with the identical request if retrying"))?;
            anyhow::ensure!(
                result.id == id && result.store_id == store,
                "platform returned different Store or branch IDs for branch request store={store} id={id}"
            );
            serde_json::to_value(result)?
        }
        BranchesCommand::Info { id } => serde_json::to_value(client.kv2_branch(store, id).await?)?,
        BranchesCommand::Delete { id, force } => {
            super::remote::confirm_deletion("branch", &id.to_string(), force, json)?;
            serde_json::to_value(client.kv2_delete_branch(store, id).await?)?
        }
        BranchesCommand::Fork { source, name, id } => {
            let id = id.unwrap_or_else(Uuid::now_v7);
            let result = client.kv2_fork(store, source, ForkRequestBody { id, name }).await
                .with_context(|| format!("fork request store={store} source={source} id={id}; no automatic retry; reuse the ID with the identical request if retrying"))?;
            anyhow::ensure!(
                result.id == id && result.store_id == store,
                "platform returned different Store or branch IDs for branch request store={store} id={id}"
            );
            serde_json::to_value(result)?
        }
        BranchesCommand::BatchWindow { id, milliseconds } => {
            serde_json::to_value(client.kv2_batch_window(store, id, milliseconds).await?)?
        }
    })
}

fn binary_input(input: String, base64: bool) -> anyhow::Result<String> {
    const MAX_BYTES: usize = 1_048_576;
    if base64 {
        anyhow::ensure!(
            input.len() <= MAX_BYTES.div_ceil(3) * 4,
            "binary input exceeds 1 MiB"
        );
        let bytes = BASE64_STANDARD
            .decode(&input)
            .context("input must be canonical padded base64")?;
        anyhow::ensure!(bytes.len() <= MAX_BYTES, "binary input exceeds 1 MiB");
        anyhow::ensure!(
            BASE64_STANDARD.encode(&bytes) == input,
            "input must be canonical padded base64"
        );
        Ok(input)
    } else {
        anyhow::ensure!(input.len() <= MAX_BYTES, "binary input exceeds 1 MiB");
        Ok(BASE64_STANDARD.encode(input.as_bytes()))
    }
}

fn durability(durable: bool) -> Option<RequestRequestBodyObject2Durability> {
    Some(if durable {
        RequestRequestBodyObject2Durability::ExplicitDurable
    } else {
        RequestRequestBodyObject2Durability::Buffered
    })
}

fn data_request(command: DataCommand) -> anyhow::Result<RequestRequestBody> {
    Ok(match command {
        DataCommand::Get(key) => RequestRequestBody::Object(RequestRequestBodyObject {
            key: binary_input(key.key, key.base64)?,
            ..Default::default()
        }),
        DataCommand::Put {
            key,
            value,
            durable,
        } => RequestRequestBody::Object2(RequestRequestBodyObject2 {
            key: binary_input(key.key, key.base64)?,
            value: binary_input(value, key.base64)?,
            durability: durability(durable),
            ..Default::default()
        }),
        DataCommand::Delete { key, durable } => {
            RequestRequestBody::Object3(RequestRequestBodyObject3 {
                key: binary_input(key.key, key.base64)?,
                durability: durability(durable),
                ..Default::default()
            })
        }
        DataCommand::Keys {
            prefix,
            after,
            base64,
            limit,
            max_bytes,
        } => RequestRequestBody::Object4(RequestRequestBodyObject4 {
            prefix: binary_input(prefix, base64)?,
            after: after.map(|value| binary_input(value, base64)).transpose()?,
            limit,
            max_bytes,
            ..Default::default()
        }),
        DataCommand::Flush => RequestRequestBody::Operation(Default::default()),
        DataCommand::Stats => RequestRequestBody::Operation2(Default::default()),
    })
}
