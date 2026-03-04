//! CLI handler for `nrz db` subcommands.

use std::io::{BufRead, IsTerminal, Read as _, Write as _};
use std::path::Path;

use anyhow::{Context, bail};
use rusqlite::Connection;
use serde::Serialize;

use nrz::config::ProjectConfig;

use super::db::{DbArgs, DbCommand};
use crate::api::ApiClient;
use crate::auth;
use crate::link::environment_ref;
use crate::output;
use nrz::config;

#[derive(Serialize)]
struct DbExecuteOutput {
    changes: usize,
}

#[derive(Serialize)]
struct DbBatchOutput {
    batch: bool,
    changes_last_statement: usize,
}

#[derive(Serialize)]
struct DbQueryOutput {
    columns: Vec<String>,
    rows: Vec<Vec<serde_json::Value>>,
}

#[derive(Serialize)]
struct DbInfoOutput {
    path: String,
    size: u64,
    tables: Vec<TableInfo>,
}

#[derive(Serialize)]
struct TableInfo {
    name: String,
    rows: i64,
}

#[derive(Serialize)]
struct StatusOutput {
    status: String,
}

pub async fn run(
    args: DbArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = Path::new(".").canonicalize()?;
    let env = env.or(config.db.default_env.as_deref());
    let data_dir = config.data_dir_path(&project_dir);
    let db_path = data_dir.join(config.db_name());

    match args.command {
        DbCommand::Shell => {
            eprintln!("nrz db shell: not yet implemented");
            eprintln!("  use `nrz db execute <sql>` for now");
        }
        DbCommand::Execute { sql, file } => {
            // Resolve SQL source: --file, stdin ("-"), or positional argument
            let sql = if let Some(path) = file {
                std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?
            } else if sql.as_deref() == Some("-")
                || (sql.is_none() && !std::io::stdin().is_terminal())
            {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .context("failed to read from stdin")?;
                buf
            } else if let Some(s) = sql {
                s
            } else {
                bail!("provide SQL as argument, --file <path>, or pipe to stdin");
            };

            let sql = sql.trim();
            if sql.is_empty() {
                bail!("SQL input is empty");
            }

            if !db_path.exists() {
                std::fs::create_dir_all(&data_dir).with_context(|| {
                    format!("failed to create data directory {}", data_dir.display())
                })?;
            }
            let mut conn = Connection::open(&db_path)
                .with_context(|| format!("failed to open {}", db_path.display()))?;

            if is_multi_statement(sql) {
                let tx = conn.transaction().context("failed to begin transaction")?;
                tx.execute_batch(sql)
                    .context("SQL batch execution failed (all changes rolled back)")?;
                let changes = tx.changes() as usize;
                tx.commit().context("failed to commit transaction")?;
                if json {
                    output::json_output(&DbBatchOutput {
                        batch: true,
                        changes_last_statement: changes,
                    });
                } else {
                    eprintln!("batch executed ({changes} row(s) affected by last statement)");
                }
            } else {
                execute_single(&conn, sql, json)?;
            }
        }
        DbCommand::Info => {
            if !db_path.exists() {
                if json {
                    output::json_output(&DbInfoOutput {
                        path: db_path.to_string_lossy().into_owned(),
                        size: 0,
                        tables: vec![],
                    });
                } else {
                    eprintln!("database: {} (not created yet)", db_path.display());
                    eprintln!("  run `nrz dev` or `nrz db execute` to create it");
                }
                return Ok(());
            }

            let file_size = std::fs::metadata(&db_path)?.len();

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

            let table_info: Vec<TableInfo> = tables
                .iter()
                .map(|table| {
                    let count: i64 = conn
                        .query_row(&format!("SELECT COUNT(*) FROM [{table}]"), [], |r| r.get(0))
                        .unwrap_or(0);
                    TableInfo {
                        name: table.clone(),
                        rows: count,
                    }
                })
                .collect();

            if json {
                output::json_output(&DbInfoOutput {
                    path: db_path.to_string_lossy().into_owned(),
                    size: file_size,
                    tables: table_info,
                });
            } else {
                eprintln!("database: {}", db_path.display());
                eprintln!("size: {}", format_size(file_size));

                if table_info.is_empty() {
                    eprintln!("tables: (none)");
                } else {
                    eprintln!("\ntables:");
                    for t in &table_info {
                        eprintln!("  {}: {} row(s)", t.name, t.rows);
                    }
                }
            }
        }
        DbCommand::Migrate { command } => {
            return super::db_migrate_handler::handle_migrate(
                command, json, token, workspace, env, config,
            )
            .await;
        }
        DbCommand::Push {
            sql,
            file,
            project_id,
        } => {
            return super::db_migrate_handler::handle_push(
                sql, file, project_id, json, token, workspace, env, config,
            )
            .await;
        }
        DbCommand::Reset {
            force,
            remote,
            project_id,
        } => {
            if remote {
                return reset_remote(
                    project_id.as_deref(),
                    force,
                    json,
                    token,
                    workspace,
                    env,
                    config,
                )
                .await;
            }
            if !force {
                eprintln!("use --force to confirm database reset");
                return Ok(());
            }
            if db_path.exists() {
                std::fs::remove_file(&db_path)?;
                let wal = db_path.with_extension("db-wal");
                let shm = db_path.with_extension("db-shm");
                let _ = std::fs::remove_file(wal);
                let _ = std::fs::remove_file(shm);
                if json {
                    output::json_output(&StatusOutput {
                        status: "ok".into(),
                    });
                } else {
                    eprintln!("database reset: {}", db_path.display());
                }
            } else if json {
                output::json_output(&StatusOutput {
                    status: "ok".into(),
                });
            } else {
                eprintln!("database does not exist yet: {}", db_path.display());
            }
        }
    }
    Ok(())
}

