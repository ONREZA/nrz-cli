//! CLI handler for `nrz kv` subcommands.

use std::path::Path;

use nrz::emulator::kv::{
    KvFileEntry, is_expired, kv_file_path, load_kv_file, save_kv_file,
};

use super::kv::{KvArgs, KvCommand};

pub async fn run(args: KvArgs) -> anyhow::Result<()> {
    let project_dir = Path::new(".").canonicalize()?;
    let path = kv_file_path(&project_dir);

    match args.command {
        KvCommand::Get { key } => {
            let kv = load_kv_file(&path);
            match kv.entries.get(&key) {
                Some(entry) if !is_expired(entry) => {
                    println!("{}", entry.value);
                }
                _ => {
                    eprintln!("(not found)");
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
            kv.entries
                .insert(key.clone(), KvFileEntry { value, expires_at });
            save_kv_file(&path, &kv)?;
            eprintln!("OK");
        }
        KvCommand::Delete { key } => {
            let mut kv = load_kv_file(&path);
            if kv.entries.remove(&key).is_some() {
                save_kv_file(&path, &kv)?;
                eprintln!("deleted");
            } else {
                eprintln!("(not found)");
            }
        }
        KvCommand::List { prefix, limit } => {
            let kv = load_kv_file(&path);
            let mut count = 0;
            for (key, entry) in &kv.entries {
                if is_expired(entry) {
                    continue;
                }
                if let Some(ref p) = prefix
                    && !key.starts_with(p)
                {
                    continue;
                }
                println!("{key}");
                count += 1;
                if count >= limit {
                    break;
                }
            }
            if count == 0 {
                eprintln!("(empty)");
            }
        }
        KvCommand::Clear { force } => {
            if !force {
                eprintln!("use --force to confirm clearing all KV data");
                return Ok(());
            }
            if path.exists() {
                std::fs::remove_file(&path)?;
                eprintln!("KV store cleared");
            } else {
                eprintln!("KV store is already empty");
            }
        }
    }
    Ok(())
}
