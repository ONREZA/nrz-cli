use crate::cli::db::{DbArgs, DbCommand};

/// CLI handler for `nrz db` subcommands.
///
/// Uses rusqlite for local D1-compatible SQLite.
/// Database file: .onreza/data/dev.db
pub async fn run(args: DbArgs) -> anyhow::Result<()> {
    match args.command {
        DbCommand::Shell => {
            eprintln!("nrz db shell: not yet implemented");
            // TODO: interactive SQLite REPL via rusqlite
        }
        DbCommand::Execute { sql } => {
            eprintln!("nrz db execute: not yet implemented (sql: {sql})");
            // TODO: execute query, print results as table
        }
        DbCommand::Info => {
            eprintln!("nrz db info: not yet implemented");
            // TODO: show tables, size, row counts
        }
        DbCommand::Reset { force } => {
            if !force {
                eprintln!("use --force to confirm database reset");
                return Ok(());
            }
            eprintln!("nrz db reset: not yet implemented");
            // TODO: delete and recreate .onreza/data/dev.db
        }
    }
    Ok(())
}
