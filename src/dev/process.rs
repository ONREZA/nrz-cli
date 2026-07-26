use std::path::Path;
use std::process::Stdio;

use anyhow::Context;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::signal;

const DEV_LOG_CHUNK_BYTES: usize = 12 * 1024;

pub(super) enum DevServerExit {
    Exited,
    Interrupted,
}

impl DevServerExit {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Exited => "exited",
            Self::Interrupted => "interrupted",
        }
    }
}

/// Spawn the dev server as a child process.
///
/// Injects the ONREZA bootstrap script via `NODE_OPTIONS=--import`.
/// Forwards stdout/stderr to the terminal.
/// Handles SIGINT/SIGTERM for graceful shutdown.
///
/// `inspect_flag` — optional Node.js inspector flag (`--inspect` or `--inspect-brk`).
pub async fn spawn_dev_server(
    project_dir: &Path,
    dev_command: &str,
    bootstrap_path: &Path,
    inspect_flag: Option<&str>,
    extra_env: &std::collections::HashMap<String, String>,
    json: bool,
) -> anyhow::Result<DevServerExit> {
    if dev_command.trim().is_empty() {
        anyhow::bail!("empty dev command");
    }
    #[cfg(unix)]
    let (bin, args) = ("sh", ["-c", dev_command]);
    #[cfg(windows)]
    let (bin, args) = ("cmd", ["/C", dev_command]);

    // Convert bootstrap path to a file:// URL to avoid issues with spaces/special chars
    let bootstrap_url = url::Url::from_file_path(bootstrap_path)
        .map_err(|_| anyhow::anyhow!("invalid bootstrap path: {}", bootstrap_path.display()))?;
    let existing = std::env::var("NODE_OPTIONS").unwrap_or_default();
    let mut node_options = if existing.is_empty() {
        format!("--import {bootstrap_url}")
    } else {
        format!("{existing} --import {bootstrap_url}")
    };
    if let Some(flag) = inspect_flag {
        node_options.push(' ');
        node_options.push_str(flag);
    }

    let mut cmd = Command::new(bin);
    cmd.args(args).current_dir(project_dir);
    if json {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    }
    for (key, val) in extra_env {
        cmd.env(key, val);
    }
    for key in crate::execution_context::private_cli_environment_keys() {
        cmd.env_remove(key);
    }
    // Emulator bootstrap is a platform patch and wins over materialized NODE_OPTIONS.
    cmd.env("NODE_OPTIONS", node_options);

    let mut child = cmd.spawn().context("failed to start dev server")?;
    let output_tasks = if json {
        let stdout = child
            .stdout
            .take()
            .context("expected piped stdout from dev server")?;
        let stderr = child
            .stderr
            .take()
            .context("expected piped stderr from dev server")?;
        Some((
            tokio::spawn(forward_json_output(stdout, "user", "info")),
            tokio::spawn(forward_json_output(stderr, "debug", "warn")),
        ))
    } else {
        None
    };

    // Wait for either the child to exit or a shutdown signal
    let exit = tokio::select! {
        status = child.wait() => {
            let status = status?;
            if !status.success() {
                anyhow::bail!("dev server exited with {status}");
            }
            DevServerExit::Exited
        }
        _ = signal::ctrl_c() => {
            tracing::info!("shutting down dev server...");
            // Graceful shutdown: SIGTERM on Unix, kill on Windows
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    child.wait()
                ).await {
                    Ok(_) => {},
                    Err(_) => {
                        tracing::warn!("dev server did not exit after 5s, force killing");
                        unsafe { libc::kill(pid as i32, libc::SIGKILL); }
                        let _ = child.wait().await;
                    }
                }
            } else {
                let _ = child.kill().await;
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill().await;
            }
            DevServerExit::Interrupted
        }
    };

    if let Some((stdout, stderr)) = output_tasks {
        await_output_task(stdout, "stdout").await;
        await_output_task(stderr, "stderr").await;
    }

    Ok(exit)
}

async fn await_output_task(handle: tokio::task::JoinHandle<anyhow::Result<()>>, stream: &str) {
    match tokio::time::timeout(std::time::Duration::from_secs(2), handle).await {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => tracing::warn!(%error, stream, "failed to read dev server output"),
        Ok(Err(error)) => tracing::warn!(%error, stream, "dev server output task failed"),
        Err(_) => tracing::warn!(stream, "timed out draining dev server output"),
    }
}

async fn forward_json_output<R>(
    mut reader: R,
    stream: &'static str,
    level: &'static str,
) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut read_buffer = [0u8; 4096];
    let mut pending = Vec::new();
    loop {
        let read = reader.read(&mut read_buffer).await?;
        if read == 0 {
            if !pending.is_empty() {
                emit_json_chunk(stream, level, &pending);
            }
            return Ok(());
        }

        for byte in &read_buffer[..read] {
            if *byte == b'\n' {
                if pending.last() == Some(&b'\r') {
                    pending.pop();
                }
                emit_json_chunk(stream, level, &pending);
                pending.clear();
            } else {
                pending.push(*byte);
                if pending.len() >= DEV_LOG_CHUNK_BYTES {
                    emit_json_chunk(stream, level, &pending);
                    pending.clear();
                }
            }
        }
    }
}

fn emit_json_chunk(stream: &str, level: &str, bytes: &[u8]) {
    crate::output::log_line(
        stream,
        level,
        crate::output::Phase::Dev.as_str(),
        &String::from_utf8_lossy(bytes),
    );
}
