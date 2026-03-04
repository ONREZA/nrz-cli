//! CLI handler for `nrz db migrate` and `nrz db push` subcommands.

use std::io::{IsTerminal, Read as _};
use std::path::Path;

use anyhow::{Context, bail};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use nrz::config::ProjectConfig;

use crate::api::ApiClient;
use crate::auth;
use crate::link::environment_ref;
use crate::migrations::{self, tracking};
use crate::output;
use nrz::config;

use super::db::DbMigrateCommand;

// ── JSON output structs ─────────────────────────────────────

#[derive(Serialize)]
struct CreateOutput {
    path: String,
}

#[derive(Serialize)]
struct ApplyOutput {
    applied: Vec<String>,
    #[serde(rename = "alreadyApplied")]
    already_applied: Vec<String>,
}

#[derive(Serialize)]
struct StatusOutput {
    applied: Vec<tracking::AppliedMigration>,
    pending: Vec<String>,
}

#[derive(Serialize)]
struct PushOutput {
    count: u64,
    duration: String,
    #[serde(rename = "sizeAfter")]
    size_after: u64,
}

#[derive(Serialize)]
struct DryRunOutput {
    migrations: Vec<DryRunEntry>,
}

#[derive(Serialize)]
struct DryRunEntry {
    name: String,
    sql: String,
}

// ── Remote API response structs ─────────────────────────────

