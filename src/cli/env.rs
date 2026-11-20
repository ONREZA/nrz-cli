use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub command: EnvCommand,

    /// Project ID (skip auto-detection)
    #[arg(long, global = true)]
    pub project_id: Option<String>,

    /// Service ID (required when the Product has multiple Services and none is linked)
    #[arg(long, global = true)]
    pub service: Option<uuid::Uuid>,
}

#[derive(Subcommand)]
pub enum EnvCommand {
    /// List environment variables
    List {
        /// Exact environment ID or name
        #[arg(long, conflicts_with = "all")]
        environment: Option<String>,

        /// List the Service defaults without selecting an environment
        #[arg(long, conflicts_with = "environment")]
        all: bool,
    },

    /// Set an environment variable
    Set {
        /// Variable name
        key: String,

        /// Variable value
        #[arg(long, conflicts_with_all = ["stdin", "from_file"])]
        value: Option<String>,

        /// Read the value from stdin and trim one terminal newline
        #[arg(long, conflicts_with_all = ["value", "from_file"])]
        stdin: bool,

        /// Read the exact value from a UTF-8 file
        #[arg(long, conflicts_with_all = ["value", "stdin"])]
        from_file: Option<String>,

        /// Mark as secret (value will be encrypted)
        #[arg(long, conflicts_with = "plain")]
        secret: bool,

        /// Store as a plain value
        #[arg(long, conflicts_with = "secret")]
        plain: bool,

        /// Non-secret metadata describing the variable
        #[arg(long)]
        note: Option<String>,

        /// Exact environment ID or name
        #[arg(long, conflicts_with = "all")]
        environment: Option<String>,

        /// Set the Service default; existing environment overrides stay unchanged
        #[arg(long, conflicts_with = "environment")]
        all: bool,

        /// Allow replacing an existing legacy scope on this definition
        #[arg(long)]
        replace_scope: bool,

        /// Allow changing plain/secret category
        #[arg(long)]
        change_category: bool,

        /// Confirm changing a scope or category, or replacing a linked Workspace value
        #[arg(long)]
        yes: bool,
    },

    /// Delete an environment variable
    Delete {
        /// Variable name
        key: String,

        /// Exact environment ID or name whose override is removed
        #[arg(long, conflicts_with = "all")]
        environment: Option<String>,

        /// Delete only the Service default; environment overrides stay unchanged
        #[arg(long, conflicts_with = "environment")]
        all: bool,

        /// Confirm deletion non-interactively
        #[arg(long)]
        yes: bool,
    },

    /// Validate one materialized environment against onreza.toml declarations
    Validate {
        /// Exact environment ID or name
        #[arg(long)]
        environment: Option<String>,
    },

    /// Run a command with one materialized environment snapshot
    Exec {
        /// Exact environment ID or name
        #[arg(long)]
        environment: Option<String>,

        /// Command and arguments after `--`
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },
}
