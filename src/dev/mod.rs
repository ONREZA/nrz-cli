pub mod inject;
mod process;

#[cfg(test)]
mod inject_tests;

use anyhow::Context;

use crate::cli::DevArgs;
use nrz::config::ProjectConfig;
use nrz::emulator::kv::KvStore;
use nrz::emulator::server::EmulatorServer;

/// Start local dev server with platform emulation.
///
/// 1. Resolve dev command (--alias > --command > config)
/// 2. Ensure data directory exists
/// 3. Generate JS bootstrap (globalThis.ONREZA)
/// 4. Create KV store + emulator server
/// 5. Start emulator in background, wait for readiness
/// 6. Build NODE_OPTIONS (bootstrap + optional inspector)
/// 7. Spawn dev command as child process
/// 8. Cleanup on exit
pub async fn run(args: DevArgs, config: &ProjectConfig) -> anyhow::Result<()> {
    let project_dir = std::path::Path::new(&args.dir)
        .canonicalize()
        .with_context(|| format!("project directory not found: {}", args.dir))?;

    // 1. Resolve dev command: --alias > --command > [dev] command
    let dev_command = if let Some(ref name) = args.alias {
        config
            .dev_alias_command(name)
            .with_context(|| {
                format!("alias '{name}' not found — define it in [dev.aliases] in onreza.toml")
            })?
            .to_string()
    } else if let Some(ref cmd) = args.command {
        cmd.clone()
    } else if let Some(ref cmd) = config.dev.command {
        cmd.clone()
    } else {
        anyhow::bail!(
            "no dev command specified — set [dev] command in onreza.toml, use --command, or define aliases in [dev.aliases]"
        );
    };

    // 2. Ensure data directory
    let data_dir = config.data_dir_path(&project_dir);
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("failed to create data directory: {}", data_dir.display()))?;
    let db_path = data_dir.join(config.db_name());

    // 3. Generate bootstrap script
    let port = args.port.unwrap_or(config.dev_port());
    let emulator_port = port + 1;
    let bootstrap = inject::generate_bootstrap(&data_dir, emulator_port)?;
    let bootstrap_path = data_dir.join("bootstrap.mjs");
    std::fs::write(&bootstrap_path, &bootstrap)
        .with_context(|| format!("failed to write bootstrap: {}", bootstrap_path.display()))?;

    // 4. Create KV store + emulator server
    let kv = KvStore::new();
    let host = config.dev_host();
    let server = EmulatorServer::new(kv, db_path, emulator_port, host)?;

    // 5. Start emulator server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            tracing::error!(%e, "emulator server error");
        }
    });

    // 6. Wait for emulator to be ready
    wait_for_emulator(emulator_port, host).await?;

    eprintln!(
        "  {} emulator ready on port {emulator_port}",
        console::style("~").cyan().bold(),
    );

    // 6. Build extra NODE_OPTIONS (--inspect / --inspect-brk)
    let inspect_flag = if args.inspect_brk {
        Some("--inspect-brk")
    } else if args.inspect {
        Some("--inspect")
    } else {
        None
    };

    eprintln!(
        "  {} starting: {dev_command}",
        console::style(">").green().bold(),
    );

    // 7. Spawn dev server (blocks until exit or Ctrl+C)
    let result =
        process::spawn_dev_server(&project_dir, &dev_command, &bootstrap_path, inspect_flag).await;

    // 8. Cleanup
    server_handle.abort();
    let _ = std::fs::remove_file(&bootstrap_path);

    result
}

async fn wait_for_emulator(port: u16, host: &str) -> anyhow::Result<()> {
    let url = format!("http://{host}:{port}/__nrz/health");
    for _ in 0..50 {
        match reqwest::get(&url).await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    }
    anyhow::bail!("emulator server failed to start on port {port}")
}
