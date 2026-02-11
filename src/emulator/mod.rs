pub mod db;
pub mod kv;
pub mod server;

#[cfg(test)]
mod kv_tests;

use std::path::{Path, PathBuf};

/// Data directory for local emulator state.
///
/// Located at `<project>/.onreza/data/` — should be gitignored.
pub fn data_dir(project_dir: &Path) -> PathBuf {
    project_dir.join(".onreza").join("data")
}

/// Ensure data directory exists.
pub fn ensure_data_dir(project_dir: &Path) -> anyhow::Result<PathBuf> {
    let dir = data_dir(project_dir);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
