//! Fixed application-image operations. Tools and their rootfs must remain in
//! trusted immutable custody from construction through the final invocation.

use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Copy, Debug)]
pub struct ApplicationToolLimits {
    /// Logical regular-file bytes validated before and after execution.
    /// Extraction requires a separate physical storage budget while running.
    pub max_output_bytes: u64,
    /// Validated output entries, including the extraction root (one for the image inode).
    /// This is not a filesystem inode quota: hardlinks can share an inode.
    pub max_output_nodes: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum ApplicationToolOperation<'a> {
    Generate {
        source_tree: &'a Path,
        image_path: &'a Path,
    },
    Extract {
        image_path: &'a Path,
        extraction_directory: &'a Path,
    },
}

pub struct ApplicationToolInvocation<'a> {
    pub executable: &'a Path,
    pub arguments: &'a [OsString],
    pub operation: ApplicationToolOperation<'a>,
    pub limits: ApplicationToolLimits,
}

#[derive(Debug)]
pub struct ApplicationErofsToolOutput {
    pub success: bool,
    pub status: String,
    pub stderr: Vec<u8>,
}

/// Production implementations must use bounded, no-network one-shot workers,
/// mount input scopes read-only, expose only the declared output scopes, and reap
/// every descendant/close writable handles before return. Generation writes the
/// precreated image inode; extraction writes only the empty destination tree.
/// An error without cleanup proof must carry a pending-cleanup retainer; ordinary
/// errors and successful output certify that writable workers have stopped.
/// Output capture, time, memory, process and filesystem admission belong here.
/// Extraction must use isolated storage with admitted physical bytes and inodes,
/// including filesystem overhead. Block limits and per-file RLIMIT_FSIZE do not
/// enforce aggregate logical bytes while running; the materializer validates the
/// image inventory before extraction and the exact output after worker cleanup.
/// The executor owns its physical storage reservation. Successful packages retain
/// it; pending errors must retain the same storage until native cleanup is proved.
pub trait ApplicationErofsToolExecutor: Send + Sync {
    fn execute(
        &self,
        invocation: ApplicationToolInvocation<'_>,
    ) -> Result<ApplicationErofsToolOutput, ToolError>;
}

pub use crate::ErofsToolExecutionError as ToolError;

pub struct ApplicationErofsTools {
    mkfs_path: PathBuf,
    fsck_path: PathBuf,
    generator_identity: String,
    executor: Arc<dyn ApplicationErofsToolExecutor>,
}

impl ApplicationErofsTools {
    pub fn new(
        mkfs_path: PathBuf,
        fsck_path: PathBuf,
        expected_mkfs_sha256: &str,
        expected_fsck_sha256: &str,
        executor: Arc<dyn ApplicationErofsToolExecutor>,
    ) -> Result<Self, ToolError> {
        verify_tool(&mkfs_path, expected_mkfs_sha256)?;
        verify_tool(&fsck_path, expected_fsck_sha256)?;
        let mut identity = Sha256::new();
        identity.update(b"ONREZA_APPLICATION_EROFS_TOOLS_V1\n");
        identity.update(expected_mkfs_sha256.as_bytes());
        identity.update(b"\n");
        identity.update(expected_fsck_sha256.as_bytes());
        identity.update(b"\n");
        Ok(Self {
            mkfs_path,
            fsck_path,
            generator_identity: format!("sha256:{}", hex::encode(identity.finalize())),
            executor,
        })
    }

    #[must_use]
    pub fn generator_identity(&self) -> &str {
        &self.generator_identity
    }

    pub(crate) fn execution_owner(&self) -> Arc<dyn ApplicationErofsToolExecutor> {
        Arc::clone(&self.executor)
    }

    pub(crate) fn generate(
        &self,
        source_tree: &Path,
        image_path: &Path,
        uuid: [u8; 16],
        limits: ApplicationToolLimits,
    ) -> Result<(), ToolError> {
        let uuid = hex::encode(uuid);
        let uuid = format!(
            "-U{}-{}-{}-{}-{}",
            &uuid[..8],
            &uuid[8..12],
            &uuid[12..16],
            &uuid[16..20],
            &uuid[20..]
        );
        let mut arguments = ["-z", "zstd,level=3", "-C4096", "-b4096", "-T0"]
            .map(OsString::from)
            .to_vec();
        arguments.push(uuid.into());
        arguments.extend(
            [
                "--all-root",
                "--all-time",
                "--hard-dereference",
                "--ignore-mtime",
                "--workers=1",
                "-x",
                "-1",
                "--quiet",
            ]
            .map(OsString::from),
        );
        arguments.extend([
            image_path.as_os_str().to_owned(),
            source_tree.as_os_str().to_owned(),
        ]);
        self.run(
            &self.mkfs_path,
            &arguments,
            ApplicationToolOperation::Generate {
                source_tree,
                image_path,
            },
            limits,
        )
    }

    pub(crate) fn extract(
        &self,
        image_path: &Path,
        extraction_directory: &Path,
        limits: ApplicationToolLimits,
    ) -> Result<(), ToolError> {
        let mut destination = OsString::from("--extract=");
        destination.push(extraction_directory);
        let arguments = [
            destination,
            "--no-preserve-owner".into(),
            "--preserve-perms".into(),
            image_path.as_os_str().to_owned(),
        ];
        self.run(
            &self.fsck_path,
            &arguments,
            ApplicationToolOperation::Extract {
                image_path,
                extraction_directory,
            },
            limits,
        )
    }

    fn run(
        &self,
        executable: &Path,
        arguments: &[OsString],
        operation: ApplicationToolOperation<'_>,
        limits: ApplicationToolLimits,
    ) -> Result<(), ToolError> {
        let output = self.executor.execute(ApplicationToolInvocation {
            executable,
            arguments,
            operation,
            limits,
        })?;
        if output.success {
            return Ok(());
        }
        let status = output.status.chars().take(128).collect::<String>();
        let stderr = String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(4096)]);
        Err(ToolError::new(format!("{status}: {stderr}")))
    }
}

fn verify_tool(path: &Path, expected: &str) -> Result<(), ToolError> {
    if !path.is_absolute()
        || expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ToolError::new(
            "absolute tool path and lowercase SHA-256 required",
        ));
    }
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| ToolError::new(format!("inspect pinned tool: {error}")))?;
    use std::os::unix::fs::PermissionsExt;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(ToolError::new("pinned tool must be a regular executable"));
    }
    let mut file =
        File::open(path).map_err(|error| ToolError::new(format!("open pinned tool: {error}")))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| ToolError::new(format!("hash pinned tool: {error}")))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if hex::encode(hash.finalize()) != expected {
        return Err(ToolError::new("pinned tool SHA-256 mismatch"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "application_erofs_tools_tests.rs"]
mod tests;
