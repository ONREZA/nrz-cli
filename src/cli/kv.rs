use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct KvArgs {
    #[command(subcommand)]
    pub command: KvCommand,
}

#[derive(Subcommand)]
pub enum KvCommand {
    /// Get a value by key
    Get {
        key: String,
    },

    /// Set a key-value pair
    Set {
        key: String,
        value: String,

        /// TTL in seconds (0 = no expiry)
        #[arg(long, default_value = "0")]
        ttl: u64,
    },

    /// Delete a key
    Delete {
        key: String,
    },

    /// List keys with optional prefix
    List {
        /// Key prefix filter
        #[arg(long)]
        prefix: Option<String>,

        /// Max number of keys to return
        #[arg(long, default_value = "100")]
        limit: usize,
    },

    /// Clear all KV data
    Clear {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}