#[derive(Deserialize)]
struct RemoteApplyResponse {
    applied: Vec<String>,
    #[serde(default, rename = "alreadyApplied")]
    already_applied: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteMigrationEntry {
    name: String,
    checksum: String,
    applied_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteExecResponse {
    count: u64,
    duration: String,
    size_after: u64,
}

// ── Migrate handler ─────────────────────────────────────────

pub async fn handle_migrate(
    command: DbMigrateCommand,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let project_dir = Path::new(".")
        .canonicalize()
        .context("failed to resolve current directory")?;
    let mig_dir = config.migrations_dir();

    match command {
        DbMigrateCommand::Create { name } => create(&project_dir, &name, json, mig_dir),
        DbMigrateCommand::Apply {
            remote,
            dry_run,
            project_id,
        } => {
            if remote {
                apply_remote(
                    &project_dir,
                    dry_run,
                    project_id.as_deref(),
                    json,
                    token,
                    workspace,
                    env,
                    config,
                )
                .await
            } else {
                apply_local(&project_dir, dry_run, json, config)
            }
        }
        DbMigrateCommand::Status { remote, project_id } => {
            if remote {
                status_remote(
                    &project_dir,
                    project_id.as_deref(),
                    json,
                    token,
                    workspace,
                    env,
                    config,
                )
                .await
            } else {
                status_local(&project_dir, json, config)
            }
        }
    }
}

// ── Push handler ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn handle_push(
    sql: Option<String>,
    file: Option<String>,
    project_id: Option<String>,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let sql = resolve_sql(sql, file)?;

    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let pid = config::resolve_project_id(project_id.as_deref(), config)?;
    let (eid, _) = environment_ref::resolve_environment_id(env, &pid, &client, json).await?;

    output::status(
        json,
        "~",
        "Pushing SQL to remote database...",
        output::Phase::Db,
    );

    let resp: RemoteExecResponse = client
        .post(
            &format!("/api/d1/databases/{pid}/exec?environmentId={eid}"),
            &serde_json::json!({ "sql": sql }),
        )
        .await
        .context("failed to execute remote SQL")?;

    if json {
        output::json_output(&PushOutput {
            count: resp.count,
            duration: resp.duration,
            size_after: resp.size_after,
        });
    } else {
        output::success(
            false,
            format!(
                "Executed on remote ({} change(s), {})",
                resp.count, resp.duration
            ),
            output::Phase::Db,
        );
    }

    Ok(())
}

// ── Create ──────────────────────────────────────────────────

fn create(project_dir: &Path, name: &str, json: bool, mig_dir: &str) -> anyhow::Result<()> {
    let num = migrations::next_migration_number(project_dir, mig_dir)?;
    let filename = format!("{num:04}_{name}.sql");

    let migrations_dir = project_dir.join(mig_dir);
    std::fs::create_dir_all(&migrations_dir)
        .with_context(|| format!("failed to create {}", migrations_dir.display()))?;

    let path = migrations_dir.join(&filename);
    std::fs::write(
        &path,
        format!("-- Migration: {name}\n-- Created: {}\n\n", timestamp_now()),
    )
    .with_context(|| format!("failed to write {}", path.display()))?;

    if json {
        output::json_output(&CreateOutput {
            path: format!("{mig_dir}/{filename}"),
        });
    } else {
        output::success(
            false,
            format!("Created {mig_dir}/{filename}"),
            output::Phase::Db,
        );
    }

    Ok(())
}

// ── Apply (local) ───────────────────────────────────────────

fn apply_local(
    project_dir: &Path,
    dry_run: bool,
    json: bool,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let mig_dir = config.migrations_dir();
    let all_migrations = migrations::scan_migrations_dir(project_dir, mig_dir)?;
    if all_migrations.is_empty() {
        if json {
            output::json_output(&ApplyOutput {
                applied: vec![],
                already_applied: vec![],
            });
        } else {
            eprintln!("  No migrations found in {mig_dir}/");
        }
        return Ok(());
    }

    let data = config.data_dir_path(project_dir);
    if !data.exists() {
        std::fs::create_dir_all(&data)
            .with_context(|| format!("failed to create {}", data.display()))?;
    }
    let db_path = data.join(config.db_name());
    let mut conn = Connection::open(&db_path)
        .with_context(|| format!("failed to open {}", db_path.display()))?;

    tracking::init_tracking_table(&conn)?;
    let applied = tracking::get_applied(&conn)?;

    // Detect checksum drift — modified migrations that were already applied
    for m in &all_migrations {
        if let Some(a) = applied.iter().find(|a| a.name == m.name)
            && a.checksum != m.checksum
        {
            bail!(
                "migration {} has been modified after being applied \
                 (expected checksum {}, got {}). \
                 Do not modify applied migrations — create a new one instead, \
                 or use `nrz db reset --force` to start fresh.",
                m.name,
                a.checksum,
                m.checksum
            );
        }
    }

    let applied_names: std::collections::HashSet<_> = applied.iter().map(|a| &a.name).collect();

    let pending: Vec<_> = all_migrations
        .iter()
        .filter(|m| !applied_names.contains(&m.name))
        .collect();

    if dry_run {
        if json {
            output::json_output(&DryRunOutput {
                migrations: pending
                    .iter()
                    .map(|m| DryRunEntry {
                        name: m.name.clone(),
                        sql: m.sql.clone(),
                    })
                    .collect(),
            });
        } else if pending.is_empty() {
            eprintln!("  No pending migrations");
        } else {
            eprintln!("  Dry run — {} pending migration(s):", pending.len());
            for m in &pending {
                eprintln!();
                eprintln!("  -- {}", m.name);
                for line in m.sql.lines().take(10) {
                    eprintln!("  {line}");
                }
            }
        }
        return Ok(());
    }

    let mut applied_names_list = Vec::new();
    let already_applied: Vec<_> = applied.iter().map(|a| a.name.clone()).collect();

    for m in &pending {
        let tx = conn.transaction()?;
        tx.execute_batch(&m.sql)
            .with_context(|| format!("migration {} failed", m.name))?;
        tracking::record_applied(&tx, &m.name, &m.checksum)?;
        tx.commit()?;
        applied_names_list.push(m.name.clone());

        if !json {
            output::success(false, format!("Applied {}", m.name), output::Phase::Db);
        }
    }

    if json {
        output::json_output(&ApplyOutput {
            applied: applied_names_list,
            already_applied,
        });
    } else if pending.is_empty() {
        eprintln!("  All migrations already applied");
    }

    Ok(())
}

// ── Apply (remote) ──────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn apply_remote(
    project_dir: &Path,
    dry_run: bool,
    project_id: Option<&str>,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let mig_dir = config.migrations_dir();
    let all_migrations = migrations::scan_migrations_dir(project_dir, mig_dir)?;
    if all_migrations.is_empty() {
        if json {
            output::json_output(&ApplyOutput {
                applied: vec![],
                already_applied: vec![],
            });
        } else {
            eprintln!("  No migrations found in {mig_dir}/");
        }
        return Ok(());
    }

    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let pid = config::resolve_project_id(project_id, config)?;
    let (eid, _) = environment_ref::resolve_environment_id(env, &pid, &client, json).await?;

    output::status(
        json,
        "~",
        "Applying migrations to remote database...",
        output::Phase::Db,
    );

    let resp: RemoteApplyResponse = client
        .post(
            &format!("/api/d1/databases/{pid}/migrations/apply?environmentId={eid}"),
            &serde_json::json!({
                "migrations": all_migrations,
                "dryRun": dry_run,
            }),
        )
        .await
        .context("failed to apply remote migrations")?;

