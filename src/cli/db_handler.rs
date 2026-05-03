//! CLI handler for `nrz db` subcommands — managed PostgreSQL (kaiki).

use std::io::{IsTerminal, Write};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use super::db::{BranchesCommand, ConfigArgs, DbArgs, DbCommand};
use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config::ProjectConfig;

// ── API response types ──────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedDatabase {
    id: String,
    #[serde(default)]
    db_name: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    cu_size: Option<f64>,
    #[serde(default)]
    pg_version: Option<i32>,
    #[serde(default)]
    auto_inject_db_url: Option<bool>,
    #[serde(default)]
    env_var_name: Option<String>,
    #[serde(default)]
    auto_create_preview_branch: Option<bool>,
    #[serde(default)]
    kaiki_status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    data: Vec<ManagedDatabase>,
    #[serde(default)]
    allowed_cu_sizes: Option<Vec<f64>>,
    #[serde(default)]
    autoscale_max_cu: Option<f64>,
    #[serde(default)]
    plan: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DbInfoResponse {
    #[serde(flatten)]
    db: ManagedDatabase,
    #[serde(default)]
    allowed_cu_sizes: Option<Vec<f64>>,
    #[serde(default)]
    autoscale_max_cu: Option<f64>,
    #[serde(default)]
    plan: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateResponse {
    id: String,
    #[serde(default)]
    db_name: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionResponse {
    #[serde(alias = "connection_uri")]
    connection_uri: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Branch {
    id: String,
    name: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    is_preview_branch: Option<bool>,
}

#[derive(Debug, Serialize)]
struct BranchListResponse {
    data: Vec<Branch>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BranchListWire {
    Envelope { data: Vec<Branch> },
    Direct(Vec<Branch>),
}

impl<'de> Deserialize<'de> for BranchListResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match BranchListWire::deserialize(deserializer)? {
            BranchListWire::Envelope { data } | BranchListWire::Direct(data) => Self { data },
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryResult {
    #[serde(default)]
    columns: Option<Vec<String>>,
    #[serde(default)]
    rows: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    row_count: Option<i64>,
    #[serde(default)]
    duration_ms: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevEnvResponse {
    pub env_vars: std::collections::HashMap<String, String>,
    pub database: DevEnvDatabase,
    pub branch: Option<DevEnvBranch>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DevEnvDatabase {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DevEnvBranch {
    pub id: String,
    pub name: String,
}

// ── Request bodies ──────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    db_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cu_size: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryBody {
    sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_name: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateBranchBody {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoInjectBody {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    env_var_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_create_preview_branch: Option<bool>,
}

// ── Schema introspection types ──────────────────────────────

#[derive(Debug, Serialize)]
struct SchemaOutput {
    tables: Vec<SchemaTable>,
}

#[derive(Debug, Serialize)]
struct SchemaTable {
    name: String,
    columns: Vec<SchemaColumn>,
}

#[derive(Debug, Serialize)]
struct SchemaColumn {
    name: String,
    #[serde(rename = "type")]
    col_type: String,
    nullable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
}

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
    let base = format!("/v1/managed-databases/{project_id}");

    match args.command {
        DbCommand::List => cmd_list(&client, &base, json).await,
        DbCommand::Create {
            name,
            cu_size,
            wait,
        } => cmd_create(&client, &base, json, name, cu_size, wait).await,
        DbCommand::Info { database } => {
            let db_id = resolve_db(&client, &base, database.as_deref(), config).await?;
            cmd_info(&client, &base, &db_id, json).await
        }
        DbCommand::Delete { database, force } => {
            let db_id = resolve_db(&client, &base, Some(&database), config).await?;
            cmd_delete(&client, &base, &db_id, json, force).await
        }
        DbCommand::Start { database } => {
            let db_id = resolve_db(&client, &base, database.as_deref(), config).await?;
            cmd_start_stop(&client, &base, &db_id, json, "start").await
        }
        DbCommand::Stop { database } => {
            let db_id = resolve_db(&client, &base, database.as_deref(), config).await?;
            cmd_start_stop(&client, &base, &db_id, json, "stop").await
        }
        DbCommand::Connection { database, branch } => {
            let db_id = resolve_db(&client, &base, database.as_deref(), config).await?;
            cmd_connection(&client, &base, &db_id, json, branch.as_deref()).await
        }
        DbCommand::Query {
            database,
            sql,
            file,
            branch,
        } => {
            let db_id = resolve_db(&client, &base, database.as_deref(), config).await?;
            let sql = resolve_sql(sql.as_deref(), file.as_deref())?;
            cmd_query(&client, &base, &db_id, json, &sql, branch.as_deref()).await
        }
        DbCommand::Branches(bargs) => {
            let db_id = resolve_db(&client, &base, bargs.database.as_deref(), config).await?;
            match bargs.command {
                None | Some(BranchesCommand::List) => {
                    cmd_branches_list(&client, &base, &db_id, json).await
                }
                Some(BranchesCommand::Create { name }) => {
                    cmd_branches_create(&client, &base, &db_id, json, &name).await
                }
                Some(BranchesCommand::Delete { branch }) => {
                    cmd_branches_delete(&client, &base, &db_id, json, &branch).await
                }
                Some(BranchesCommand::Connection { branch }) => {
                    cmd_branch_connection(&client, &base, &db_id, json, &branch).await
                }
            }
        }
        DbCommand::Config(cargs) => {
            let db_id = resolve_db(&client, &base, cargs.database.as_deref(), config).await?;
            cmd_config(&client, &base, &db_id, json, cargs).await
        }
        DbCommand::Schema { database, branch } => {
            let db_id = resolve_db(&client, &base, database.as_deref(), config).await?;
            cmd_schema(&client, &base, &db_id, json, branch.as_deref()).await
        }
    }
}

// ── Database resolution ─────────────────────────────────────

/// Resolve database ID from explicit arg, config, or by listing databases.
async fn resolve_db(
    client: &ApiClient,
    base: &str,
    explicit: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    // 1. Explicit argument
    if let Some(val) = explicit
        && !val.is_empty()
    {
        return resolve_db_by_id_or_name(client, base, val).await;
    }

    // 2. Config: [db] database
    if let Some(val) = config.db_database() {
        return resolve_db_by_id_or_name(client, base, val).await;
    }

    // 3. Auto-resolve: first auto-inject, or first available
    let list: ListResponse = client.get(base).await.context("failed to list databases")?;

    let db = list
        .data
        .iter()
        .find(|d| d.auto_inject_db_url == Some(true))
        .or(list.data.first())
        .ok_or_else(|| anyhow::anyhow!("no databases found in project"))?;

    Ok(db.id.clone())
}

async fn resolve_db_by_id_or_name(
    client: &ApiClient,
    base: &str,
    val: &str,
) -> anyhow::Result<String> {
    // If it looks like an ID (UUIDv7 hex or with hyphens), use directly
    if looks_like_id(val) {
        return Ok(val.to_string());
    }

    // Resolve name → id via list
    let list: ListResponse = client.get(base).await.context("failed to list databases")?;

    list.data
        .iter()
        .find(|d| d.id == val || d.db_name.as_deref() == Some(val))
        .map(|d| d.id.clone())
        .ok_or_else(|| anyhow::anyhow!("database \"{val}\" not found"))
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

// ── Command implementations ─────────────────────────────────

async fn cmd_list(client: &ApiClient, base: &str, json: bool) -> anyhow::Result<()> {
    let list: ListResponse = client.get(base).await.context("failed to list databases")?;

    if json {
        output::json_output(&list);
        return Ok(());
    }

    if list.data.is_empty() {
        eprintln!("  No databases found. Create one with: nrz db create");
        return Ok(());
    }

    if let Some(plan) = &list.plan {
        eprintln!("  Plan: {plan}");
    }
    if let Some(sizes) = &list.allowed_cu_sizes {
        let s: Vec<String> = sizes.iter().map(|v| format!("{v}")).collect();
        eprintln!("  Allowed CU sizes: {}", s.join(", "));
    }
    eprintln!();

    for db in &list.data {
        let name = db.db_name.as_deref().unwrap_or("(unnamed)");
        let status = db.status.as_deref().unwrap_or("unknown");
        let cu = db.cu_size.map(|v| format!("{v}")).unwrap_or_default();
        let inject = if db.auto_inject_db_url == Some(true) {
            " [auto-inject]"
        } else {
            ""
        };
        eprintln!(
            "  {} {} ({}CU, {}){inject}",
            console::style(&db.id).dim(),
            console::style(name).bold(),
            cu,
            format_status(status),
        );
    }
    Ok(())
}

async fn cmd_create(
    client: &ApiClient,
    base: &str,
    json: bool,
    name: Option<String>,
    cu_size: Option<f64>,
    wait: bool,
) -> anyhow::Result<()> {
    output::status(json, "~", "Creating database...", output::Phase::Db);

    let body = CreateBody {
        db_name: name,
        cu_size,
    };
    let created: CreateResponse = client
        .post(base, &body)
        .await
        .context("failed to create database")?;

    if wait {
        output::status(
            json,
            "~",
            "Waiting for database to become active...",
            output::Phase::Db,
        );
        let db_url = format!("{}/{}", base, created.id);
        for _ in 0..120 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let info: serde_json::Value = client
                .get(&db_url)
                .await
                .context("failed to check database status")?;
            if let Some(status) = info.get("status").and_then(|v| v.as_str()) {
                match status {
                    "ACTIVE" | "active" => {
                        output::success(json, "Database is active", output::Phase::Db);
                        if json {
                            output::json_output(&info);
                        }
                        return Ok(());
                    }
                    "ERROR" | "error" => {
                        let detail = info
                            .get("error")
                            .or(info.get("message"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("no details available");
                        bail!("database creation failed: {detail}");
                    }
                    _ => continue,
                }
            }
        }
        bail!(
            "timed out waiting for database to become active (~4 minutes). \
             Check status with: nrz db info"
        );
    }

    if json {
        output::json_output(&created);
    } else {
        let name = created.db_name.as_deref().unwrap_or("kaikidb");
        output::success(
            json,
            format!("Database created: {} ({})", name, created.id),
            output::Phase::Db,
        );
        if created.status.as_deref() == Some("CREATING") {
            eprintln!("    Database is being provisioned. Use `nrz db info` to check status.");
        }
    }
    Ok(())
}

async fn cmd_info(client: &ApiClient, base: &str, db_id: &str, json: bool) -> anyhow::Result<()> {
    let url = format!("{}/{}", base, db_id);
    let info: DbInfoResponse = client
        .get(&url)
        .await
        .context("failed to get database info")?;

    if json {
        output::json_output(&info);
        return Ok(());
    }

    let db = &info.db;
    let name = db.db_name.as_deref().unwrap_or("(unnamed)");
    let status = db.status.as_deref().unwrap_or("unknown");
    eprintln!(
        "  {} {}",
        console::style(name).bold(),
        format_status(status),
    );
    eprintln!("  ID:         {}", db.id);
    if let Some(cu) = db.cu_size {
        eprintln!("  CU size:    {cu}");
    }
    if let Some(pg) = db.pg_version {
        eprintln!("  PostgreSQL: {pg}");
    }
    if let Some(true) = db.auto_inject_db_url {
        let var = db.env_var_name.as_deref().unwrap_or("DATABASE_URL");
        eprintln!("  Auto-inject: {var}");
        if db.auto_create_preview_branch == Some(true) {
            eprintln!("  Preview branches: enabled");
        }
    }
    if let Some(plan) = &info.plan {
        eprintln!("  Plan:       {plan}");
    }
    if let Some(sizes) = &info.allowed_cu_sizes {
        let s: Vec<String> = sizes.iter().map(|v| format!("{v}")).collect();
        eprintln!("  Allowed CU: {}", s.join(", "));
    }

    Ok(())
}

async fn cmd_delete(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    force: bool,
) -> anyhow::Result<()> {
    if !force {
        if json || !std::io::stdin().is_terminal() {
            bail!("--force is required to delete database in non-interactive mode");
        }
        eprint!(
            "  {} Delete database {db_id}? [y/N] ",
            console::style("?").yellow().bold()
        );
        std::io::stderr().flush()?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            eprintln!("  Cancelled.");
            return Ok(());
        }
    }

    let url = format!("{}/{}", base, db_id);
    client
        .delete_empty(&url)
        .await
        .context("failed to delete database")?;

    if json {
        output::json_output(&serde_json::json!({"deleted": db_id}));
    } else {
        output::success(
            false,
            format!("Database {db_id} deleted"),
            output::Phase::Db,
        );
    }
    Ok(())
}

async fn cmd_start_stop(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    action: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/{}/{}", base, db_id, action);
    let resp: serde_json::Value = client
        .post_empty(&url)
        .await
        .with_context(|| format!("failed to {action} database"))?;

    if json {
        output::json_output(&resp);
    } else {
        let status = resp
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or(action);
        output::success(
            false,
            format!("Database {db_id}: {status}"),
            output::Phase::Db,
        );
    }
    Ok(())
}

async fn cmd_connection(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let uri = if let Some(branch_name) = branch {
        let branch_id = resolve_branch(client, base, db_id, branch_name).await?;
        let url = format!("{}/{}/branches/{}/connection", base, db_id, branch_id);
        let resp: ConnectionResponse = client
            .get(&url)
            .await
            .context("failed to get branch connection")?;
        resp.connection_uri
    } else {
        let url = format!("{}/{}/connection", base, db_id);
        let resp: ConnectionResponse =
            client.get(&url).await.context("failed to get connection")?;
        resp.connection_uri
    };

    if json {
        output::json_output(&serde_json::json!({"connectionUri": uri}));
    } else {
        println!("{uri}");
    }
    Ok(())
}

async fn cmd_query(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    sql: &str,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let url = format!("{}/{}/query", base, db_id);
    let body = QueryBody {
        sql: sql.to_string(),
        branch_name: branch.map(String::from),
    };
    let result: QueryResult = client.post(&url, &body).await.context("query failed")?;

    if json {
        output::json_output(&result);
        return Ok(());
    }

    // Human-readable table output
    if let Some(ref cols) = result.columns
        && !cols.is_empty()
    {
        eprintln!("  {}", cols.join(" | "));
        eprintln!("  {}", "-".repeat(cols.len() * 12));
    }
    if let Some(ref rows) = result.rows {
        for row in rows {
            if let Some(obj) = row.as_object() {
                let vals: Vec<String> = obj.values().map(format_cell).collect();
                eprintln!("  {}", vals.join(" | "));
            } else {
                eprintln!("  {row}");
            }
        }
        if let Some(count) = result.row_count {
            eprintln!();
            eprintln!("  ({count} row(s))");
        }
    }
    if let Some(dur) = result.duration_ms {
        eprintln!("  Time: {dur:.1}ms");
    }

    Ok(())
}

async fn cmd_branches_list(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let url = format!("{}/{}/branches", base, db_id);
    let list: BranchListResponse = client.get(&url).await.context("failed to list branches")?;

    if json {
        output::json_output(&list);
        return Ok(());
    }

    if list.data.is_empty() {
        eprintln!("  No branches found.");
        return Ok(());
    }

    for b in &list.data {
        let status = b.status.as_deref().unwrap_or("unknown");
        let preview = if b.is_preview_branch == Some(true) {
            " (preview)"
        } else {
            ""
        };
        eprintln!(
            "  {} {}{preview} ({})",
            console::style(&b.id).dim(),
            console::style(&b.name).bold(),
            format_status(status),
        );
    }
    Ok(())
}

async fn cmd_branches_create(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    name: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/{}/branches", base, db_id);
    let body = CreateBranchBody {
        name: name.to_string(),
    };
    let branch: Branch = client
        .post(&url, &body)
        .await
        .context("failed to create branch")?;

    if json {
        output::json_output(&branch);
    } else {
        output::success(
            false,
            format!("Branch created: {} ({})", branch.name, branch.id),
            output::Phase::Db,
        );
    }
    Ok(())
}

async fn cmd_branches_delete(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    branch: &str,
) -> anyhow::Result<()> {
    let branch_id = resolve_branch(client, base, db_id, branch).await?;
    let url = format!("{}/{}/branches/{}", base, db_id, branch_id);
    client
        .delete_empty(&url)
        .await
        .context("failed to delete branch")?;

    if json {
        output::json_output(&serde_json::json!({"deleted": branch_id}));
    } else {
        output::success(false, format!("Branch {branch} deleted"), output::Phase::Db);
    }
    Ok(())
}

async fn cmd_branch_connection(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    branch: &str,
) -> anyhow::Result<()> {
    let branch_id = resolve_branch(client, base, db_id, branch).await?;
    let url = format!("{}/{}/branches/{}/connection", base, db_id, branch_id);
    let resp: ConnectionResponse = client
        .get(&url)
        .await
        .context("failed to get branch connection")?;

    if json {
        output::json_output(&serde_json::json!({"connectionUri": resp.connection_uri}));
    } else {
        println!("{}", resp.connection_uri);
    }
    Ok(())
}

async fn cmd_config(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    args: ConfigArgs,
) -> anyhow::Result<()> {
    let has_updates =
        args.auto_inject.is_some() || args.env_var.is_some() || args.preview_branches.is_some();

    if has_updates {
        let url = format!("{}/{}/auto-inject", base, db_id);
        let enabled = match args.auto_inject {
            Some(enabled) => enabled,
            None => {
                let info: DbInfoResponse = client
                    .get(&format!("{}/{}", base, db_id))
                    .await
                    .context("failed to get current auto-inject settings")?;
                info.db.auto_inject_db_url.unwrap_or(false)
            }
        };
        let body = AutoInjectBody {
            enabled,
            env_var_name: args.env_var,
            auto_create_preview_branch: args.preview_branches,
        };
        let resp: serde_json::Value = client
            .patch(&url, &body)
            .await
            .context("failed to update auto-inject settings")?;

        if json {
            output::json_output(&resp);
        } else {
            output::success(false, "Auto-inject settings updated", output::Phase::Db);
        }
    } else {
        // Show current settings
        let url = format!("{}/{}", base, db_id);
        let info: DbInfoResponse = client
            .get(&url)
            .await
            .context("failed to get database info")?;

        if json {
            output::json_output(&serde_json::json!({
                "autoInject": info.db.auto_inject_db_url,
                "envVar": info.db.env_var_name,
                "previewBranches": info.db.auto_create_preview_branch,
            }));
        } else {
            let enabled = info.db.auto_inject_db_url == Some(true);
            let var = info.db.env_var_name.as_deref().unwrap_or("DATABASE_URL");
            let preview = info.db.auto_create_preview_branch == Some(true);
            eprintln!("  Auto-inject:      {}", if enabled { "on" } else { "off" });
            eprintln!("  Env variable:     {var}");
            eprintln!("  Preview branches: {}", if preview { "on" } else { "off" });
        }
    }
    Ok(())
}

async fn cmd_schema(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    json: bool,
    branch: Option<&str>,
) -> anyhow::Result<()> {
    let url = format!("{}/{}/query", base, db_id);

    // Query table list
    let tables_sql = r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
        ORDER BY table_name
    "#;
    let body = QueryBody {
        sql: tables_sql.to_string(),
        branch_name: branch.map(String::from),
    };
    let tables_result: QueryResult = client
        .post(&url, &body)
        .await
        .context("failed to query schema")?;

    let table_names: Vec<String> = tables_result
        .rows
        .unwrap_or_default()
        .iter()
        .filter_map(|r| {
            r.get("table_name")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect();

    // Query columns for all tables
    let columns_sql = r#"
        SELECT table_name, column_name, data_type, is_nullable, column_default
        FROM information_schema.columns
        WHERE table_schema = 'public'
        ORDER BY table_name, ordinal_position
    "#;
    let body = QueryBody {
        sql: columns_sql.to_string(),
        branch_name: branch.map(String::from),
    };
    let cols_result: QueryResult = client
        .post(&url, &body)
        .await
        .context("failed to query columns")?;

    let mut schema = SchemaOutput { tables: Vec::new() };

    for name in &table_names {
        let columns: Vec<SchemaColumn> = cols_result
            .rows
            .as_ref()
            .unwrap_or(&Vec::new())
            .iter()
            .filter(|r| r.get("table_name").and_then(|v| v.as_str()) == Some(name))
            .map(|r| SchemaColumn {
                name: r
                    .get("column_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                col_type: r
                    .get("data_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                nullable: r.get("is_nullable").and_then(|v| v.as_str()) == Some("YES"),
                default: r
                    .get("column_default")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
            .collect();

        schema.tables.push(SchemaTable {
            name: name.clone(),
            columns,
        });
    }

    if json {
        output::json_output(&schema);
    } else {
        if schema.tables.is_empty() {
            eprintln!("  No tables found in public schema.");
            return Ok(());
        }
        for table in &schema.tables {
            eprintln!(
                "  {} {}",
                console::style("TABLE").dim(),
                console::style(&table.name).bold()
            );
            for col in &table.columns {
                let nullable = if col.nullable { "NULL" } else { "NOT NULL" };
                let default = col
                    .default
                    .as_ref()
                    .map(|d| format!(" DEFAULT {d}"))
                    .unwrap_or_default();
                eprintln!(
                    "    {} {} {}{default}",
                    console::style(&col.name).cyan(),
                    col.col_type,
                    nullable,
                );
            }
            eprintln!();
        }
    }
    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────

/// Fetch dev-env connection for `nrz dev` integration.
pub async fn fetch_dev_env(
    client: &ApiClient,
    project_id: &str,
    database: Option<&str>,
    branch: Option<&str>,
) -> anyhow::Result<DevEnvResponse> {
    let mut url = format!("/v1/managed-databases/{project_id}/dev-env");
    let mut params = Vec::new();
    if let Some(db) = database {
        params.push(format!(
            "database={}",
            percent_encoding::utf8_percent_encode(db, percent_encoding::NON_ALPHANUMERIC)
        ));
    }
    if let Some(br) = branch {
        params.push(format!(
            "branch={}",
            percent_encoding::utf8_percent_encode(br, percent_encoding::NON_ALPHANUMERIC)
        ));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    client.get(&url).await.context(
        "failed to fetch dev environment — is a managed database configured for this project?",
    )
}

async fn resolve_branch(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    branch: &str,
) -> anyhow::Result<String> {
    // If it looks like an ID, use directly
    if looks_like_id(branch) {
        return Ok(branch.to_string());
    }

    let url = format!("{}/{}/branches", base, db_id);
    let list: BranchListResponse = client.get(&url).await.context("failed to list branches")?;

    list.data
        .iter()
        .find(|b| b.id == branch || b.name == branch)
        .map(|b| b.id.clone())
        .ok_or_else(|| anyhow::anyhow!("branch \"{branch}\" not found"))
}

/// Check if a string looks like a platform ID (UUIDv7 / hex ID).
/// Avoids unnecessary list API call when user passes a full ID.
fn looks_like_id(val: &str) -> bool {
    // UUIDv7 with hyphens (36 chars) or without (32 chars), or other long hex-ish IDs
    let len = val.len();
    if len < 24 {
        return false;
    }
    val.chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-' || c == '_')
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

#[cfg(test)]
#[path = "db_handler_tests.rs"]
mod db_handler_tests;
