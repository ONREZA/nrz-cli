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

    /// Attach a custom domain hostname
    Add {
        /// Domain name (e.g. example.com)
        domain: String,

        /// Target environment (name or ID)
        #[arg(long)]
        environment: Option<String>,
    },

    /// Remove a custom domain hostname
    Remove {
        /// Hostname/domain binding ID to remove
        domain_id: String,
    },

    /// Verify the workspace domain zone for a hostname
    Verify {
        /// Hostname/domain binding ID to verify
        domain_id: String,
    },
}
