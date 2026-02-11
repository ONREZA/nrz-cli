pub mod detect;
pub mod inject;
mod process;

#[cfg(test)]
mod detect_tests;

#[cfg(test)]
mod inject_tests;

use anyhow::Context;

use crate::cli::DevArgs;
use nrz::emulator;
use nrz::emulator::kv::KvStore;
use nrz::emulator::server::EmulatorServer;

/// Start local dev server with platform emulation.
///
/// 1. Detect framework (Astro, Nuxt, Nitro, SvelteKit)
/// 2. Start emulator (KV, DB, Context)
/// 3. Generate JS bootstrap that sets globalThis.ONREZA
/// 4. Spawn framework dev command as child process
/// 5. Forward signals, handle graceful shutdown
pub async fn run(args: DevArgs) -> anyhow::Result<()> {
    let project_dir = std::path::Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    // 1. Detect framework or use custom command
    let dev_command = if let Some(ref cmd) = args.command {
        cmd.clone()
    } else {
        let framework = detect::detect_framework(&project_dir)?;
        eprintln!(
            "  {} detected {:?}",
            console::style("~").cyan().bold(),
            framework.name,
        );
        framework.dev_command
    };

    // 2. Ensure data directory
    let data_dir = emulator::ensure_data_dir(&project_dir)?;
    let db_path = data_dir.join("dev.db");

    // 3. Generate bootstrap script
    let emulator_port = args.port + 1;
    let bootstrap = inject::generate_bootstrap(&data_dir, emulator_port)?;
    let bootstrap_path = data_dir.join("bootstrap.mjs");
    std::fs::write(&bootstrap_path, &bootstrap)?;

    // 4. Create KV store + emulator server
    let kv = KvStore::new();
    let server = EmulatorServer::new(kv, db_path, emulator_port);

    // 5. Start emulator server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            tracing::error!(%e, "emulator server error");
        }
    });

    // 6. Wait for emulator to be ready
    wait_for_emulator(emulator_port).await?;

    eprintln!(
        "  {} emulator ready on port {emulator_port}",
        console::style("~").cyan().bold(),
    );
    eprintln!(
        "  {} starting: {dev_command}",
        console::style(">").green().bold(),
    );

    // 7. Spawn framework dev server (blocks until exit or Ctrl+C)
    let result = process::spawn_dev_server(&project_dir, &dev_command, &bootstrap_path).await;

    // 8. Cleanup
    server_handle.abort();
    let _ = std::fs::remove_file(&bootstrap_path);

    result
}

async fn wait_for_emulator(port: u16) -> anyhow::Result<()> {
    let url = format!("http://127.0.0.1:{port}/__nrz/health");
    for _ in 0..50 {
        match reqwest::get(&url).await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    anyhow::bail!("emulator server failed to start on port {port}")
}
