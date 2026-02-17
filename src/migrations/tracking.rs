//! Migration tracking via `_nrz_migrations` table in local SQLite.

use anyhow::Context;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedMigration {
    pub name: String,
    pub checksum: String,
    pub applied_at: String,
}

pub fn init_tracking_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _nrz_migrations (
            name TEXT PRIMARY KEY,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .context("failed to create _nrz_migrations table")?;
    Ok(())
}

pub fn get_applied(conn: &Connection) -> anyhow::Result<Vec<AppliedMigration>> {
    let mut stmt = conn
        .prepare("SELECT name, checksum, applied_at FROM _nrz_migrations ORDER BY name")
        .context("failed to query _nrz_migrations")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AppliedMigration {
                name: row.get(0)?,
                checksum: row.get(1)?,
                applied_at: row.get(2)?,
            })
        })
        .context("failed to read applied migrations")?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn record_applied(conn: &Connection, name: &str, checksum: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO _nrz_migrations (name, checksum) VALUES (?1, ?2)",
        rusqlite::params![name, checksum],
    )
    .with_context(|| format!("failed to record migration {name}"))?;
    Ok(())
}
