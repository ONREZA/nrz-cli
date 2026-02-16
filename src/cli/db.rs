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
}
