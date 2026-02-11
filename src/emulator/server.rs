use std::net::SocketAddr;

use super::kv::KvStore;

/// Local HTTP server for the emulator.
///
/// Runs alongside the framework dev server. The JS bootstrap
/// (injected into Node.js) proxies ONREZA.kv/db calls to this server.
///
/// Endpoints:
///   POST /__nrz/kv/{method}  — KV operations (get, set, delete, has, list)
///   POST /__nrz/db/query     — SQL query (all, first, run, raw modes)
///   POST /__nrz/db/batch     — Batch SQL execution
///   POST /__nrz/db/exec      — Multi-statement SQL
///   GET  /__nrz/health       — Health check
pub struct EmulatorServer {
    pub kv: KvStore,
    pub db_path: std::path::PathBuf,
    pub addr: SocketAddr,
}

impl EmulatorServer {
    pub fn new(kv: KvStore, db_path: std::path::PathBuf, port: u16) -> Self {
        Self {
            kv,
            db_path,
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
        }
    }

    /// Start the emulator HTTP server.
    pub async fn start(&self) -> anyhow::Result<()> {
        // TODO: implement with hyper or axum
        // For now, placeholder
        tracing::info!(%self.addr, "emulator server listening");
        Ok(())
    }
}
