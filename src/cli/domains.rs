use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct DomainsArgs {
    #[command(subcommand)]
    pub command: DomainsCommand,

    /// Project ID (skip auto-detection)
    #[arg(long, global = true)]
    pub project_id: Option<String>,
}

#[derive(Subcommand)]
pub enum DomainsCommand {
    /// List custom domains
    List,

    /// Add a custom domain
    Add {
        /// Domain name (e.g. example.com)
        domain: String,

        /// Environment ID (default: production)
        #[arg(long)]
        environment_id: Option<String>,
    },

    /// Remove a custom domain
    Remove {
        /// Domain ID to remove
        domain_id: String,
    },

    /// Verify a custom domain
    Verify {
        /// Domain ID to verify
        domain_id: String,
    },
}
