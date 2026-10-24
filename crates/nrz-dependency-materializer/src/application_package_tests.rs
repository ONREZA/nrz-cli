use crate::application_erofs_image::{metadata_only_image, write_extracted_tree};

use super::*;
use crate::application_erofs_tools::{
    ApplicationErofsToolExecutor, ApplicationErofsToolOutput, ApplicationToolInvocation,
    ApplicationToolOperation,
};
use nrz_runtime_artifact::ArtifactRoot;
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, admission::ManifestMemoryPool, encode_source_bundle_v2_manifest,
};
use sha2::{Digest, Sha256};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn file(path: &str, role: FileRole) -> FileEntry {
    FileEntry::File {
        path: nrz_contract::source_bundle_v2::NonEmptyString(path.into()),
        role,
        layer_name: None,
        content_type: None,
        blob_digest: BlobDigest {
            algorithm: DigestAlgorithm::Blake3,
            value: blake3::hash(b"abc").to_hex().to_string(),
        },
        size: 3,
        executable: false,
    }
}

use crate::pending_cleanup::{PendingCleanup, Retained};

struct PendingExecutor {
    extraction: bool,
    retained: Retained,
    touched: Arc<Mutex<Vec<PathBuf>>>,
    handoff_closed: Arc<AtomicUsize>,
}

impl ApplicationErofsToolExecutor for PendingExecutor {
    fn execute(
        &self,
        invocation: ApplicationToolInvocation<'_>,
    ) -> Result<ApplicationErofsToolOutput, ToolError> {
        match invocation.operation {
            ApplicationToolOperation::Generate {
                source_tree,
                image_path,
            } => {
                assert_eq!(invocation.limits.max_output_nodes, 1);
                self.touched
                    .lock()
                    .unwrap()
                    .extend([source_tree.to_owned(), image_path.to_owned()]);
                if self.extraction {
                    fs::write(image_path, fixture_image(&invocation)).unwrap();
                    return Ok(ApplicationErofsToolOutput {
                        success: true,
                        status: "0".into(),
                        stderr: vec![],
                    });
                }
            }
            ApplicationToolOperation::Extract {
                extraction_directory,
                ..
            } => {
                assert!(self.extraction);
                assert_eq!(invocation.limits.max_output_nodes, 2);
                assert_eq!(invocation.limits.max_output_bytes, 3);
                self.touched
                    .lock()
                    .unwrap()
                    .push(extraction_directory.to_owned());
            }
        }
        Err(ToolError::with_pending_cleanup(
            "worker cleanup pending",
            PendingCleanup {
                retained: self.retained.clone(),
                handoff_closed: self.handoff_closed.clone(),
            },
        ))
    }
}

fn fixture_image(invocation: &ApplicationToolInvocation<'_>) -> Vec<u8> {
    let uuid = invocation
        .arguments
        .iter()
        .find_map(|argument| argument.to_str()?.strip_prefix("-U"))
        .unwrap();
    let uuid = hex::decode(uuid.replace('-', ""))
        .unwrap()
        .try_into()
        .unwrap();
    metadata_only_image(uuid)
}

