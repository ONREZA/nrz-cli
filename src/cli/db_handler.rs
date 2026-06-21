//! CLI handler for `nrz db` subcommands — managed PostgreSQL (kaiki).

use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio_postgres::{
    Client, Row, SimpleQueryMessage,
    config::SslMode,
    types::{ToSql, Type},
};
use tokio_postgres_rustls::MakeRustlsConnect;
use url::Url;

use super::db::{BranchesCommand, ConfigArgs, DbArgs, DbCommand};
use crate::api::ApiClient;
use crate::auth;
use crate::output;
use nrz::config::ProjectConfig;

const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const QUERY_MAX_ROWS: usize = 1000;
const KAIKI_DATABASES_BASE: &str = "/v1/kaiki/databases";

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
    #[serde(default)]
    project_attachments: Vec<ProjectAttachment>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAttachment {
    project_id: String,
    #[serde(default)]
    auto_inject_db_url: Option<bool>,
    #[serde(default)]
    env_var_name: Option<String>,
    #[serde(default)]
    auto_create_preview_branch: Option<bool>,
}

impl ManagedDatabase {
    fn attachment_for_project(&self, project_id: &str) -> Option<&ProjectAttachment> {
        self.project_attachments
            .iter()
            .find(|attachment| attachment.project_id == project_id)
    }

    fn is_attached_to_project(&self, project_id: &str) -> bool {
        self.attachment_for_project(project_id).is_some()
    }

    fn auto_inject_db_url_for_project(&self, project_id: &str) -> Option<bool> {
        self.attachment_for_project(project_id)
            .and_then(|attachment| attachment.auto_inject_db_url)
            .or(self.auto_inject_db_url)
    }

    fn env_var_name_for_project(&self, project_id: &str) -> Option<&str> {
        self.attachment_for_project(project_id)
            .and_then(|attachment| attachment.env_var_name.as_deref())
            .or(self.env_var_name.as_deref())
    }

    fn auto_create_preview_branch_for_project(&self, project_id: &str) -> Option<bool> {
        self.attachment_for_project(project_id)
            .and_then(|attachment| attachment.auto_create_preview_branch)
            .or(self.auto_create_preview_branch)
    }
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<serde_json::Value>>,
    row_count: i64,
    duration_ms: f64,
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
struct CreateBranchBody {
    name: String,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAttachmentBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    env_var_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_inject_db_url: Option<bool>,
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
    let base = KAIKI_DATABASES_BASE;

