use nrz_dependency_materializer::{
    DependencyMaterializationKind, DependencyMaterializationRequest, DependencyMaterializerError,
    DependencySymlinkScope, DependencyTreeLimits, ErofsToolExecutionError, ErofsToolExecutor,
    ErofsToolInvocation, ErofsToolKind, ErofsToolOutput, ErofsToolchain, PendingErofsToolCleanup,
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[path = "support/pending_cleanup.rs"]
mod pending_cleanup;
use pending_cleanup::{PendingCleanup, Retained};

struct FailingExecutor {
    failure: ErofsToolKind,
    pending: bool,
    retained: Retained,
    handoff_closed: Arc<AtomicUsize>,
    image: Arc<Mutex<Option<PathBuf>>>,
}

impl ErofsToolExecutor for FailingExecutor {
    fn execute(
        &self,
        invocation: ErofsToolInvocation<'_>,
    ) -> Result<ErofsToolOutput, ErofsToolExecutionError> {
        if invocation.kind == ErofsToolKind::Mkfs {
            fs::write(invocation.image_path, b"partial image").unwrap();
            *self.image.lock().unwrap() = Some(invocation.image_path.to_owned());
        }
        if invocation.kind != self.failure {
            return Ok(ErofsToolOutput {
                success: true,
                status: "0".into(),
                stderr: vec![],
            });
        }
        if self.pending {
            Err(ErofsToolExecutionError::with_pending_cleanup(
                "worker cleanup pending",
                PendingCleanup {
                    retained: self.retained.clone(),
                    handoff_closed: self.handoff_closed.clone(),
                },
            ))
        } else {
            Err(ErofsToolExecutionError::new("worker stopped"))
        }
    }
}

#[test]
fn pending_mkfs_retains_partial_image_until_cleanup_after_error_drop() {
    check_cleanup(true, ErofsToolKind::Mkfs);
}

#[test]
fn pending_fsck_retains_partial_image_until_cleanup_after_error_drop() {
    check_cleanup(true, ErofsToolKind::Fsck);
}

#[test]
fn ordinary_mkfs_and_fsck_errors_remove_partial_image_immediately() {
    for failure in [ErofsToolKind::Mkfs, ErofsToolKind::Fsck] {
        check_cleanup(false, failure);
    }
}

fn check_cleanup(pending: bool, failure: ErofsToolKind) {
    let parent = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("index.js"), b"export const value = 1;").unwrap();
    let tool = parent.path().join("tool");
    fs::write(&tool, b"fake tool").unwrap();
    let output = parent.path().join("image.erofs");
    let retained = Retained::default();
    let handoff_closed = Arc::new(AtomicUsize::new(0));
    let image = Arc::new(Mutex::new(None));
    let tools = ErofsToolchain::with_executor(
        &tool,
        &tool,
        Arc::new(FailingExecutor {
            failure,
            pending,
            retained: retained.clone(),
            handoff_closed: handoff_closed.clone(),
            image: image.clone(),
        }),
    )
    .unwrap();
    let error = tools
        .materialize(DependencyMaterializationRequest {
            source_tree: source.path(),
            output_image: &output,
            kind: DependencyMaterializationKind::JavaScriptNodeModules,
            compatibility: serde_json::json!({}),
            limits: DependencyTreeLimits {
                max_files: 10,
                max_expanded_bytes: 1024,
                max_path_bytes: 4096,
                max_symlinks: 10,
            },
            symlink_scope: DependencySymlinkScope::ClosedTree,
        })
        .unwrap_err();
    let DependencyMaterializerError::ToolExecution(tool_error) = &error else {
        panic!("{error}");
    };
    assert_eq!(tool_error.cleanup_pending(), pending);
    let image = image.lock().unwrap().clone().unwrap();
    let source_path = source.path().to_owned();
    assert!(!output.exists());
    assert_eq!(image.exists(), pending, "{failure:?}");
    if pending {
        assert!(
            std::error::Error::source(tool_error)
                .unwrap()
                .downcast_ref::<PendingCleanup>()
                .is_some()
        );
        tool_error.retain_until_cleanup(source);
    } else {
        assert!(std::error::Error::source(tool_error).is_none());
        tool_error.retain_until_cleanup(source);
        assert!(!source_path.exists());
    }
    drop(error);
    drop(tools);
    assert_eq!(handoff_closed.load(Ordering::Relaxed), usize::from(pending));
    assert_eq!(image.exists(), pending);
    assert_eq!(source_path.exists(), pending);
    retained.lock().unwrap().clear();
    assert!(!image.exists());
    assert!(!source_path.exists());
}
