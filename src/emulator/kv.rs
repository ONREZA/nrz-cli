use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::data_dir;

struct Entry {
    value: String,
    metadata: Option<String>,
    expires_at: Option<Instant>,
}

impl Entry {
    fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|e| now > e)
    }
}

/// In-memory KV store with TTL support.
///
/// Matches the ONREZA.kv API contract from BUILD_OUTPUT_SPEC.
#[derive(Clone)]
pub struct KvStore {
    inner: Arc<Mutex<BTreeMap<String, Entry>>>,
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
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
            if entry.is_expired(Instant::now()) {
                store.remove(key);
                return None;
            }
            Some(entry.value.clone())
        } else {
            None
        }
    }

    pub fn set(&self, key: String, value: String, ttl_secs: u64, metadata: Option<String>) {
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
            .insert(
                key,
                Entry {
                    value,
                    expires_at,
                    metadata,
                },
            );
    }

    pub fn get_with_metadata(&self, key: &str) -> (Option<String>, Option<String>) {
        let mut store = self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("KV store mutex was poisoned, recovering");
            poisoned.into_inner()
        });
        if let Some(entry) = store.get(key) {
            if entry.is_expired(Instant::now()) {
                store.remove(key);
                return (None, None);
            }
            (Some(entry.value.clone()), entry.metadata.clone())
        } else {
            (None, None)
        }
    }

    pub fn get_many(&self, keys: &[String]) -> Vec<Option<String>> {
        let mut store = self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("KV store mutex was poisoned, recovering");
            poisoned.into_inner()
        });
        let now = Instant::now();
        let mut expired = Vec::new();
        let values: Vec<Option<String>> = keys
            .iter()
            .map(|key| {
                if let Some(entry) = store.get(key.as_str()) {
                    if entry.is_expired(now) {
                        expired.push(key.clone());
                        None
                    } else {
                        Some(entry.value.clone())
                    }
                } else {
                    None
                }
            })
            .collect();
        for key in &expired {
            store.remove(key.as_str());
        }
        values
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
            if entry.is_expired(Instant::now()) {
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
                if entry.is_expired(now) {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

pub fn kv_file_path_for_env(project_dir: &Path, env: &str) -> std::path::PathBuf {
    let env = sanitize_env_name(env);
    data_dir(project_dir).join(format!("kv.{env}.json"))
}

fn sanitize_env_name(env: &str) -> String {
    let sanitized: String = env
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "development".to_string()
    } else {
        sanitized
    }
}

pub fn load_kv_file(path: &Path) -> KvFile {
    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(kv) => kv,
            Err(e) => {
                tracing::warn!("corrupt KV file at {}: {e}, starting fresh", path.display());
                KvFile::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => KvFile::default(),
        Err(e) => {
            eprintln!(
                "  {} failed to read {}: {e}. Using empty KV store.",
                console::style("!").yellow().bold(),
                path.display()
            );
            KvFile::default()
        }
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
