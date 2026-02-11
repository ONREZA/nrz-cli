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
    let (bin, args) = parts.split_first().context("empty dev command")?;

    // Convert bootstrap path to a file:// URL to avoid issues with spaces/special chars
    let bootstrap_url = url::Url::from_file_path(bootstrap_path)
        .map_err(|_| anyhow::anyhow!("invalid bootstrap path: {}", bootstrap_path.display()))?;
    let existing = std::env::var("NODE_OPTIONS").unwrap_or_default();
    let node_options = if existing.is_empty() {
        format!("--import {bootstrap_url}")
    } else {
        format!("{existing} --import {bootstrap_url}")
    };

    // Use npx/bunx to resolve the binary
    let mut cmd = Command::new("bunx");
    cmd.arg(bin)
        .args(args)
        .current_dir(project_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("NODE_OPTIONS", node_options);

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
            // Send SIGTERM for graceful shutdown
            if let Some(pid) = child.id() {
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                // Wait up to 5 seconds
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    child.wait()
                ).await {
                    Ok(_) => {},
                    Err(_) => {
                        tracing::warn!("dev server did not exit after 5s, force killing");
                        let _ = child.kill().await;
                    }
                }
            } else {
                let _ = child.kill().await;
            }
        }
    }

    Ok(())
}
