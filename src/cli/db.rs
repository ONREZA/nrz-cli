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

    /// Execute a SQL query
    Execute {
        /// SQL query to execute
        sql: String,
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