#[test]
fn successful_packages_keep_executor_storage_until_package_readers_are_released() {
    struct StorageExecutor {
        storage: tempfile::TempDir,
        remaining_at_release: Arc<AtomicUsize>,
    }
    impl Drop for StorageExecutor {
        fn drop(&mut self) {
            self.remaining_at_release.store(
                fs::read_dir(self.storage.path()).unwrap().count(),
                Ordering::Relaxed,
            );
        }
    }
    impl ApplicationErofsToolExecutor for StorageExecutor {
        fn execute(
            &self,
            invocation: ApplicationToolInvocation<'_>,
        ) -> Result<ApplicationErofsToolOutput, ToolError> {
            match invocation.operation {
                ApplicationToolOperation::Generate { image_path, .. } => {
                    fs::write(image_path, fixture_image(&invocation)).unwrap();
                }
                ApplicationToolOperation::Extract {
                    extraction_directory,
                    ..
                } => {
                    write_extracted_tree(extraction_directory);
                }
            }
            Ok(ApplicationErofsToolOutput {
                success: true,
                status: "0".into(),
                stderr: vec![],
            })
        }
    }
    let storage = tempfile::tempdir().unwrap();
    let parent = storage.path().to_owned();
    let pool = ManifestMemoryPool::new(8 * 1024 * 1024);
    let source = prepared(&parent, &pool);
    let remaining_at_release = Arc::new(AtomicUsize::new(usize::MAX));
    let executor = Arc::new(StorageExecutor {
        storage,
        remaining_at_release: remaining_at_release.clone(),
    });
    let retained = Arc::downgrade(&executor);
    let tool_parent = tempfile::tempdir().unwrap();
    let executable = tool_parent.path().join("tool");
    fs::write(&executable, b"test tool").unwrap();
    fs::set_permissions(&executable, Permissions::from_mode(0o700)).unwrap();
    let hash = hex::encode(Sha256::digest(b"test tool"));
    let tools =
        ApplicationErofsTools::new(executable.clone(), executable, &hash, &hash, executor).unwrap();
    let packages = materialize_application_packages(source, &parent, &tools, limits()).unwrap();
    drop(tools);
    assert!(
        retained.upgrade().is_some(),
        "live packages lost their storage owner"
    );
    let mut reader = packages.open_package(Role::Static).unwrap();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(
        bytes.len() as u64,
        packages.descriptors().next().unwrap().wire().blob.size
    );
    drop(reader);
    drop(packages);
    assert!(retained.upgrade().is_none());
    assert_eq!(remaining_at_release.load(Ordering::Relaxed), 0);
    assert!(!parent.exists());
    assert_eq!(pool.reserved_bytes(), 0);
}

// A metadata-only EROFS fixture: extraction is simulated at the cleanup boundary.

#[test]
fn pending_generate_and_extract_keep_custody_after_error_drop_until_cleanup_proof() {
    struct Reservation(Arc<AtomicUsize>);
    impl Drop for Reservation {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    for extraction in [false, true] {
        let parent = tempfile::tempdir().unwrap();
        let tool_parent = tempfile::tempdir().unwrap();
        let pool = ManifestMemoryPool::new(8 * 1024 * 1024);
        let source = prepared(parent.path(), &pool);
        let retained = Retained::default();
        let touched = Arc::new(Mutex::new(Vec::new()));
        let handoff_closed = Arc::new(AtomicUsize::new(0));
        let executable = tool_parent.path().join("tool");
        fs::write(&executable, b"test tool").unwrap();
        fs::set_permissions(&executable, Permissions::from_mode(0o700)).unwrap();
        let hash = hex::encode(Sha256::digest(b"test tool"));
        let tools = ApplicationErofsTools::new(
            executable.clone(),
            executable,
            &hash,
            &hash,
            Arc::new(PendingExecutor {
                extraction,
                retained: retained.clone(),
                touched: touched.clone(),
                handoff_closed: handoff_closed.clone(),
            }),
        )
        .unwrap();
        let error = materialize_application_packages(source, parent.path(), &tools, limits())
            .err()
            .unwrap();
        let ApplicationMaterializationError::Tool(tool_error) = &error else {
            panic!("{error}")
        };
        assert!(tool_error.cleanup_pending());
        assert!(
            std::error::Error::source(tool_error)
                .unwrap()
                .downcast_ref::<PendingCleanup>()
                .is_some()
        );
        let released = Arc::new(AtomicUsize::new(0));
        tool_error.retain_until_cleanup(Reservation(released.clone()));
        drop(error);
        assert_eq!(handoff_closed.load(Ordering::Relaxed), 1);
        drop(tools);
        assert!(pool.reserved_bytes() > 0);
        assert_eq!(released.load(Ordering::Relaxed), 0);
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 2);
        assert!(touched.lock().unwrap().iter().all(|path| path.exists()));
        retained.lock().unwrap().clear();
        assert_eq!(pool.reserved_bytes(), 0);
        assert_eq!(released.load(Ordering::Relaxed), 1);
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }
}

fn limits() -> ApplicationMaterializationLimits {
    ApplicationMaterializationLimits {
        max_package_bytes: 16384,
        max_staging_file_bytes: 100000,
        max_staging_nodes: 100,
        max_path_bytes: 4096,
        max_verification_memory_bytes: 1024 * 1024,
    }
}

