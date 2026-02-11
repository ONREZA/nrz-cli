use std::path::Path;
use std::process::Stdio;

use anyhow::Context;
use tokio::process::Command;
use tokio::signal;

/// Spawn the framework dev server as a child process.
///
/// Injects the ONREZA bootstrap script via `NODE_OPTIONS=--import`.
/// Forwards stdout/stderr to the terminal.
/// Handles SIGINT/SIGTERM for graceful shutdown.
pub async fn spawn_dev_server(
    project_dir: &Path,
    dev_command: &str,
    bootstrap_path: &Path,
) -> anyhow::Result<()> {
    let parts: Vec<&str> = dev_command.split_whitespace().collect();
    let (bin, args) = parts
        .split_first()
        .context("empty dev command")?;

    // Use npx/bunx to resolve the binary
    let mut cmd = Command::new("bunx");
    cmd.arg(bin)
        .args(args)
        .current_dir(project_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env(
            "NODE_OPTIONS",
            format!("--import {}", bootstrap_path.display()),
        );

    let mut child = cmd.spawn().context("failed to start dev server")?;

    // Wait for either the child to exit or a shutdown signal
    tokio::select! {
        status = child.wait() => {
            let status = status?;
            if !status.success() {
                anyhow::bail!("dev server exited with {status}");
            }
        }
        _ = signal::ctrl_c() => {
            tracing::info!("shutting down dev server...");
            child.kill().await?;
        }
    }

    Ok(())
}
