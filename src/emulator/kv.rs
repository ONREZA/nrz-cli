use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::data_dir;

struct Entry {
    value: String,
    expires_at: Option<Instant>,
}

/// In-memory KV store with TTL support.
///
/// Matches the ONREZA.kv API contract from BUILD_OUTPUT_SPEC.
#[derive(Clone)]
pub struct KvStore {
    inner: Arc<Mutex<BTreeMap<String, Entry>>>,
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut store = self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("KV store mutex was poisoned, recovering");
            poisoned.into_inner()
        });
        if let Some(entry) = store.get(key) {
            if entry.expires_at.is_some_and(|e| Instant::now() > e) {
                store.remove(key);
                return None;
            }
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn set(&self, key: String, value: String, ttl_secs: u64) {
        let expires_at = if ttl_secs > 0 {
            Some(Instant::now() + Duration::from_secs(ttl_secs))
        } else {
            None
        };

        self.inner
            .lock()
            .unwrap_or_else(|poisoned| {
                tracing::warn!("KV store mutex was poisoned, recovering");
                poisoned.into_inner()
            })
            .insert(key, Entry { value, expires_at });
    }

    pub fn delete(&self, key: &str) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| {
                tracing::warn!("KV store mutex was poisoned, recovering");
                poisoned.into_inner()
            })
            .remove(key)
            .is_some()
    }

    pub fn has(&self, key: &str) -> bool {
        let mut store = self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("KV store mutex was poisoned, recovering");
            poisoned.into_inner()
        });
        if let Some(entry) = store.get(key) {
            if entry.expires_at.is_some_and(|e| Instant::now() > e) {
                store.remove(key);
                return false;
            }
            true
        } else {
            false
        }
    }

    pub fn list(&self, prefix: Option<&str>, limit: usize) -> Vec<String> {
        let mut store = self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("KV store mutex was poisoned, recovering");
            poisoned.into_inner()
        });
        let now = Instant::now();

        // Collect expired keys first
        let expired: Vec<String> = store
            .iter()
            .filter_map(|(k, entry)| {
                if entry.expires_at.is_some_and(|e| now > e) {
                    return Some(k.clone());
                }
                None
            })
            .collect();

        for key in &expired {
            store.remove(key.as_str());
        }

        // Now collect the result after removing expired entries
        store
            .iter()
            .filter(|(k, _)| prefix.is_none_or(|p| k.starts_with(p)))
            .take(limit)
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn clear(&self) {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| {
                tracing::warn!("KV store mutex was poisoned, recovering");
                poisoned.into_inner()
            })
            .clear();
    }
}

// --- Persistent KV file format for CLI commands ---

#[derive(Serialize, Deserialize, Default)]
pub struct KvFile {
    pub entries: BTreeMap<String, KvFileEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct KvFileEntry {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

pub fn kv_file_path(project_dir: &Path) -> std::path::PathBuf {
    data_dir(project_dir).join("kv.json")
}

pub fn load_kv_file(path: &Path) -> KvFile {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => KvFile::default(),
    }
}

pub fn save_kv_file(path: &Path, kv: &KvFile) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(kv)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn is_expired(entry: &KvFileEntry) -> bool {
    if let Some(expires_at) = entry.expires_at {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > expires_at
    } else {
        false
    }
}