#[test]
fn recipe_uses_physical_tree_and_tool_identity_only() {
    let entry = file("a/f", FileRole::Static);
    let original = recipe(&[&entry], Role::Static, "tools");
    assert_eq!(
        hex::encode(original),
        "14e8e636d0f2ced7f1ded822ce77c0f651cbc5235482f9d6fba6451b32b5900f"
    );
    let mut other = entry.clone();
    if let FileEntry::File {
        role,
        layer_name,
        content_type,
        executable,
        ..
    } = &mut other
    {
        *role = FileRole::Prerender;
        *layer_name = Some(nrz_contract::source_bundle_v2::LayerName("other".into()));
        *content_type = Some("text/plain".into());
        *executable = true;
    }
    assert_eq!(original, recipe(&[&other], Role::Static, "tools"));
    assert_ne!(original, recipe(&[&entry], Role::Compute, "tools"));
    assert_ne!(original, recipe(&[&entry], Role::Static, "other-tools"));
    if let FileEntry::File { size, .. } = &mut other {
        *size += 1;
    }
    assert_ne!(original, recipe(&[&other], Role::Static, "tools"));
    let link = FileEntry::Symlink {
        path: nrz_contract::source_bundle_v2::NonEmptyString("a/f".into()),
        role: FileRole::Static,
        layer_name: None,
        content_type: None,
        link_target: nrz_contract::source_bundle_v2::NonEmptyString("other".into()),
        size: 0,
    };
    assert_ne!(original, recipe(&[&link], Role::Static, "tools"));
}

#[test]
fn partition_preflight_counts_implicit_directories_and_bounds_before_work() {
    let files = [
        file("a-", FileRole::Config),
        file("a/f", FileRole::Static),
        file("a/g", FileRole::Compute),
        file("a/h", FileRole::Prerender),
        file("dep", FileRole::Dependency),
    ];
    let usage = preflight(&files, limits()).unwrap();
    assert_eq!(
        (usage[0].entries, usage[0].nodes, usage[0].file_bytes),
        (3, 5, 9)
    );
    assert_eq!(
        (usage[1].entries, usage[1].nodes, usage[1].file_bytes),
        (1, 3, 3)
    );
    for bad in [
        ApplicationMaterializationLimits {
            max_staging_file_bytes: 20,
            ..limits()
        },
        ApplicationMaterializationLimits {
            max_staging_nodes: 10,
            ..limits()
        },
        ApplicationMaterializationLimits {
            max_verification_memory_bytes: 10,
            ..limits()
        },
        ApplicationMaterializationLimits {
            max_path_bytes: 2,
            ..limits()
        },
        ApplicationMaterializationLimits {
            max_package_bytes: u64::MAX,
            ..limits()
        },
    ] {
        assert!(preflight(&files, bad).is_err());
    }
}

