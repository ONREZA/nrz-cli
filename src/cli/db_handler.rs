//! CLI handler for `nrz db` subcommands.

use std::path::Path;

use anyhow::Context;
use rusqlite::Connection;

use nrz::emulator::data_dir;

use super::db::{DbArgs, DbCommand};

pub async fn run(args: DbArgs) -> anyhow::Result<()> {
    let project_dir = Path::new(".").canonicalize()?;
    let data_dir = data_dir(&project_dir);
    let db_path = data_dir.join("dev.db");

    match args.command {
        DbCommand::Shell => {
            // TODO: interactive SQLite REPL
            eprintln!("nrz db shell: not yet implemented");
            eprintln!("  use `nrz db execute <sql>` for now");
        }
        DbCommand::Execute { sql } => {
            if !db_path.exists() {
                std::fs::create_dir_all(&data_dir)?;
            }
            let conn = Connection::open(&db_path)
                .with_context(|| format!("failed to open {}", db_path.display()))?;

            let mut stmt = conn
                .prepare(&sql)
                .with_context(|| format!("SQL error: {sql}"))?;

            let col_names: Vec<String> =
                stmt.column_names().iter().map(|s| s.to_string()).collect();

            if col_names.is_empty() {
                // Non-query statement (CREATE, INSERT, etc.)
                let changes = stmt.raw_execute()?;
                eprintln!("{changes} row(s) affected");
            } else {
                // Query with results — print as table
                let mut rows: Vec<Vec<String>> = Vec::new();
                let mut raw_rows = stmt.raw_query();
                while let Some(row) = raw_rows.next()? {
                    let mut values = Vec::new();
                    for i in 0..col_names.len() {
                        let val = match row.get_ref(i)? {
                            rusqlite::types::ValueRef::Null => "NULL".to_string(),
                            rusqlite::types::ValueRef::Integer(n) => n.to_string(),
                            rusqlite::types::ValueRef::Real(f) => f.to_string(),
                            rusqlite::types::ValueRef::Text(s) => {
                                String::from_utf8_lossy(s).into_owned()
                            }
                            rusqlite::types::ValueRef::Blob(b) => {
                                format!("<blob {} bytes>", b.len())
                            }
                        };
                        values.push(val);
                    }
                    rows.push(values);
                }

                // Calculate column widths
                let mut widths: Vec<usize> = col_names.iter().map(|n| n.len()).collect();
                for row in &rows {
                    for (i, val) in row.iter().enumerate() {
                        widths[i] = widths[i].max(val.len());
                    }
                }

                // Print header
                let header: Vec<String> = col_names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| format!("{:width$}", n, width = widths[i]))
                    .collect();
                eprintln!("{}", header.join(" | "));
                let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
                eprintln!("{}", sep.join("-+-"));

                // Print rows
                for row in &rows {
                    let formatted: Vec<String> = row
                        .iter()
                        .enumerate()
                        .map(|(i, v)| format!("{:width$}", v, width = widths[i]))
                        .collect();
                    eprintln!("{}", formatted.join(" | "));
                }

                eprintln!("\n{} row(s)", rows.len());
            }
        }
        DbCommand::Info => {
            if !db_path.exists() {
                eprintln!("database: {} (not created yet)", db_path.display());
                eprintln!("  run `nrz dev` or `nrz db execute` to create it");
                return Ok(());
            }

            let file_size = std::fs::metadata(&db_path)?.len();
            eprintln!("database: {}", db_path.display());
            eprintln!("size: {}", format_size(file_size));

            let conn = Connection::open(&db_path)?;
            let mut stmt =
                conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
            let mut rows = stmt.raw_query();

            let mut tables = Vec::new();
            while let Some(row) = rows.next()? {
                let name: String = row.get(0)?;
                tables.push(name);
            }
            drop(rows);
            drop(stmt);

            if tables.is_empty() {
                eprintln!("tables: (none)");
            } else {
                eprintln!("\ntables:");
                for table in &tables {
                    let count: i64 = conn
                        .query_row(&format!("SELECT COUNT(*) FROM [{table}]"), [], |r| r.get(0))
                        .unwrap_or(0);
                    eprintln!("  {table}: {count} row(s)");
                }
            }
        }
        DbCommand::Reset { force } => {
            if !force {
                eprintln!("use --force to confirm database reset");
                return Ok(());
            }
            if db_path.exists() {
                std::fs::remove_file(&db_path)?;
                // Also remove WAL/SHM files if they exist
                let wal = db_path.with_extension("db-wal");
                let shm = db_path.with_extension("db-shm");
                let _ = std::fs::remove_file(wal);
                let _ = std::fs::remove_file(shm);
                eprintln!("database reset: {}", db_path.display());
            } else {
                eprintln!("database does not exist yet: {}", db_path.display());
            }
        }
    }
    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
