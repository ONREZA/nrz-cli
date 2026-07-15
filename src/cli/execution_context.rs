use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct ContextArgs {
    #[command(subcommand)]
    pub command: ContextCommand,

    /// Project ID (skip auto-detection)
    #[arg(long, global = true)]
    pub project_id: Option<String>,

    /// Path to project directory
    #[arg(long, global = true, default_value = ".")]
    pub dir: String,
}

#[derive(Subcommand)]
pub enum ContextCommand {
    /// Show the saved execution context
    Show,

    /// Resolve and save an environment for this checkout
    Use {
        /// Environment ID or exact name
        environment: String,
    },

    /// Remove the saved execution context
    Clear,
}