#[test]
fn small_role_preflight_does_not_reserve_the_global_package_ceiling() {
    let files = (0..995)
        .map(|index| {
            let mut entry = file(&format!("app/{index:04}.js"), FileRole::Compute);
            if let FileEntry::File { size, .. } = &mut entry {
                *size = 6000;
            }
            entry
        })
        .collect::<Vec<_>>();
    let usage = preflight(
        &files,
        ApplicationMaterializationLimits {
            max_package_bytes: 12 * 1024 * 1024 * 1024,
            max_staging_file_bytes: 128 * 1024 * 1024,
            max_staging_nodes: 4096,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(usage[0].package_bytes, 0);
    assert!(usage[1].package_bytes < 64 * 1024 * 1024);
    assert_eq!(usage[1].file_bytes, 995 * 6000);
}

#[derive(Clone, Copy)]
enum Failure {
    Execute,
    Replace,
    Oversize,
}

struct BrokenGenerator {
    failure: Failure,
    calls: Arc<AtomicUsize>,
}

impl ApplicationErofsToolExecutor for BrokenGenerator {
    fn execute(
        &self,
        invocation: ApplicationToolInvocation<'_>,
    ) -> Result<ApplicationErofsToolOutput, ToolError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let ApplicationToolOperation::Generate { image_path, .. } = invocation.operation else {
            panic!("invalid image must fail before extraction");
        };
        match self.failure {
            Failure::Execute => return Err(ToolError::new("worker failed")),
            Failure::Replace => {
                fs::remove_file(image_path).unwrap();
                // The still-open original inode cannot be reused by this new file.
                fs::write(image_path, b"replacement").unwrap();
            }
            Failure::Oversize => {
                OpenOptions::new()
                    .write(true)
                    .open(image_path)
                    .unwrap()
                    .set_len(invocation.limits.max_output_bytes + 1)
                    .unwrap();
            }
        }
        Ok(ApplicationErofsToolOutput {
            success: true,
            status: "0".into(),
            stderr: vec![],
        })
    }
}

fn prepared(parent: &Path, pool: &ManifestMemoryPool) -> PreparedSourceV2 {
    let input = tempfile::tempdir().unwrap();
    fs::write(input.path().join("file"), b"abc").unwrap();
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 1,
        max_logical_bytes: 3,
        max_static_file_bytes: 3,
    };
    let bytes = encode_source_bundle_v2_manifest(serde_json::json!({
        "schemaVersion": "SOURCE_BUNDLE_V2.0", "capabilities": [],
        "files": [file("file", FileRole::Static)], "layers": [], "routes": [], "entrypoints": [],
    }), budget).unwrap();
    let mut input_manifest = pool.reserve_input(bytes.len()).unwrap();
    input_manifest.append(&bytes).unwrap();
    let admitted = input_manifest.verify(budget, u64::MAX).unwrap();
    crate::source_v2_preparation::prepare_source_v2(
        admitted,
        &ArtifactRoot::open(input.path()).unwrap(),
        parent,
        crate::source_v2_preparation::PreparationLimits {
            max_file_bytes: 3,
            max_nodes: 3,
            max_path_bytes: 4,
        },
    )
    .unwrap()
}

fn broken_tools(parent: &Path, failure: Failure, calls: Arc<AtomicUsize>) -> ApplicationErofsTools {
    let executable = parent.join("tool");
    fs::write(&executable, b"test tool").unwrap();
    fs::set_permissions(&executable, Permissions::from_mode(0o700)).unwrap();
    let hash = hex::encode(Sha256::digest(b"test tool"));
    ApplicationErofsTools::new(
        executable.clone(),
        executable,
        &hash,
        &hash,
        Arc::new(BrokenGenerator { failure, calls }),
    )
    .unwrap()
}

#[test]
fn generation_failure_replacement_and_overflow_release_private_custody() {
    for failure in [Failure::Execute, Failure::Replace, Failure::Oversize] {
        let parent = tempfile::tempdir().unwrap();
        let tool_parent = tempfile::tempdir().unwrap();
        let pool = ManifestMemoryPool::new(8 * 1024 * 1024);
        let source = prepared(parent.path(), &pool);
        assert!(pool.reserved_bytes() > 0);
        let calls = Arc::new(AtomicUsize::new(0));
        let tools = broken_tools(tool_parent.path(), failure, calls.clone());
        let error = materialize_application_packages(source, parent.path(), &tools, limits())
            .err()
            .expect("invalid image accepted");
        match failure {
            Failure::Execute => assert!(matches!(error, ApplicationMaterializationError::Tool(_))),
            Failure::Replace => assert!(
                error
                    .to_string()
                    .contains("replaced the private image inode")
            ),
            Failure::Oversize => assert!(matches!(
                error,
                ApplicationMaterializationError::Limit("package byte limit")
            )),
        }
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(pool.reserved_bytes(), 0);
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }
}

#[test]
fn insufficient_admission_rejects_before_invoking_worker() {
    let parent = tempfile::tempdir().unwrap();
    let tool_parent = tempfile::tempdir().unwrap();
    let pool = ManifestMemoryPool::new(8 * 1024 * 1024);
    let calls = Arc::new(AtomicUsize::new(0));
    let tools = broken_tools(tool_parent.path(), Failure::Execute, calls.clone());
    let error = materialize_application_packages(
        prepared(parent.path(), &pool),
        parent.path(),
        &tools,
        ApplicationMaterializationLimits {
            max_staging_nodes: 0,
            ..limits()
        },
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        ApplicationMaterializationError::Limit("staging node limit")
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(pool.reserved_bytes(), 0);
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
}
