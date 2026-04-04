use clap::{Parser, Subcommand};

/// Managed PostgreSQL databases (kaiki)
#[derive(Parser)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,

    /// Project ID (skip auto-detection)
    #[arg(long, global = true)]
    pub project_id: Option<String>,
}

#[derive(Subcommand)]
pub enum DbCommand {
    /// List databases in project
    List,

    /// Create a new managed database
    Create {
        /// Database name
        #[arg(long)]
        name: Option<String>,

        /// Compute unit size (0.25, 0.5, 1.0, 2.0, 4.0, 8.0)
        #[arg(long)]
        cu_size: Option<f64>,

        /// Wait for database to become active before returning
        #[arg(long)]
        wait: bool,
    },

    /// Show database details
    Info {
        /// Database ID or name (default: auto-resolved from config)
        database: Option<String>,
    },

    /// Delete a database
    Delete {
        /// Database ID or name
        database: String,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Start a stopped database
    Start {
        /// Database ID or name (default: auto-resolved from config)
        database: Option<String>,
    },

    /// Stop a running database
    Stop {
        /// Database ID or name (default: auto-resolved from config)
        database: Option<String>,
    },

    /// Show connection string
    Connection {
        /// Database ID or name (default: auto-resolved from config)
        database: Option<String>,

        /// Branch name
        #[arg(long)]
        branch: Option<String>,
    },

    /// Execute SQL query
    Query {
        /// Database ID or name (default: auto-resolved from config)
        #[arg(long)]
        database: Option<String>,

        /// SQL to execute
        #[arg(conflicts_with = "file")]
        sql: Option<String>,

        /// Read SQL from file
        #[arg(long)]
        file: Option<String>,

        /// Target branch name
        #[arg(long)]
        branch: Option<String>,
    },

    /// Manage database branches
    Branches(BranchesArgs),

    /// Show or update auto-inject settings
    Config(ConfigArgs),

    /// Show database schema (tables, columns, types)
    Schema {
        /// Database ID or name (default: auto-resolved from config)
        database: Option<String>,

        /// Target branch name
        #[arg(long)]
        branch: Option<String>,
    },
}

#[derive(Parser)]
pub struct BranchesArgs {
    #[command(subcommand)]
    pub command: Option<BranchesCommand>,

    /// Database ID or name (default: auto-resolved from config)
    #[arg(long, global = true)]
    pub database: Option<String>,
}

#[derive(Subcommand)]
pub enum BranchesCommand {
    /// List branches
    List,

    /// Create a new branch
    Create {
        /// Branch name
        name: String,
    },

    /// Delete a branch
    Delete {
        /// Branch ID or name
        branch: String,
    },

    /// Show branch connection string
    Connection {
        /// Branch ID or name
        branch: String,
    },
}

#[derive(Parser)]
pub struct ConfigArgs {
    /// Database ID or name (default: auto-resolved from config)
    pub database: Option<String>,

    /// Enable or disable auto-inject of DATABASE_URL
    #[arg(long)]
    pub auto_inject: Option<bool>,

    /// Environment variable name for auto-inject
    #[arg(long)]
    pub env_var: Option<String>,

    /// Auto-create preview branches for preview deployments
    #[arg(long)]
    pub preview_branches: Option<bool>,
}
