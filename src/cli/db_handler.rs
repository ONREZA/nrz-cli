//! CLI handler for `nrz db` subcommands — managed PostgreSQL (kaiki).

use std::io::{IsTerminal, Write};

use anyhow::{Context, bail};

use super::db::{BranchesCommand, ConfigArgs, DbArgs, DbCommand};
use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config::ProjectConfig;

mod attachment;
mod branches;
mod databases;
mod models;
mod query;
mod sql;
mod wire;
use attachment::*;
use branches::*;
use databases::*;
use models::*;
use query::*;
use sql::execute_sql_locally;

// ── Main entry point ────────────────────────────────────────

pub async fn run(
    args: DbArgs,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let project_id = nrz::config::resolve_project_id(args.project_id.as_deref(), config)?;

    match args.command {
        DbCommand::List => cmd_list(&client, &project_id, json).await,
        DbCommand::Create {
            name,
            cu_size,
            wait,
        } => cmd_create(&client, &project_id, json, name, cu_size, wait).await,
        DbCommand::Info { database } => {
            let db_id = resolve_db(&client, &project_id, database.as_deref(), config).await?;
            cmd_info(&client, &project_id, &db_id, json).await
        }
        DbCommand::Delete { database, force } => {
            let db_id = resolve_db(&client, &project_id, Some(&database), config).await?;
            cmd_delete(&client, &db_id, json, force).await
        }
        DbCommand::Start { database } => {
            let db_id = resolve_db(&client, &project_id, database.as_deref(), config).await?;
            cmd_start_stop(&client, &db_id, json, "start").await
        }
        DbCommand::Stop { database } => {
            let db_id = resolve_db(&client, &project_id, database.as_deref(), config).await?;
            cmd_start_stop(&client, &db_id, json, "stop").await
        }
        DbCommand::Connection { database, branch } => {
            let db_id = resolve_db(&client, &project_id, database.as_deref(), config).await?;
            cmd_connection(&client, &db_id, json, branch.as_deref()).await
        }
        DbCommand::Query {
            database,
            sql,
            file,
            branch,
        } => {
            let db_id = resolve_db(&client, &project_id, database.as_deref(), config).await?;
            let sql = resolve_sql(sql.as_deref(), file.as_deref())?;
            cmd_query(&client, &db_id, json, &sql, branch.as_deref()).await
        }
        DbCommand::Branches(bargs) => {
            let db_id = resolve_db(&client, &project_id, bargs.database.as_deref(), config).await?;
            match bargs.command {
                None | Some(BranchesCommand::List) => {
                    cmd_branches_list(&client, &db_id, json).await
                }
                Some(BranchesCommand::Create { name }) => {
                    cmd_branches_create(&client, &db_id, json, &name).await
                }
                Some(BranchesCommand::Delete { branch }) => {
                    cmd_branches_delete(&client, &db_id, json, &branch).await
                }
                Some(BranchesCommand::Connection { branch }) => {
                    cmd_branch_connection(&client, &db_id, json, &branch).await
                }
            }
        }
        DbCommand::Config(cargs) => {
            let db_id = resolve_db(&client, &project_id, cargs.database.as_deref(), config).await?;
            cmd_config(&client, &project_id, &db_id, json, cargs).await
        }
        DbCommand::Schema { database, branch } => {
            let db_id = resolve_db(&client, &project_id, database.as_deref(), config).await?;
            cmd_schema(&client, &db_id, json, branch.as_deref()).await
        }
    }
}

// ── Database resolution ─────────────────────────────────────

/// Resolve database ID from explicit arg, config, or by listing databases.
async fn resolve_db(
    client: &ApiClient,
    project_id: &str,
    explicit: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    // 1. Explicit argument
    if let Some(val) = explicit {
        let val = val.trim();
        if val.is_empty() {
            anyhow::bail!("--database requires a non-empty database ID or name");
        }
        return resolve_db_by_id_or_name(client, project_id, val).await;
    }

    // 2. Config: [db] database
    if let Some(val) = config
        .db_database()
        .map(str::trim)
        .filter(|val| !val.is_empty())
    {
        return resolve_db_by_id_or_name(client, project_id, val).await;
    }

    // Resolve only a unique project attachment.
    let list: ListResponse = client
        .databases()
        .await
        .context("failed to list databases")?
        .into();

    Ok(select_database(&list.data, project_id, None)?.id.clone())
}