    if json {
        output::json_output(&ApplyOutput {
            applied: resp.applied,
            already_applied: resp.already_applied,
        });
    } else if dry_run {
        eprintln!(
            "  Dry run: {} would be applied, {} already applied",
            resp.applied.len(),
            resp.already_applied.len()
        );
    } else {
        for name in &resp.applied {
            output::success(false, format!("Applied {name} (remote)"), output::Phase::Db);
        }
        if resp.applied.is_empty() {
            eprintln!("  All migrations already applied on remote");
        }
    }

    Ok(())
}

// ── Status (local) ──────────────────────────────────────────

fn status_local(project_dir: &Path, json: bool, config: &ProjectConfig) -> anyhow::Result<()> {
    let all_migrations = migrations::scan_migrations_dir(project_dir, config.migrations_dir())?;

    let data = config.data_dir_path(project_dir);
    let db_path = data.join(config.db_name());

    let applied = if db_path.exists() {
        let conn = Connection::open(&db_path)?;
        tracking::init_tracking_table(&conn)?;
        tracking::get_applied(&conn)?
    } else {
        vec![]
    };

    let applied_names: std::collections::HashSet<_> = applied.iter().map(|a| &a.name).collect();
    let pending: Vec<_> = all_migrations
        .iter()
        .filter(|m| !applied_names.contains(&m.name))
        .map(|m| m.name.clone())
        .collect();

    if json {
        output::json_output(&StatusOutput {
            applied: applied.clone(),
            pending,
        });
    } else {
        if applied.is_empty() && pending.is_empty() {
            eprintln!("  No migrations found");
            return Ok(());
        }

        if !applied.is_empty() {
            eprintln!("  {} Applied:", console::style("✓").green());
            for a in &applied {
                eprintln!("    {} ({})", a.name, a.applied_at);
            }
        }

        if !pending.is_empty() {
            eprintln!("  {} Pending:", console::style("○").yellow());
            for name in &pending {
                eprintln!("    {name}");
            }
        }
    }

    Ok(())
}

// ── Status (remote) ─────────────────────────────────────────

async fn status_remote(
    project_dir: &Path,
    project_id: Option<&str>,
    json: bool,
    token: Option<&str>,
    workspace: Option<&str>,
    env: Option<&str>,
    config: &ProjectConfig,
) -> anyhow::Result<()> {
    let tok = auth::resolve_token(token, workspace)?;
    let client = ApiClient::authenticated(&tok)?;
    let pid = config::resolve_project_id(project_id, config)?;
    let (eid, _) = environment_ref::resolve_environment_id(env, &pid, &client, json).await?;

    let remote_applied: Vec<RemoteMigrationEntry> = client
        .get(&format!(
            "/api/d1/databases/{pid}/migrations?environmentId={eid}"
        ))
        .await
        .context("failed to fetch remote migrations")?;

    // Compare with local migration files to find pending
    let all_local = migrations::scan_migrations_dir(project_dir, config.migrations_dir())
        .context("failed to scan local migrations")?;
    let remote_names: std::collections::HashSet<_> =
        remote_applied.iter().map(|m| m.name.as_str()).collect();
    let pending: Vec<_> = all_local
        .iter()
        .filter(|m| !remote_names.contains(m.name.as_str()))
        .map(|m| m.name.clone())
        .collect();

    if json {
        output::json_output(&StatusOutput {
            applied: remote_applied
                .iter()
                .map(|m| tracking::AppliedMigration {
                    name: m.name.clone(),
                    checksum: m.checksum.clone(),
                    applied_at: m.applied_at.clone(),
                })
                .collect(),
            pending,
        });
    } else {
        if remote_applied.is_empty() && pending.is_empty() {
            eprintln!("  No migrations found");
            return Ok(());
        }

        if !remote_applied.is_empty() {
            eprintln!("  {} Applied (remote):", console::style("✓").green());
            for m in &remote_applied {
                eprintln!("    {} ({})", m.name, m.applied_at);
            }
        }

        if !pending.is_empty() {
            eprintln!("  {} Pending:", console::style("○").yellow());
            for name in &pending {
                eprintln!("    {name}");
            }
        }
    }

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────

pub(crate) fn resolve_sql(sql: Option<String>, file: Option<String>) -> anyhow::Result<String> {
    let sql = if let Some(path) = file {
        std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?
    } else if sql.as_deref() == Some("-") || (sql.is_none() && !std::io::stdin().is_terminal()) {
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

    let sql = sql.trim().to_string();
    if sql.is_empty() {
        bail!("SQL input is empty");
    }
    Ok(sql)
}

fn timestamp_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Manual UTC formatting without chrono dependency
    let s = secs;
    let days = s / 86400;
    let time_of_day = s % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    // Days since 1970-01-01 to Y-M-D
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970;
    loop {
        let year_days = if is_leap(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