    match args.command {
        DbCommand::List => cmd_list(&client, base, &project_id, json).await,
        DbCommand::Create {
            name,
            cu_size,
            wait,
        } => cmd_create(&client, base, &project_id, json, name, cu_size, wait).await,
        DbCommand::Info { database } => {
            let db_id = resolve_db(&client, base, &project_id, database.as_deref(), config).await?;
            cmd_info(&client, base, &project_id, &db_id, json).await
        }
        DbCommand::Delete { database, force } => {
            let db_id = resolve_db(&client, base, &project_id, Some(&database), config).await?;
            cmd_delete(&client, base, &db_id, json, force).await
        }
        DbCommand::Start { database } => {
            let db_id = resolve_db(&client, base, &project_id, database.as_deref(), config).await?;
            cmd_start_stop(&client, base, &db_id, json, "start").await
        }
        DbCommand::Stop { database } => {
            let db_id = resolve_db(&client, base, &project_id, database.as_deref(), config).await?;
            cmd_start_stop(&client, base, &db_id, json, "stop").await
        }
        DbCommand::Connection { database, branch } => {
            let db_id = resolve_db(&client, base, &project_id, database.as_deref(), config).await?;
            cmd_connection(&client, base, &db_id, json, branch.as_deref()).await
        }
        DbCommand::Query {
            database,
            sql,
            file,
            branch,
        } => {
            let db_id = resolve_db(&client, base, &project_id, database.as_deref(), config).await?;
            let sql = resolve_sql(sql.as_deref(), file.as_deref())?;
            cmd_query(&client, base, &db_id, json, &sql, branch.as_deref()).await
        }
        DbCommand::Branches(bargs) => {
            let db_id = resolve_db(
                &client,
                base,
                &project_id,
                bargs.database.as_deref(),
                config,
            )
            .await?;
            match bargs.command {
                None | Some(BranchesCommand::List) => {
                    cmd_branches_list(&client, base, &db_id, json).await
                }
                Some(BranchesCommand::Create { name }) => {
                    cmd_branches_create(&client, base, &db_id, json, &name).await
                }
                Some(BranchesCommand::Delete { branch }) => {
                    cmd_branches_delete(&client, base, &db_id, json, &branch).await
                }
                Some(BranchesCommand::Connection { branch }) => {
                    cmd_branch_connection(&client, base, &db_id, json, &branch).await
                }
            }
        }
        DbCommand::Config(cargs) => {
            let db_id = resolve_db(
                &client,
                base,
                &project_id,
                cargs.database.as_deref(),
                config,
            )
            .await?;
            cmd_config(&client, base, &project_id, &db_id, json, cargs).await
        }
        DbCommand::Schema { database, branch } => {
            let db_id = resolve_db(&client, base, &project_id, database.as_deref(), config).await?;
            cmd_schema(&client, base, &db_id, json, branch.as_deref()).await
        }
    }
}

// ── Database resolution ─────────────────────────────────────

/// Resolve database ID from explicit arg, config, or by listing databases.
async fn resolve_db(
    client: &ApiClient,
    base: &str,
    project_id: &str,
    explicit: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<String> {
    // 1. Explicit argument
    if let Some(val) = explicit
        && !val.is_empty()
    {
        return resolve_db_by_id_or_name(client, base, project_id, val).await;
    }

    // 2. Config: [db] database
    if let Some(val) = config.db_database() {
        return resolve_db_by_id_or_name(client, base, project_id, val).await;
    }

    // 3. Auto-resolve: first project auto-inject attachment, or first project attachment.
    let list: ListResponse = client.get(base).await.context("failed to list databases")?;

    let db = list
        .data
        .iter()
        .filter(|db| db.is_attached_to_project(project_id))
        .find(|db| db.auto_inject_db_url_for_project(project_id) == Some(true))
        .or_else(|| {
            list.data
                .iter()
                .find(|db| db.is_attached_to_project(project_id))
        })
        .ok_or_else(|| anyhow::anyhow!("no databases found in project"))?;

    Ok(db.id.clone())
}

async fn resolve_db_by_id_or_name(
    client: &ApiClient,
    base: &str,
    project_id: &str,
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
        .filter(|db| db.is_attached_to_project(project_id))
        .find(|d| d.id == val || d.db_name.as_deref() == Some(val))
        .or_else(|| {
            list.data
                .iter()
                .find(|d| d.id == val || d.db_name.as_deref() == Some(val))
        })
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

async fn cmd_list(
    client: &ApiClient,
    base: &str,
    project_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let mut list: ListResponse = client.get(base).await.context("failed to list databases")?;
    list.data.retain(|db| db.is_attached_to_project(project_id));

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
        let inject = if db.auto_inject_db_url_for_project(project_id) == Some(true) {
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
    project_id: &str,
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

    let _: serde_json::Value = client
        .patch(
            &project_attachment_url(base, &created.id, project_id),
            &ProjectAttachmentBody::default(),
        )
        .await
        .with_context(|| {
            format!(
                "database {} was created, but failed to attach it to project {}",
                created.id, project_id
            )
        })?;

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

async fn cmd_info(
    client: &ApiClient,
    base: &str,
    project_id: &str,
    db_id: &str,
    json: bool,
) -> anyhow::Result<()> {
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
    if let Some(true) = db.auto_inject_db_url_for_project(project_id) {
        let var = db
            .env_var_name_for_project(project_id)
            .unwrap_or("DATABASE_URL");
        eprintln!("  Auto-inject: {var}");
        if db.auto_create_preview_branch_for_project(project_id) == Some(true) {
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
    let uri = fetch_connection_uri(client, base, db_id, branch).await?;

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
    let connection_uri = fetch_connection_uri(client, base, db_id, branch).await?;
    let result = execute_sql_locally(&connection_uri, sql)
        .await
        .context("query failed")?;

    if json {
        output::json_output(&result);
        return Ok(());
    }

    // Human-readable table output
    if !result.columns.is_empty() {
        eprintln!("  {}", result.columns.join(" | "));
        eprintln!("  {}", "-".repeat(result.columns.len() * 12));
    }
    for row in &result.rows {
        let vals: Vec<String> = row.iter().map(format_cell).collect();
        eprintln!("  {}", vals.join(" | "));
    }
    eprintln!();
    eprintln!("  ({} row(s))", result.row_count);
    eprintln!("  Time: {:.1}ms", result.duration_ms);

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
    project_id: &str,
    db_id: &str,
    json: bool,
    args: ConfigArgs,
) -> anyhow::Result<()> {
    let has_updates =
        args.auto_inject.is_some() || args.env_var.is_some() || args.preview_branches.is_some();

    if has_updates {
        let body = ProjectAttachmentBody {
            env_var_name: args.env_var,
            auto_inject_db_url: args.auto_inject,
            auto_create_preview_branch: args.preview_branches,
        };
        let resp: serde_json::Value = client
            .patch(&project_attachment_url(base, db_id, project_id), &body)
            .await
            .context("failed to update project database attachment")?;

        if json {
            output::json_output(&resp);
        } else {
            output::success(
                false,
                "Database project settings updated",
                output::Phase::Db,
            );
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
                "autoInject": info.db.auto_inject_db_url_for_project(project_id),
                "envVar": info.db.env_var_name_for_project(project_id),
                "previewBranches": info.db.auto_create_preview_branch_for_project(project_id),
            }));
        } else {
            let enabled = info.db.auto_inject_db_url_for_project(project_id) == Some(true);
            let var = info
                .db
                .env_var_name_for_project(project_id)
                .unwrap_or("DATABASE_URL");
            let preview = info.db.auto_create_preview_branch_for_project(project_id) == Some(true);
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
    let connection_uri = fetch_connection_uri(client, base, db_id, branch).await?;

    // Query table list
    let tables_sql = r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
        ORDER BY table_name
    "#;
    let tables_result = execute_sql_locally(&connection_uri, tables_sql)
        .await
        .context("failed to query schema")?;

    let table_names: Vec<String> = tables_result
        .rows
        .iter()
        .filter_map(|r| value_as_str(r, 0).map(String::from))
        .collect();

    // Query columns for all tables
    let columns_sql = r#"
        SELECT table_name, column_name, data_type, is_nullable, column_default
        FROM information_schema.columns
        WHERE table_schema = 'public'
        ORDER BY table_name, ordinal_position
    "#;
    let cols_result = execute_sql_locally(&connection_uri, columns_sql)
        .await
        .context("failed to query columns")?;

    let mut schema = SchemaOutput { tables: Vec::new() };

    for name in &table_names {
        let columns: Vec<SchemaColumn> = cols_result
            .rows
            .iter()
            .filter(|r| value_as_str(r, 0) == Some(name.as_str()))
            .map(|r| SchemaColumn {
                name: value_as_str(r, 1).unwrap_or("").to_string(),
                col_type: value_as_str(r, 2).unwrap_or("").to_string(),
                nullable: value_as_str(r, 3) == Some("YES"),
                default: value_as_str(r, 4).map(String::from),
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

fn project_attachment_url(base: &str, db_id: &str, project_id: &str) -> String {
    format!("{base}/{db_id}/attachments/{project_id}")
}

fn query_value(value: &str) -> String {
    percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn dev_env_url(project_id: &str, database: Option<&str>, branch: Option<&str>) -> String {
    let mut params = vec![format!("projectId={}", query_value(project_id))];
    if let Some(database) = database {
        params.push(format!("database={}", query_value(database)));
    }
    if let Some(branch) = branch {
        params.push(format!("branch={}", query_value(branch)));
    }
    format!("{KAIKI_DATABASES_BASE}/dev-env?{}", params.join("&"))
}

/// Fetch dev-env connection for `nrz dev` integration.
pub async fn fetch_dev_env(
    client: &ApiClient,
    project_id: &str,
    database: Option<&str>,
    branch: Option<&str>,
) -> anyhow::Result<DevEnvResponse> {
    let url = dev_env_url(project_id, database, branch);

    client.get(&url).await.context(
        "failed to fetch dev environment — is a managed database configured for this project?",
    )
}

async fn fetch_connection_uri(
    client: &ApiClient,
    base: &str,
    db_id: &str,
    branch: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(branch_name) = branch {
        let branch_id = resolve_branch(client, base, db_id, branch_name).await?;
        let url = format!("{}/{}/branches/{}/connection", base, db_id, branch_id);
        let resp: ConnectionResponse = client
            .get(&url)
            .await
            .context("failed to get branch connection")?;
        return Ok(resp.connection_uri);
    }

    let url = format!("{}/{}/connection", base, db_id);
    let resp: ConnectionResponse = client.get(&url).await.context("failed to get connection")?;
    Ok(resp.connection_uri)
}

async fn execute_sql_locally(connection_uri: &str, sql: &str) -> anyhow::Result<QueryResult> {
    let config = postgres_config_from_uri(connection_uri)?;
    let tls = make_tls_connector()?;
    let start = Instant::now();
    let (client, connection_task) = connect_postgres(config, tls).await?;
    let result = match sql_execution_mode(sql) {
        SqlExecutionMode::RowCapable => execute_typed_query(&client, sql, start).await,
        SqlExecutionMode::SimpleCommand => execute_simple_query(&client, sql, start).await,
    };
    drop(client);
    connection_task.abort();
    let _ = connection_task.await;

    result
}

async fn connect_postgres(
    config: tokio_postgres::Config,
    tls: MakeRustlsConnect,
) -> anyhow::Result<(Client, JoinHandle<()>)> {
    let (client, connection) = tokio::time::timeout(QUERY_TIMEOUT, config.connect(tls))
        .await
        .map_err(|_| connection_timeout_error())?
        .context("failed to connect to database")?;
    let connection_task = tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::warn!("database connection error: {error}");
        }
    });

    Ok((client, connection_task))
}

fn make_tls_connector() -> anyhow::Result<MakeRustlsConnect> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let (connector, cert_errors) = MakeRustlsConnect::with_native_certs()
        .map_err(|errors| anyhow::anyhow!("failed to load native TLS roots: {errors:?}"))?;
    if !cert_errors.is_empty() {
        tracing::warn!("some native TLS roots failed to load: {cert_errors:?}");
    }
    Ok(connector)
}

fn postgres_config_from_uri(uri: &str) -> anyhow::Result<tokio_postgres::Config> {
    let url = Url::parse(uri).context("invalid PostgreSQL connection URI")?;
    match url.scheme() {
        "postgres" | "postgresql" => {}
        scheme => bail!("unsupported PostgreSQL connection URI scheme: {scheme}"),
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL connection URI is missing host"))?;
    let user = decode_uri_component(url.username(), "username")?;
    if user.is_empty() {
        bail!("PostgreSQL connection URI is missing username");
    }
    let database = decode_uri_component(url.path().trim_start_matches('/'), "database")?;
    if database.is_empty() {
        bail!("PostgreSQL connection URI is missing database name");
    }

    let mut config = tokio_postgres::Config::new();
    config
        .host(host)
        .port(url.port().unwrap_or(5432))
        .user(user)
        .dbname(database)
        .ssl_mode(SslMode::Require)
        .connect_timeout(QUERY_TIMEOUT);
    if let Some(password) = url.password() {
        config.password(decode_uri_component(password, "password")?);
    }

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "application_name" => {
                config.application_name(value.into_owned());
            }
            "connect_timeout" => {
                let seconds = value
                    .parse::<u64>()
                    .with_context(|| format!("invalid PostgreSQL connect_timeout: {value}"))?;
                config.connect_timeout(Duration::from_secs(seconds));
            }
            "options" => {
                config.options(value.into_owned());
            }
            "sslmode" => validate_sslmode(&value)?,
            _ => {}
        }
    }

    Ok(config)
}

fn validate_sslmode(value: &str) -> anyhow::Result<()> {
    match value {
        "disable" | "allow" | "prefer" | "require" | "verify-ca" | "verify-full" => Ok(()),
        other => bail!("unsupported PostgreSQL sslmode: {other}"),
    }
}

fn decode_uri_component(value: &str, label: &str) -> anyhow::Result<String> {
    percent_encoding::percent_decode_str(value)
        .decode_utf8()
        .map(std::borrow::Cow::into_owned)
        .with_context(|| format!("invalid percent-encoding in PostgreSQL URI {label}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SqlExecutionMode {
    RowCapable,
    SimpleCommand,
}

fn sql_execution_mode(sql: &str) -> SqlExecutionMode {
    match first_sql_keyword(sql).as_deref() {
        Some(
            "delete" | "explain" | "insert" | "merge" | "select" | "show" | "table" | "update"
            | "values" | "with",
        ) => SqlExecutionMode::RowCapable,
        _ => SqlExecutionMode::SimpleCommand,
    }
}

fn first_sql_keyword(sql: &str) -> Option<String> {
    let mut rest = sql.trim_start();
    loop {
        if let Some(after_comment) = rest.strip_prefix("--") {
            rest = after_comment
                .split_once('\n')
                .map(|(_, tail)| tail.trim_start())
                .unwrap_or("");
            continue;
        }
        if let Some(after_comment) = rest.strip_prefix("/*") {
            rest = after_comment
                .split_once("*/")
                .map(|(_, tail)| tail.trim_start())
                .unwrap_or("");
            continue;
        }
        break;
    }

    let keyword: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    if keyword.is_empty() {
        None
    } else {
        Some(keyword)
    }
}

async fn execute_typed_query(
    client: &Client,
    sql: &str,
    start: Instant,
) -> anyhow::Result<QueryResult> {
    let statement = match tokio::time::timeout(QUERY_TIMEOUT, client.prepare(sql)).await {
        Ok(Ok(statement)) => statement,
        Ok(Err(_)) => return execute_simple_query(client, sql, start).await,
        Err(_) => return Err(query_timeout_error()),
    };

    if statement.columns().is_empty() {
        return execute_simple_query(client, sql, start).await;
    }

    if !statement
        .columns()
        .iter()
        .all(|column| supports_typed_json(column.type_()))
    {
        return execute_simple_query(client, sql, start).await;
    }

    let params = std::iter::empty::<&(dyn ToSql + Sync)>();
    let stream = tokio::time::timeout(QUERY_TIMEOUT, client.query_raw(&statement, params))
        .await
        .map_err(|_| query_timeout_error())?
        .context("failed to execute SQL")?;
    match tokio::time::timeout(
        QUERY_TIMEOUT,
        typed_query_result_from_stream(stream, start, statement.columns()),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(query_timeout_error()),
    }
}

async fn execute_simple_query(
    client: &Client,
    sql: &str,
    start: Instant,
) -> anyhow::Result<QueryResult> {
    let stream = tokio::time::timeout(QUERY_TIMEOUT, client.simple_query_raw(sql))
        .await
        .map_err(|_| query_timeout_error())?
        .context("failed to execute SQL")?;
    match tokio::time::timeout(
        QUERY_TIMEOUT,
        simple_query_result_from_stream(stream, start),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(query_timeout_error()),
    }
}

async fn typed_query_result_from_stream(
    stream: tokio_postgres::RowStream,
    start: Instant,
    columns: &[tokio_postgres::Column],
) -> anyhow::Result<QueryResult> {
    let columns = columns.iter().map(|col| col.name().to_string()).collect();
    let mut rows = Vec::new();
    let mut selected_row_count = 0_i64;

    futures::pin_mut!(stream);
    while let Some(row) = stream.next().await {
        let row = row.context("failed to execute SQL")?;
        selected_row_count = selected_row_count.saturating_add(1);
        if rows.len() < QUERY_MAX_ROWS {
            rows.push(row_to_json_values(&row)?);
        }
    }

    Ok(QueryResult {
        columns,
        rows,
        row_count: selected_row_count,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

fn supports_typed_json(ty: &Type) -> bool {
    ty == &Type::BOOL
        || ty == &Type::INT2
        || ty == &Type::INT4
        || ty == &Type::INT8
        || ty == &Type::FLOAT4
        || ty == &Type::FLOAT8
        || ty == &Type::TEXT
        || ty == &Type::VARCHAR
        || ty == &Type::BPCHAR
        || ty == &Type::NAME
        || ty == &Type::UNKNOWN
        || ty == &Type::JSON
        || ty == &Type::JSONB
        || ty == &Type::UUID
        || ty == &Type::OID
}

fn row_to_json_values(row: &Row) -> anyhow::Result<Vec<serde_json::Value>> {
    (0..row.len())
        .map(|idx| cell_to_json_value(row, idx))
        .collect()
}

fn cell_to_json_value(row: &Row, idx: usize) -> anyhow::Result<serde_json::Value> {
    let ty = row.columns()[idx].type_();
    if ty == &Type::BOOL {
        let value = row.try_get::<usize, Option<bool>>(idx)?;
        return Ok(optional_json(value, serde_json::Value::Bool));
    }
    if ty == &Type::INT2 {
        let value = row.try_get::<usize, Option<i16>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(i64::from(value).into())
        }));
    }
    if ty == &Type::INT4 {
        let value = row.try_get::<usize, Option<i32>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(i64::from(value).into())
        }));
    }
    if ty == &Type::INT8 {
        let value = row.try_get::<usize, Option<i64>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(value.into())
        }));
    }
    if ty == &Type::OID {
        let value = row.try_get::<usize, Option<u32>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::Number(u64::from(value).into())
        }));
    }
    if ty == &Type::FLOAT4 {
        let value = row.try_get::<usize, Option<f32>>(idx)?;
        return Ok(optional_json(value, |value| {
            json_number_from_f64(value.into())
        }));
    }
    if ty == &Type::FLOAT8 {
        let value = row.try_get::<usize, Option<f64>>(idx)?;
        return Ok(optional_json(value, json_number_from_f64));
    }
    if ty == &Type::JSON || ty == &Type::JSONB {
        let value = row.try_get::<usize, Option<serde_json::Value>>(idx)?;
        return Ok(value.unwrap_or(serde_json::Value::Null));
    }
    if ty == &Type::UUID {
        let value = row.try_get::<usize, Option<uuid::Uuid>>(idx)?;
        return Ok(optional_json(value, |value| {
            serde_json::Value::String(value.to_string())
        }));
    }
    if is_text_type(ty) {
        let value = row.try_get::<usize, Option<String>>(idx)?;
        return Ok(optional_json(value, serde_json::Value::String));
    }

    bail!("unsupported PostgreSQL result type: {}", ty.name());
}

