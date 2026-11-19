use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct ServicesArgs {
    #[command(subcommand)]
    pub command: ServicesCommand,
}

#[derive(Subcommand)]
pub enum ServicesCommand {
    /// List Services and available Sources within a Product
    List {
        #[arg(long)]
        project_id: uuid::Uuid,
    },
    /// Create an independent Source for a Service (CLI by default, Git with --git-url)
    CreateSource {
        #[arg(long)]
        project_id: uuid::Uuid,
        #[arg(long)]
        git_url: Option<String>,
        #[arg(long, requires = "git_url")]
        git_connection_id: Option<uuid::Uuid>,
    },
    /// Create a Service using an explicit Source
    Create(ServiceConfiguration),
    /// Update one Service without changing neighboring Services
    Update {
        service_id: uuid::Uuid,
        #[command(flatten)]
        configuration: ServiceConfiguration,
    },
    /// Remove a Service while preserving Product databases and KV data
    Delete {
        /// Explicit Service ID
        #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
        service_id: String,
        /// Product ID that owns this Service
        #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new())]
        project_id: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Args)]
pub struct ServiceConfiguration {
    #[arg(long)]
    pub project_id: uuid::Uuid,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub source_id: uuid::Uuid,
    /// Source ref (create: Git default branch; update: keep the current ref)
    #[arg(long)]
    pub source_ref: Option<String>,
    /// Source root (create: repository root; update: keep the current root)
    #[arg(long)]
    pub root_directory: Option<String>,
}
