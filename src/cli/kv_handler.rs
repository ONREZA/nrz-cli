//! CLI handler for `nrz kv` subcommands.

use std::path::Path;

use serde::Serialize;

use nrz::emulator::kv::{
    KvFileEntry, is_expired, kv_file_path_for_env, load_kv_file, save_kv_file,
};

use super::kv::{KvArgs, KvCommand};
use crate::output;

#[derive(Serialize)]
struct KvGetOutput {
    key: String,
    value: Option<String>,
}

#[derive(Serialize)]
struct KvListOutput {
    keys: Vec<String>,
}

#[derive(Serialize)]
struct StatusOutput {
    status: String,
}

pub async fn run(args: KvArgs, json: bool) -> anyhow::Result<()> {
    let project_dir = Path::new(".").canonicalize()?;
    let path = kv_file_path_for_env(&project_dir, &args.env);

    match args.command {
        KvCommand::Get { key } => {
            let kv = load_kv_file(&path);
            let value = match kv.entries.get(&key) {
                Some(entry) if !is_expired(entry) => Some(entry.value.clone()),
                _ => None,
            };

            if json {
                output::json_output(&KvGetOutput { key, value });
            } else {
                match value {
                    Some(v) => println!("{v}"),
                    None => eprintln!("(not found)"),
                }
            }
        }
        KvCommand::Set { key, value, ttl } => {
            let mut kv = load_kv_file(&path);
            let expires_at = if ttl > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Some(now + ttl)
            } else {
                None
            };
            kv.entries.insert(
                key.clone(),
                KvFileEntry {
                    value,
                    expires_at,
                    metadata: None,
                },
            );
            save_kv_file(&path, &kv)?;

            if json {
                output::json_output(&StatusOutput {
                    status: "ok".into(),
                });
            } else {
                eprintln!("OK");
            }
        }
        KvCommand::Delete { key } => {
            let mut kv = load_kv_file(&path);
            let found = kv.entries.remove(&key).is_some();
            if found {
                save_kv_file(&path, &kv)?;
            }

            if json {
                output::json_output(&StatusOutput {
                    status: "ok".into(),
                });
            } else if found {
                eprintln!("deleted");
            } else {
                eprintln!("(not found)");
            }
        }
        KvCommand::List { prefix, limit } => {
            let kv = load_kv_file(&path);
            let mut keys = Vec::new();
            for (key, entry) in &kv.entries {
                if is_expired(entry) {
                    continue;
                }
                if let Some(ref p) = prefix
                    && !key.starts_with(p)
                {
                    continue;
                }
                keys.push(key.clone());
                if keys.len() >= limit {
                    break;
                }
            }

            if json {
                output::json_output(&KvListOutput { keys });
            } else if keys.is_empty() {
                eprintln!("(empty)");
            } else {
                for key in &keys {
                    println!("{key}");
                }
            }
        }
        KvCommand::Clear { force } => {
            if !force {
                eprintln!("use --force to confirm clearing all KV data");
                return Ok(());
            }
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            if json {
                output::json_output(&StatusOutput {
                    status: "ok".into(),
                });
            } else {
                eprintln!("KV store cleared");
            }
        }
    }
    Ok(())
}
