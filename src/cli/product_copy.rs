use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Subcommand};
use nrz_api::{
    ExpectedGenerationRequestBody, PlanRequestBody, PreviewPublicationRequestBody,
    PublishServiceRequestBody, UpdateRequestBody,
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::remote::confirm_deletion;
use crate::{api::ApiClient, auth, output};

#[derive(Args)]
pub struct ForkArgs {
    /// Product owning the copy
    #[arg(long)]
    pub product_id: Uuid,
    #[command(subcommand)]
    pub action: ForkAction,
}

#[derive(Subcommand)]
pub enum ForkAction {
    /// Preview topology, data access, blockers and expected cost
    Plan {
        #[arg(long)]
        spec: PathBuf,
    },
    /// Read measured costs for the copy and its resources
    Usage { fork_id: Uuid },
    /// Manage the Git pull request policy for an exact Product Source
    Policy {
        source_id: Uuid,
        #[command(subcommand)]
        action: ForkPolicyAction,
    },
    /// Create a copy from an explicit running parent
    Create {
        /// JSON file with the parent, expiry, idempotency key and any database or secret choices
        #[arg(long)]
        spec: PathBuf,
    },
    /// Show the copy, its source versions, resources and current cleanup receipts
    Show { fork_id: Uuid },
    /// Update selected Services while keeping the copy's database and test data
    Update {
        fork_id: Uuid,
        /// JSON file containing idempotencyKey and services selected by deployment or Git revision
        #[arg(long)]
        spec: PathBuf,
    },
    /// Keep a copy available for QA until the specified instant
    Retain {
        fork_id: Uuid,
        #[arg(long)]
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    /// Request cleanup and return its current state
    Delete {
        fork_id: Uuid,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum ForkPolicyAction {
    /// Read the policy and its current runs
    Show,
    /// Replace the policy at the expected generation using a JSON document
    Set {
        #[arg(long)]
        spec: PathBuf,
    },
    /// Disable automatic copies at this observed generation
    Disable {
        #[arg(long)]
        expected_generation: u64,
    },
}

#[derive(Args)]
pub struct PublishArgs {
    /// Destination ProductEnvironment
    #[arg(long)]
    target_environment: Uuid,
    /// Environment where the selected version was tested
    #[arg(long)]
    source_environment: Uuid,
    /// Immutable composition tested in the source environment
    #[arg(long)]
    source_composition: Uuid,
    /// Select a Service from that composition; repeat for multiple Services
    #[arg(long = "service", required = true)]
    services: Vec<Uuid>,
    /// Read the accepted, observed and pending target versions
    #[arg(long,conflicts_with_all=["idempotency_key","confirmations"])]
    preview: bool,
    /// Keep this key to resume the same publication after a lost response
    #[arg(long, required_unless_present = "preview")]
    idempotency_key: Option<Uuid>,
    /// JSON array of exact serviceId, candidatePreparationId, replacedRuntimeReleaseId confirmations
    #[arg(long)]
    confirmations: Option<PathBuf>,
}

fn read_spec<T: DeserializeOwned>(path: &PathBuf) -> anyhow::Result<T> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("invalid request in {}", path.display()))
}

fn display(value: &impl serde::Serialize, json: bool) -> anyhow::Result<()> {
    let value = serde_json::to_value(value)?;
    if json {
        output::json_output(&value);
    } else {
        eprintln!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

pub async fn fork(
    args: ForkArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    let id = args.product_id;
    let result = match args.action {
        ForkAction::Plan { spec } => {
            return display(
                &client.plan_product_fork(id, read_spec(&spec)?).await?,
                json,
            );
        }
        ForkAction::Usage { fork_id } => {
            return display(&client.product_fork_usage(id, fork_id).await?, json);
        }
        ForkAction::Policy { source_id, action } => {
            let response = match action {
                ForkPolicyAction::Show => client.product_fork_policy(id, source_id).await?,
                ForkPolicyAction::Set { spec } => {
                    client
                        .put_product_fork_policy(id, source_id, read_spec(&spec)?)
                        .await?
                }
                ForkPolicyAction::Disable {
                    expected_generation,
                } => {
                    client
                        .disable_product_fork_policy(
                            id,
                            source_id,
                            ExpectedGenerationRequestBody {
                                expected_generation: expected_generation.to_string(),
                            },
                        )
                        .await?
                }
            };
            return display(&response, json);
        }
        ForkAction::Create { spec } => {
            client
                .create_product_fork(id, read_spec::<PlanRequestBody>(&spec)?)
                .await?
        }
        ForkAction::Show { fork_id } => client.product_fork(id, fork_id).await?,
        ForkAction::Update { fork_id, spec } => {
            client
                .update_product_fork(id, fork_id, read_spec::<UpdateRequestBody>(&spec)?)
                .await?
        }
        ForkAction::Retain {
            fork_id,
            expires_at,
        } => client.retain_product_fork(id, fork_id, expires_at).await?,
        ForkAction::Delete { fork_id, force } => {
            confirm_deletion("Product fork", &fork_id.to_string(), force, json)?;
            client.delete_product_fork(id, fork_id).await?
        }
    };
    display(&result, json)
}

pub async fn publish(
    args: PublishArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
) -> anyhow::Result<()> {
    let token = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&token)?;
    if args.preview {
        let plan = client
            .preview_service_publication(
                args.target_environment,
                PreviewPublicationRequestBody {
                    source_environment_id: args.source_environment,
                    source_composition_id: args.source_composition,
                    service_ids: args.services,
                },
            )
            .await?;
        return display(&plan, json);
    }
    let key = args
        .idempotency_key
        .context("publication requires --idempotency-key")?;
    let result = client
        .publish_services(
            args.target_environment,
            key,
            PublishServiceRequestBody {
                source_environment_id: args.source_environment,
                source_composition_id: args.source_composition,
                service_ids: args.services,
                confirmations: args.confirmations.as_ref().map(read_spec).transpose()?,
            },
        )
        .await?;
    display(&result, json)
}