async fn resolve_db_by_id_or_name(
    client: &ApiClient,
    project_id: &str,
    val: &str,
) -> anyhow::Result<String> {
    let list: ListResponse = client
        .databases()
        .await
        .context("failed to list databases")?
        .into();

    Ok(select_database(&list.data, project_id, Some(val))?
        .id
        .clone())
}

fn select_database<'a>(
    databases: &'a [ManagedDatabase],
    project_id: &str,
    value: Option<&str>,
) -> anyhow::Result<&'a ManagedDatabase> {
    let attached: Vec<_> = databases
        .iter()
        .filter(|db| db.is_attached_to_project(project_id))
        .collect();
    let candidates: Vec<_> = if let Some(value) = value {
        if let Some(db) = attached.iter().find(|db| db.id == value) {
            return Ok(db);
        }
        attached
            .into_iter()
            .filter(|db| db.db_name.as_deref() == Some(value))
            .collect()
    } else {
        let injected: Vec<_> = attached
            .iter()
            .copied()
            .filter(|db| db.auto_inject_db_url_for_project(project_id) == Some(true))
            .collect();
        if injected.is_empty() {
            attached
        } else {
            injected
        }
    };
    match candidates.as_slice() {
        [db] => Ok(db),
        [] => bail!("no matching database found in project"),
        _ => bail!("multiple databases match; specify an exact database ID"),
    }
}

fn resolve_sql(sql: Option<&str>, file: Option<&str>) -> anyhow::Result<String> {
    match (sql, file) {
        (Some(s), _) => Ok(s.to_string()),
        (_, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read SQL file: {path}")),
        (None, None) => {
            // Try stdin
            if std::io::stdin().is_terminal() {
                bail!("no SQL provided — pass as argument, --file, or pipe via stdin");
            }
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                .context("failed to read SQL from stdin")?;
            if buf.trim().is_empty() {
                bail!("empty SQL input");
            }
            Ok(buf)
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────

async fn fetch_connection_uri(
    client: &ApiClient,
    db_id: &str,
    branch: Option<&str>,
) -> anyhow::Result<String> {
    let response = if let Some(branch_name) = branch {
        let branch_id = resolve_branch(client, db_id, branch_name).await?;
        client
            .database_branch_connection(db_id, &branch_id)
            .await
            .context("failed to get branch connection")?
    } else {
        client
            .database_connection(db_id)
            .await
            .context("failed to get connection")?
    };
    Ok(response.connection_uri)
}

async fn resolve_branch(client: &ApiClient, db_id: &str, branch: &str) -> anyhow::Result<String> {
    let list = client
        .database_branches(db_id)
        .await
        .context("failed to list branches")?;

    if let Some(found) = list.iter().find(|item| item.id == branch) {
        return Ok(found.id.clone());
    }
    let mut matching = list.iter().filter(|item| item.name == branch);
    let found = matching
        .next()
        .ok_or_else(|| anyhow::anyhow!("branch \"{branch}\" not found"))?;
    anyhow::ensure!(
        matching.next().is_none(),
        "multiple branches have this name; specify an exact branch ID"
    );
    Ok(found.id.clone())
}

fn format_status(s: &str) -> console::StyledObject<&str> {
    match s.to_lowercase().as_str() {
        "active" | "running" => console::style(s).green(),
        "creating" | "starting" => console::style(s).yellow(),
        "stopped" | "deleted" | "deleting" => console::style(s).red(),
        "error" => console::style(s).red().bold(),
        _ => console::style(s).dim(),
    }
}

fn format_cell(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "(null)".to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn value_as_str(row: &[serde_json::Value], idx: usize) -> Option<&str> {
    row.get(idx).and_then(|v| v.as_str())
}

#[cfg(test)]
#[path = "db_handler_tests.rs"]
mod db_handler_tests;

#[cfg(test)]
#[path = "db_handler/wire_tests.rs"]
mod wire_tests;
