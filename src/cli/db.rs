use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct DbArgs {
    #[command(subcommand)]
    pub command: DbCommand,
}

#[derive(Subcommand)]
pub enum DbCommand {
    /// Open interactive SQLite shell
    Shell,

    /// Execute SQL query (from argument, --file, or stdin)
    Execute {
        /// SQL query to execute (use '-' to read from stdin)
        #[arg(allow_hyphen_values = true)]
        sql: Option<String>,

        /// Read SQL from a file
        #[arg(long, short)]
        file: Option<String>,
    },

    /// Show database info (tables, size)
    Info,

    /// Reset local database (delete and recreate)
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Manage database migrations
    Migrate {
        #[command(subcommand)]
        command: DbMigrateCommand,
    },

    /// Push SQL to remote D1 database
    Push {
        /// SQL to execute (use '-' to read from stdin)
        #[arg(allow_hyphen_values = true)]
        sql: Option<String>,

        /// Read SQL from a file
        #[arg(long, short)]
        file: Option<String>,

        /// Project ID (skip auto-detection)
        #[arg(long)]
        project_id: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum DbMigrateCommand {
    /// Create a new migration file
    Create {
        /// Migration name (e.g. add_users)
        name: String,
    },

    /// Apply pending migrations
    Apply {
        /// Apply to remote D1 database
        #[arg(long)]
        remote: bool,

        /// Show SQL without executing
        #[arg(long)]
        dry_run: bool,

        /// Project ID (skip auto-detection)
        #[arg(long)]
        project_id: Option<String>,
    },

    /// Show migration status
    Status {
        /// Check remote D1 database
        #[arg(long)]
        remote: bool,

        /// Project ID (skip auto-detection)
        #[arg(long)]
        project_id: Option<String>,
    },
}
