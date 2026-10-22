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

/// Ordinary errors and all output certify that workers have stopped and closed
/// writable handles. Without that proof, return a pending-cleanup error so the
/// caller can retain its complete source/output owner and resource reservations.
pub trait ErofsToolExecutor: Send + Sync {
    fn execute(
        &self,
        invocation: ErofsToolInvocation<'_>,
    ) -> Result<ErofsToolOutput, ErofsToolExecutionError>;
}

/// Owns the executor's pending-cleanup error/handoff. Retention must be infallible
/// and transfer resources into an independent cleanup registry, releasing them
/// only after cleanup proof. Dropping this wrapper closes the handoff, without
/// releasing retained resources. Never retain the enclosing tool error itself.
pub trait PendingErofsToolCleanup: std::error::Error + Send + Sync {
    fn retain(&self, resource: Box<dyn Send>);
}

#[derive(Debug)]
pub struct ErofsToolExecutionError {
    message: String,
    pending_cleanup: Option<Box<dyn PendingErofsToolCleanup>>,
}

impl std::fmt::Display for ErofsToolExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ErofsToolExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.pending_cleanup.as_deref().map(|pending| pending as _)
    }
}

impl ErofsToolExecutionError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            pending_cleanup: None,
        }
    }

    #[must_use]
    pub fn with_pending_cleanup(
        message: impl Into<String>,
        pending: impl PendingErofsToolCleanup + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            pending_cleanup: Some(Box::new(pending)),
        }
    }

    #[must_use]
    pub fn cleanup_pending(&self) -> bool {
        self.pending_cleanup.is_some()
    }

    /// Transfer caller-owned custody/reservations before dropping a pending error.
    /// Ordinary errors immediately drop the resource; their cleanup is complete.
    pub fn retain_until_cleanup<T: Send + 'static>(&self, resource: T) {
        if let Some(pending) = &self.pending_cleanup {
            pending.retain(Box::new(resource));
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