/// Check if SQL contains multiple statements.
/// Handles `'...'`, `"..."`, `-- ...`, and `/* ... */`.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn is_multi_statement(sql: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut found_semi = false;
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '-' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'-') {
                    for c2 in chars.by_ref() {
                        if c2 == '\n' {
                            break;
                        }
                    }
                }
            }
            '/' if !in_single_quote && !in_double_quote => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume '*'
                    loop {
                        match chars.next() {
                            Some('*') if chars.peek() == Some(&'/') => {
                                chars.next(); // consume '/'
                                break;
                            }
                            Some(_) => {}
                            None => break,
                        }
                    }
                }
            }
            ';' if !in_single_quote && !in_double_quote => {
                if found_semi {
                    return true;
                }
                found_semi = true;
            }
            c if !c.is_whitespace() && !in_single_quote && !in_double_quote && found_semi => {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn execute_single(conn: &Connection, sql: &str, json: bool) -> anyhow::Result<()> {
    let mut stmt = conn
        .prepare(sql)
        .with_context(|| format!("SQL error: {sql}"))?;

    let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    if col_names.is_empty() {
        let changes = stmt.raw_execute()?;
        if json {
            output::json_output(&DbExecuteOutput { changes });
        } else {
            eprintln!("{changes} row(s) affected");
        }
    } else {
        let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
        let mut raw_rows = stmt.raw_query();
        while let Some(row) = raw_rows.next()? {
            let mut values = Vec::new();
            for i in 0..col_names.len() {
                let val = match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(n) => serde_json::Value::Number(n.into()),
                    rusqlite::types::ValueRef::Real(f) => serde_json::json!(f),
                    rusqlite::types::ValueRef::Text(s) => {
                        serde_json::Value::String(String::from_utf8_lossy(s).into_owned())
                    }
                    rusqlite::types::ValueRef::Blob(b) => {
                        serde_json::Value::String(format!("<blob {} bytes>", b.len()))
                    }
                };
                values.push(val);
            }
            rows.push(values);
        }

        if json {
            output::json_output(&DbQueryOutput {
                columns: col_names,
                rows,
            });
        } else {
            print_table(&col_names, &rows);
        }
    }
    Ok(())
}

fn print_table(col_names: &[String], rows: &[Vec<serde_json::Value>]) {
    let str_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| match v {
                    serde_json::Value::Null => "NULL".to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect()
        })
        .collect();

    let mut widths: Vec<usize> = col_names.iter().map(|n| n.len()).collect();
    for row in &str_rows {
        for (i, val) in row.iter().enumerate() {
            widths[i] = widths[i].max(val.len());
        }
    }

    let header: Vec<String> = col_names
        .iter()
        .enumerate()
        .map(|(i, n)| format!("{:width$}", n, width = widths[i]))
        .collect();
    eprintln!("{}", header.join(" | "));
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    eprintln!("{}", sep.join("-+-"));

    for row in &str_rows {
        let formatted: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{:width$}", v, width = widths[i]))
            .collect();
        eprintln!("{}", formatted.join(" | "));
    }

    eprintln!("\n{} row(s)", str_rows.len());
}

async fn reset_remote(
    project_id: Option<&str>,
    force: bool,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let pid = config::resolve_project_id(project_id, config)?;
    let (eid, env_type) = environment_ref::resolve_environment_id(env, &pid, &client, json).await?;

    // Require confirmation for production environments
    if !force {
        if json || !std::io::stdin().is_terminal() {
            bail!("--force is required to reset remote database in non-interactive mode");
        }

        if env_type == environment_ref::EnvironmentType::Production {
            eprintln!(
                "  {} This will delete ALL data in the production database.",
                console::style("WARNING:").red().bold(),
            );
            eprint!(
                "  {} ",
                console::style("Type 'production' to confirm:").bold(),
            );
            std::io::stderr().flush()?;

            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line)?;
            if line.trim() != "production" {
                bail!("reset cancelled");
            }
        } else {
            eprintln!(
                "  {} This will delete ALL data in the remote database.",
                console::style("WARNING:").red().bold(),
            );
            eprint!("  {} ", console::style("Continue? (y/N):").bold());
            std::io::stderr().flush()?;

            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line)?;
            if !matches!(line.trim().to_lowercase().as_str(), "y" | "yes") {
                bail!("reset cancelled");
            }
        }
    }

    output::status(json, "~", "Resetting remote database...", output::Phase::Db);

    let resp: serde_json::Value = client
        .post(
            &format!("/api/d1/databases/{pid}/reset?environmentId={eid}"),
            &serde_json::json!({}),
        )
        .await
        .context("failed to reset remote database")?;

    if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
        bail!("remote database reset failed: {err}");
    }

    if json {
        output::json_output(&StatusOutput {
            status: "ok".into(),
        });
    } else {
        output::success(
            false,
            "Remote database reset successfully.",
            output::Phase::Db,
        );
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
