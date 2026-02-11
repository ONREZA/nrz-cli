use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::cli::kv::{KvArgs, KvCommand};

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
        let mut store = self.inner.lock().unwrap();
        if let Some(entry) = store.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    store.remove(key);
                    return None;
                }
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

        self.inner.lock().unwrap().insert(
            key,
            Entry { value, expires_at },
        );
    }

    pub fn delete(&self, key: &str) -> bool {
        self.inner.lock().unwrap().remove(key).is_some()
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn list(&self, prefix: Option<&str>, limit: usize) -> Vec<String> {
        let store = self.inner.lock().unwrap();
        let now = Instant::now();

        store
            .iter()
            .filter(|(k, entry)| {
                if let Some(p) = prefix {
                    if !k.starts_with(p) {
                        return false;
                    }
                }
                entry.expires_at.is_none_or(|exp| now <= exp)
            })
            .take(limit)
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

/// CLI handler for `nrz kv` subcommands.
pub async fn run(args: KvArgs) -> anyhow::Result<()> {
    // For CLI commands, we read/write from a JSON file for persistence.
    // The in-memory KvStore is used during `nrz dev` runtime.
    match args.command {
        KvCommand::Get { key } => {
            eprintln!("nrz kv get: not yet implemented (key: {key})");
        }
        KvCommand::Set { key, value, ttl } => {
            eprintln!("nrz kv set: not yet implemented (key: {key}, value: {value}, ttl: {ttl})");
        }
        KvCommand::Delete { key } => {
            eprintln!("nrz kv delete: not yet implemented (key: {key})");
        }
        KvCommand::List { prefix, limit } => {
            eprintln!("nrz kv list: not yet implemented (prefix: {prefix:?}, limit: {limit})");
        }
        KvCommand::Clear { force } => {
            if !force {
                eprintln!("use --force to confirm clearing all KV data");
                return Ok(());
            }
            eprintln!("nrz kv clear: not yet implemented");
        }
    }
    Ok(())
}
