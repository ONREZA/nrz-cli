//! D1 migration system — scan, checksum, track, apply.

pub mod tracking;

#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod tracking_tests;

use std::path::Path;

use anyhow::Context;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Migration {
    pub name: String,
    pub sql: String,
    pub checksum: String,
}

/// Scan `migrations/` directory, read `.sql` files, sort by name, compute checksums.
pub fn scan_migrations_dir(dir: &Path) -> anyhow::Result<Vec<Migration>> {
    let migrations_dir = dir.join("migrations");
    if !migrations_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut entries = Vec::new();
    for result in std::fs::read_dir(&migrations_dir)
        .with_context(|| format!("failed to read {}", migrations_dir.display()))?
    {
        let entry = result
            .with_context(|| format!("error reading entry in {}", migrations_dir.display()))?;
        if entry.path().extension().is_some_and(|ext| ext == "sql") {
            entries.push(entry);
        }
    }

    entries.sort_by_key(|e| e.file_name());

    let mut migrations = Vec::new();
    for entry in entries {
        let path = entry.path();
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.is_empty() {
            anyhow::bail!("migration file has no name: {}", path.display());
        }
        let sql = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let checksum = compute_checksum(&sql);
        migrations.push(Migration {
            name,
            sql,
            checksum,
        });
    }

    Ok(migrations)
}

/// SHA256 checksum of SQL content.
pub fn compute_checksum(sql: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sql.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Parse `NNNN_desc.sql` filenames and return max+1, or 1 if empty.
pub fn next_migration_number(dir: &Path) -> anyhow::Result<u32> {
    let migrations_dir = dir.join("migrations");
    if !migrations_dir.is_dir() {
        return Ok(1);
    }

    let mut max = 0u32;
    for entry in std::fs::read_dir(&migrations_dir)
        .with_context(|| format!("failed to read {}", migrations_dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(num_str) = name.split('_').next()
            && let Ok(n) = num_str.parse::<u32>()
        {
            max = max.max(n);
        }
    }

    Ok(max + 1)
}
