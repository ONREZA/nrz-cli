use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct ServicesArgs {
    #[command(subcommand)]
    pub command: ServicesCommand,
}

#[derive(Subcommand)]
pub enum ServicesCommand {
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
