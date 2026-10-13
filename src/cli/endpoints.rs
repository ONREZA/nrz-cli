use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
pub struct EndpointsArgs {
    #[command(flatten)]
    pub scope: EndpointScope,
    #[command(subcommand)]
    pub command: EndpointCommand,
}
#[derive(Args)]
pub struct EndpointScope {
    #[arg(long)]
    pub project_id: uuid::Uuid,
    #[arg(long)]
    pub service_id: uuid::Uuid,
    #[arg(long)]
    pub environment_id: uuid::Uuid,
}
#[derive(Clone, ValueEnum)]
pub enum Exposure {
    Public,
    Private,
}
#[derive(Subcommand)]
pub enum EndpointCommand {
    /// Inspect declared network, sharing and actual applied state
    List,
    /// Publish or revoke explicit provider port exports
    Export {
        layer: String,
        #[arg(long, value_delimiter = ',', required = true)]
        ports: Vec<String>,
        #[arg(long)]
        expected_generation: i64,
        #[arg(long)]
        revoke: bool,
    },
    /// Grant provider imports with both Product permissions, or revoke this caller's existing import
    Import {
        layer: String,
        #[arg(long)]
        export_id: uuid::Uuid,
        #[arg(long)]
        export_generation: i64,
        #[arg(long, value_delimiter = ',', required = true)]
        ports: Vec<String>,
        #[arg(long)]
        expected_generation: i64,
        #[arg(long)]
        revoke: bool,
    },
    /// Update the complete network policy from JSON without changing exposure
    Network {
        layer: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        expected_generation: i64,
    },
    /// Explicitly change exposure; promotion requires a JSON public route preview
    Exposure {
        layer: String,
        #[arg(value_enum)]
        exposure: Exposure,
        #[arg(long)]
        expected_generation: i64,
        #[arg(long)]
        public_route_preview: Option<PathBuf>,
    },
}