fn optional_json<T>(
    value: Option<T>,
    convert: impl FnOnce(T) -> serde_json::Value,
) -> serde_json::Value {
    value.map(convert).unwrap_or(serde_json::Value::Null)
}

fn json_number_from_f64(value: f64) -> serde_json::Value {
    serde_json::Number::from_f64(value)
        .map(serde_json::Value::Number)
        .unwrap_or(serde_json::Value::Null)
}

fn is_text_type(ty: &Type) -> bool {
    ty == &Type::TEXT
        || ty == &Type::VARCHAR
        || ty == &Type::BPCHAR
        || ty == &Type::NAME
        || ty == &Type::UNKNOWN
}

async fn simple_query_result_from_stream(
    stream: tokio_postgres::SimpleQueryStream,
    start: Instant,
) -> anyhow::Result<QueryResult> {
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut command_row_count = 0_i64;
    let mut selected_row_count = 0_i64;

    futures::pin_mut!(stream);
    while let Some(message) = stream.next().await {
        match message {
            Err(error) => return Err(error).context("failed to execute SQL"),
            Ok(message) => apply_query_message(
                message,
                &mut columns,
                &mut rows,
                &mut command_row_count,
                &mut selected_row_count,
            ),
        }
    }

    let row_count = if selected_row_count > 0 {
        selected_row_count
    } else {
        command_row_count
    };

    Ok(QueryResult {
        columns,
        rows,
        row_count,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

fn apply_query_message(
    message: SimpleQueryMessage,
    columns: &mut Vec<String>,
    rows: &mut Vec<Vec<serde_json::Value>>,
    command_row_count: &mut i64,
    selected_row_count: &mut i64,
) {
    match message {
        SimpleQueryMessage::RowDescription(desc) if rows.is_empty() => {
            *columns = desc.iter().map(|col| col.name().to_string()).collect();
        }
        SimpleQueryMessage::Row(row) => {
            *selected_row_count = selected_row_count.saturating_add(1);
            if rows.len() < QUERY_MAX_ROWS {
                if columns.is_empty() {
                    *columns = row
                        .columns()
                        .iter()
                        .map(|col| col.name().to_string())
                        .collect();
                }
                let values = (0..row.len())
                    .map(|idx| match row.get(idx) {
                        Some(value) => serde_json::Value::String(value.to_string()),
                        None => serde_json::Value::Null,
                    })
                    .collect();
                rows.push(values);
            }
        }
        SimpleQueryMessage::CommandComplete(count) => {
            *command_row_count = i64::try_from(count).unwrap_or(i64::MAX);
        }
        SimpleQueryMessage::RowDescription(_) => {}
        _ => {}
    }
}

fn query_timeout_error() -> anyhow::Error {
    anyhow::anyhow!("query timed out after {}s", QUERY_TIMEOUT.as_secs())
}

fn connection_timeout_error() -> anyhow::Error {
    anyhow::anyhow!(
        "database connection timed out after {}s",
        QUERY_TIMEOUT.as_secs()
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

fn value_as_str(row: &[serde_json::Value], idx: usize) -> Option<&str> {
    row.get(idx).and_then(|v| v.as_str())
}

#[cfg(test)]
#[path = "db_handler_tests.rs"]
mod db_handler_tests;
