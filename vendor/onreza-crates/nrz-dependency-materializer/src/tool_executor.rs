// @generated vendored copy of platform crates/nrz-dependency-materializer/src/tool_executor.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErofsToolKind {
    Mkfs,
    Fsck,
}

impl ErofsToolKind {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Mkfs => "mkfs.erofs",
            Self::Fsck => "fsck.erofs --extract",
        }
    }
}

pub struct ErofsToolInvocation<'a> {
    pub kind: ErofsToolKind,
    pub executable: &'a Path,
    pub arguments: Vec<OsString>,
    pub source_tree: Option<&'a Path>,
    pub image_path: &'a Path,
    pub max_output_bytes: u64,
}

#[derive(Debug)]
pub struct ErofsToolOutput {
    pub success: bool,
    pub status: String,
    pub stderr: Vec<u8>,
}

pub trait ErofsToolExecutor: Send + Sync {
    fn execute(
        &self,
        invocation: ErofsToolInvocation<'_>,
    ) -> Result<ErofsToolOutput, ErofsToolExecutionError>;
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct ErofsToolExecutionError {
    message: String,
}

impl ErofsToolExecutionError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub(crate) fn direct_executor() -> Arc<dyn ErofsToolExecutor> {
    Arc::new(DirectErofsToolExecutor)
}

struct DirectErofsToolExecutor;

impl ErofsToolExecutor for DirectErofsToolExecutor {
    fn execute(
        &self,
        invocation: ErofsToolInvocation<'_>,
    ) -> Result<ErofsToolOutput, ErofsToolExecutionError> {
        let output = Command::new(invocation.executable)
            .args(invocation.arguments)
            .output()
            .map_err(|error| {
                ErofsToolExecutionError::new(format!(
                    "execute {} at {}: {error}",
                    invocation.kind.name(),
                    invocation.executable.display()
                ))
            })?;
        Ok(ErofsToolOutput {
            success: output.status.success(),
            status: output.status.to_string(),
            stderr: output.stderr,
        })
    }
}
