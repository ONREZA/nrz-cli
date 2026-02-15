use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub command: EnvCommand,

    /// Project ID (skip auto-detection)
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Subcommand)]
pub enum EnvCommand {
    /// List environment variables
    List,

    /// Set an environment variable
    Set {
        /// Variable name
        key: String,

        /// Variable value
        value: String,

        /// Mark as secret (value will be encrypted)
        #[arg(long)]
        secret: bool,
    },

    /// Delete an environment variable
    Delete {
        /// Variable name
        key: String,
    },

    /// Pull environment variables to a local file
    Pull {
        /// Output file path
        #[arg(default_value = ".env.local")]
        file: String,
    },
}
