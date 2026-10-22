use super::*;
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    sync::Mutex,
};

#[derive(Default)]
struct RecordingExecutor {
    calls: Mutex<Vec<Vec<OsString>>>,
    fail: bool,
}

impl ApplicationErofsToolExecutor for RecordingExecutor {
    fn execute(
        &self,
        invocation: ApplicationToolInvocation<'_>,
    ) -> Result<ApplicationErofsToolOutput, ToolError> {
        assert_eq!(invocation.limits.max_output_bytes, 1234);
        assert_eq!(invocation.limits.max_output_nodes, 7);
        match invocation.operation {
            ApplicationToolOperation::Generate {
                source_tree,
                image_path,
            } => {
                assert_eq!(source_tree, Path::new("/private/source"));
                assert_eq!(image_path, Path::new("/private/image"));
            }
            ApplicationToolOperation::Extract {
                image_path,
                extraction_directory,
            } => {
                assert_eq!(image_path, Path::new("/private/image"));
                assert_eq!(extraction_directory, Path::new("/private/readback"));
            }
        }
        assert!(invocation.executable.is_absolute());
        self.calls
            .lock()
            .unwrap()
            .push(invocation.arguments.to_vec());
        Ok(ApplicationErofsToolOutput {
            success: !self.fail,
            status: "exited 1".into(),
            stderr: vec![b'x'; 20_000],
        })
    }
}

fn tool(directory: &Path, name: &str, byte: u8) -> (PathBuf, String) {
    let path = directory.join(name);
    let bytes = vec![byte; 100_003];
    std::fs::write(&path, &bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (path, hex::encode(Sha256::digest(bytes)))
}

#[test]
fn builds_only_fixed_commands_and_identifies_both_tools() {
    let staging = tempfile::tempdir().unwrap();
    let (mkfs, mkfs_hash) = tool(staging.path(), "mkfs", 1);
    let (fsck, fsck_hash) = tool(staging.path(), "fsck", 2);
    let executor = Arc::new(RecordingExecutor::default());
    let tools = ApplicationErofsTools::new(
        mkfs.clone(),
        fsck.clone(),
        &mkfs_hash,
        &fsck_hash,
        executor.clone(),
    )
    .unwrap();
    let same = ApplicationErofsTools::new(
        mkfs.clone(),
        fsck.clone(),
        &mkfs_hash,
        &fsck_hash,
        executor.clone(),
    )
    .unwrap();
    assert_eq!(tools.generator_identity(), same.generator_identity());
    let other =
        ApplicationErofsTools::new(fsck.clone(), mkfs, &fsck_hash, &mkfs_hash, executor.clone())
            .unwrap();
    assert_ne!(tools.generator_identity(), other.generator_identity());
    let limits = ApplicationToolLimits {
        max_output_bytes: 1234,
        max_output_nodes: 7,
    };
    tools
        .generate(
            Path::new("/private/source"),
            Path::new("/private/image"),
            [0x12; 16],
            limits,
        )
        .unwrap();
    tools
        .extract(
            Path::new("/private/image"),
            Path::new("/private/readback"),
            limits,
        )
        .unwrap();
    let calls = executor.calls.lock().unwrap();
    assert_eq!(
        calls[0],
        [
            "-z",
            "zstd,level=3",
            "-C4096",
            "-b4096",
            "-T0",
            "-U12121212-1212-1212-1212-121212121212",
            "--all-root",
            "--all-time",
            "--hard-dereference",
            "--ignore-mtime",
            "--workers=1",
            "-x",
            "-1",
            "--quiet",
            "/private/image",
            "/private/source"
        ]
        .map(OsString::from)
    );
    assert_eq!(
        calls[1],
        [
            "--extract=/private/readback",
            "--no-preserve-owner",
            "--preserve-perms",
            "/private/image"
        ]
        .map(OsString::from)
    );
}

#[test]
fn rejects_wrong_hash_non_executable_symlink_and_relative_tool() {
    let staging = tempfile::tempdir().unwrap();
    let (mkfs, mkfs_hash) = tool(staging.path(), "mkfs", 1);
    let (fsck, fsck_hash) = tool(staging.path(), "fsck", 2);
    let executor = Arc::new(RecordingExecutor::default());
    assert!(
        ApplicationErofsTools::new(
            mkfs.clone(),
            fsck.clone(),
            &fsck_hash,
            &fsck_hash,
            executor.clone()
        )
        .is_err()
    );
    assert!(
        ApplicationErofsTools::new(
            "relative".into(),
            fsck.clone(),
            &mkfs_hash,
            &fsck_hash,
            executor.clone()
        )
        .is_err()
    );
    let alias = staging.path().join("alias");
    symlink(&mkfs, &alias).unwrap();
    assert!(
        ApplicationErofsTools::new(
            alias,
            fsck.clone(),
            &mkfs_hash,
            &fsck_hash,
            executor.clone()
        )
        .is_err()
    );
    std::fs::set_permissions(&mkfs, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(ApplicationErofsTools::new(mkfs, fsck, &mkfs_hash, &fsck_hash, executor).is_err());
}

#[test]
fn bounds_failure_diagnostics() {
    let staging = tempfile::tempdir().unwrap();
    let (mkfs, mkfs_hash) = tool(staging.path(), "mkfs", 1);
    let (fsck, fsck_hash) = tool(staging.path(), "fsck", 2);
    let tools = ApplicationErofsTools::new(
        mkfs,
        fsck,
        &mkfs_hash,
        &fsck_hash,
        Arc::new(RecordingExecutor {
            fail: true,
            ..Default::default()
        }),
    )
    .unwrap();
    let error = tools
        .extract(
            Path::new("/private/image"),
            Path::new("/private/readback"),
            ApplicationToolLimits {
                max_output_bytes: 1234,
                max_output_nodes: 7,
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("exited 1"));
    assert!(error.to_string().len() < 4300);
}
