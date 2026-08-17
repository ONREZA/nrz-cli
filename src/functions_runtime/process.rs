use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use super::CachedRuntime;

const PROTOCOL_VERSION: &str = "onreza-functions-poc/v1";
const CONTROL_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct RuntimeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Option<JoinHandle<Vec<u8>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlMessage {
    #[serde(rename = "type")]
    kind: String,
    protocol_version: Option<String>,
    runtime_release_id: Option<String>,
}

impl RuntimeProcess {
    pub(crate) async fn start(
        runtime: &CachedRuntime,
        bundle_root: &Path,
        entrypoint: &str,
    ) -> anyhow::Result<Self> {
        let mut command = Command::new(&runtime.path);
        command
            .env_clear()
            .env("NODE_ENV", "production")
            .env("ONREZA_FUNCTIONS_BUNDLE_ROOT", bundle_root)
            .env("ONREZA_FUNCTIONS_CONTROL_MODE", "stdio")
            .env("ONREZA_FUNCTIONS_ENTRYPOINT", entrypoint)
            .env(
                "ONREZA_FUNCTIONS_RUNTIME_RELEASE_ID",
                &runtime.runtime_release_id,
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        preserve_host_environment(&mut command);
        let mut child = command.spawn().with_context(|| {
            format!(
                "failed to start ONREZA Functions runtime {}",
                runtime.path.display()
            )
        })?;
        let stdin = child
            .stdin
            .take()
            .context("runtime stdin pipe is missing")?;
        let stdout = child
            .stdout
            .take()
            .context("runtime stdout pipe is missing")?;
        let mut child_stderr = child
            .stderr
            .take()
            .context("runtime stderr pipe is missing")?;
        let stderr = tokio::spawn(async move {
            let mut bytes = Vec::new();
            let _ = child_stderr.read_to_end(&mut bytes).await;
            bytes
        });
        let mut process = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
            stderr: Some(stderr),
        };
        let ready = match process.read_control().await {
            Ok(message) => message,
            Err(error) => {
                if process.child.try_wait()?.is_none() {
                    let _ = process.child.kill().await;
                }
                let _ = process.child.wait().await;
                let stderr = process.take_stderr().await;
                return Err(error.context(stderr_context(&stderr)));
            }
        };
        if ready.kind != "ready"
            || ready.protocol_version.as_deref() != Some(PROTOCOL_VERSION)
            || ready.runtime_release_id.as_deref() != Some(&runtime.runtime_release_id)
        {
            bail!("ONREZA Functions runtime returned an incompatible ready message");
        }
        Ok(process)
    }

    pub(crate) async fn shutdown(mut self) -> anyhow::Result<()> {
        self.write_control(&json!({ "type": "shutdown" })).await?;
        self.stdin.shutdown().await.ok();
        let status = match timeout(CONTROL_TIMEOUT, self.child.wait()).await {
            Ok(status) => status.context("failed to wait for ONREZA Functions runtime")?,
            Err(_) => {
                self.child
                    .kill()
                    .await
                    .context("failed to stop ONREZA Functions runtime")?;
                bail!("ONREZA Functions runtime did not stop within 5 seconds");
            }
        };
        let stderr = self.take_stderr().await;
        if !status.success() {
            bail!(
                "ONREZA Functions runtime exited with {status}: {}",
                stderr_context(&stderr)
            );
        }
        Ok(())
    }

    async fn write_control(&mut self, message: &Value) -> anyhow::Result<()> {
        let mut line =
            serde_json::to_vec(message).context("failed to encode runtime control message")?;
        line.push(b'\n');
        self.stdin
            .write_all(&line)
            .await
            .context("failed to write ONREZA Functions runtime control message")?;
        self.stdin
            .flush()
            .await
            .context("failed to flush ONREZA Functions runtime control message")
    }

    async fn read_control(&mut self) -> anyhow::Result<ControlMessage> {
        let line = timeout(CONTROL_TIMEOUT, self.stdout.next_line())
            .await
            .context("ONREZA Functions runtime control timeout")??
            .context("ONREZA Functions runtime closed its control stream")?;
        serde_json::from_str(&line)
            .context("ONREZA Functions runtime returned invalid control JSON")
    }

    async fn take_stderr(&mut self) -> Vec<u8> {
        match self.stderr.take() {
            Some(task) => task.await.unwrap_or_default(),
            None => Vec::new(),
        }
    }
}

#[cfg(windows)]
fn preserve_host_environment(command: &mut Command) {
    for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
}

#[cfg(not(windows))]
fn preserve_host_environment(_: &mut Command) {}

fn stderr_context(stderr: &[u8]) -> String {
    let value = String::from_utf8_lossy(stderr);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "runtime produced no diagnostic output".to_string()
    } else {
        trimmed.to_string()
    }
}
