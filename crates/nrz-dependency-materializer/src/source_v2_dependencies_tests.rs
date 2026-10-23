use crate::application_erofs_image::{metadata_only_image, write_extracted_tree};

use super::*;
use crate::application_erofs_tools::{
    ApplicationErofsToolExecutor, ApplicationErofsToolOutput, ApplicationToolInvocation,
    ApplicationToolOperation, ToolError,
};
use crate::source_v2_preparation::{PreparationLimits, prepare_source_v2};
use crate::{
    ErofsToolExecutionError, ErofsToolExecutor, ErofsToolInvocation, ErofsToolKind, ErofsToolOutput,
};
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, admission::ManifestMemoryPool, encode_source_bundle_v2_manifest,
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::pending_cleanup::{PendingCleanup, Retained};

struct Fixture {
    source: Option<PreparedSourceV2>,
    pool: ManifestMemoryPool,
    input: TempDir,
    _snapshot: TempDir,
}
fn fixture(roots: &[&str], cross_link: bool) -> Fixture {
    fixture_entries(roots, cross_link, false)
}
fn fixture_entries(roots: &[&str], cross_link: bool, nested: bool) -> Fixture {
    let input = tempfile::tempdir().unwrap();
    let snapshot = tempfile::tempdir().unwrap();
    fs::write(input.path().join("file"), b"abc").unwrap();
    let mut files = vec![
        json!({"path":"file","entryType":"file","role":"compute","layerName":"app",
        "blobDigest":{"algorithm":"BLAKE3","value":blake3::hash(b"abc").to_hex().as_str()},"size":3,"executable":false}),
    ];
    for root in roots {
        fs::create_dir_all(input.path().join(root)).unwrap();
        for (name, executable) in [("plain", false), ("run", true)] {
            let path = if nested {
                format!("{root}/pkg/{name}")
            } else {
                format!("{root}/{name}")
            };
            fs::create_dir_all(input.path().join(&path).parent().unwrap()).unwrap();
            fs::write(input.path().join(&path), b"dependency").unwrap();
            fs::set_permissions(
                input.path().join(&path),
                Permissions::from_mode(if executable { 0o755 } else { 0o644 }),
            )
            .unwrap();
            files.push(json!({"path":path,"entryType":"file","role":"dependency","layerName":"app",
                "blobDigest":{"algorithm":"BLAKE3","value":blake3::hash(b"dependency").to_hex().as_str()},"size":10,"executable":executable}));
        }
    }
    if cross_link {
        let path = format!("{}/link", roots[0]);
        let target = format!("../{}/plain", roots[1]);
        symlink(&target, input.path().join(&path)).unwrap();
        files.push(json!({"path":path,"entryType":"symlink","role":"dependency","layerName":"app","linkTarget":target,"size":0}));
    }
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 100,
        max_logical_bytes: 4096,
        max_static_file_bytes: 4096,
    };
    let bytes = encode_source_bundle_v2_manifest(json!({"schemaVersion":"SOURCE_BUNDLE_V2.0","capabilities":["COMPUTE_NETWORK_V1"],
        "files":files,"layers":[{"name":"app","target":"COMPUTE","entrypoint":"file","runtimeConfig":{
            "applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24","network":nrz_source_bundle::ComputeNetwork::public_default()}}],
        "routes":[{"pattern":"/","layerName":"app"}],"entrypoints":["file"]}),budget).unwrap();
    let pool = ManifestMemoryPool::new(8 * 1024 * 1024);
    let mut admitted = pool.reserve_input(bytes.len()).unwrap();
    admitted.append(&bytes).unwrap();
    let source = prepare_source_v2(
        admitted.verify(budget, 100_000).unwrap(),
        &ArtifactRoot::open(input.path()).unwrap(),
        snapshot.path(),
        PreparationLimits {
            max_file_bytes: 4096,
            max_nodes: 100,
            max_path_bytes: 4096,
        },
    )
    .unwrap();
    Fixture {
        source: Some(source),
        pool,
        input,
        _snapshot: snapshot,
    }
}
fn runtime_limits() -> RuntimeVerificationLimits {
    RuntimeVerificationLimits {
        max_metadata_bytes: 1024 * 1024,
        max_native_file_bytes: 1024 * 1024,
        max_native_total_bytes: 4 * 1024 * 1024,
        max_path_nodes: 100,
        max_path_work: 100_000,
    }
}
fn limits() -> SourceV2DependencyLimits {
    SourceV2DependencyLimits {
        max_roots: 16,
        max_files: 100,
        max_bytes: 4096,
        max_staging_nodes: 200,
        max_image_bytes: 4096,
    }
}
fn compatibility() -> Value {
    json!({"runtimeFamily":"javascript","runtimeVersion":"node-24","os":"linux","architecture":"x86_64","libc":"glibc","abi":"glibc-2.42",
        "packageManager":"bun","packageManagerVersion":"1.4.2","runnerRootfsDigest":format!("sha256:{}","d".repeat(64)),"buildPolicyGeneration":1})
}
fn policies<'a>(
    roots: &'a [&'a str],
    compatibility: &'a Value,
) -> Vec<SourceV2DependencyPolicy<'a>> {
    roots
        .iter()
        .map(|root| SourceV2DependencyPolicy {
            root: AdmittedDependencyRoot {
                layer_name: "app",
                source_root: root,
                kind: crate::DependencyMaterializationKind::JavaScriptNodeModules,
                build_runtime_target: "node-24",
            },
            compatibility,
            tree_limits: DependencyTreeLimits {
                max_files: 100,
                max_expanded_bytes: 4096,
                max_path_bytes: 4096,
                max_symlinks: 100,
            },
        })
        .collect()
}
fn targets() -> [LayerBuildTarget<'static>; 1] {
    [LayerBuildTarget {
        layer_name: "app",
        build_runtime_target: "node-24",
    }]
}

struct DependencyTools {
    calls: Arc<AtomicUsize>,
    fail_at: Option<usize>,
    retained: Retained,
    closed: Arc<AtomicUsize>,
    touched: Arc<Mutex<Vec<PathBuf>>>,
}
impl ErofsToolExecutor for DependencyTools {
    fn execute(
        &self,
        invocation: ErofsToolInvocation<'_>,
    ) -> Result<ErofsToolOutput, ErofsToolExecutionError> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if invocation.kind == ErofsToolKind::Mkfs {
            fs::write(invocation.image_path, b"one controlled dependency image").unwrap();
            self.touched
                .lock()
                .unwrap()
                .push(invocation.source_tree.unwrap().to_owned());
        }
        if self.fail_at == Some(0) {
            return Err(ErofsToolExecutionError::new("controlled stopped worker"));
        }
        if self.fail_at == Some(call) {
            self.touched
                .lock()
                .unwrap()
                .push(invocation.image_path.to_owned());
            return Err(ErofsToolExecutionError::with_pending_cleanup(
                "controlled pending",
                PendingCleanup {
                    retained: self.retained.clone(),
                    handoff_closed: self.closed.clone(),
                },
            ));
        }
        Ok(ErofsToolOutput {
            success: true,
            status: "0".into(),
            stderr: Vec::new(),
        })
    }
}
struct ToolsFixture {
    toolchain: Arc<ErofsToolchain>,
    calls: Arc<AtomicUsize>,
    retained: Retained,
    closed: Arc<AtomicUsize>,
    touched: Arc<Mutex<Vec<PathBuf>>>,
    executor: std::sync::Weak<DependencyTools>,
    _tools: TempDir,
}
fn tools(fail_at: Option<usize>) -> ToolsFixture {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("never-executed");
    fs::write(&executable, b"controlled pinned dependency tools").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let retained = Retained::default();
    let closed = Arc::new(AtomicUsize::new(0));
    let touched = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(DependencyTools {
        calls: calls.clone(),
        fail_at,
        retained: retained.clone(),
        closed: closed.clone(),
        touched: touched.clone(),
    });
    let weak = Arc::downgrade(&executor);
    let toolchain =
        Arc::new(ErofsToolchain::with_executor(&executable, &executable, executor).unwrap());
    ToolsFixture {
        toolchain,
        calls,
        retained,
        closed,
        touched,
        executor: weak,
        _tools: root,
    }
}
fn prepared(
    fixture: &mut Fixture,
    tools: &ToolsFixture,
    stage: &Path,
    roots: &[&str],
) -> PreparedSourceV2Dependencies {
    prepare_source_v2_dependencies(
        fixture.source.take().unwrap(),
        tools.toolchain.clone(),
        stage,
        &targets(),
        &policies(roots, &compatibility()),
        limits(),
        runtime_limits(),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn prepared_dependency_containers_stay_private_under_permissive_umask() {
    const CHILD: &str = "NRZ_DEPENDENCY_PRIVATE_CONTAINERS_CHILD";
    if std::env::var_os(CHILD).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "source_v2_dependencies::tests::prepared_dependency_containers_stay_private_under_permissive_umask", "--nocapture"])
            .env(CHILD, "1")
            .env("TMPDIR", "/var/tmp")
            .status().unwrap().success());
        return;
    }
    // This isolated test process owns its umask; parallel tests retain their own state.
    unsafe {
        libc::umask(0o022);
    }
    let roots = ["node_modules"];
    let mut fixture = fixture(&roots, false);
    let tools = tools(None);
    let stage = tempfile::tempdir_in("/var/tmp").unwrap();
    let owner = prepared(&mut fixture, &tools, stage.path(), &roots);
    let storage = owner.storage._staging.path();
    for relative in ["", "images", "trees"] {
        assert_eq!(
            fs::metadata(storage.join(relative)).unwrap().mode() & 0o777,
            0o700,
            "dependency storage container must be private: {relative}"
        );
    }
    assert_eq!(
        fs::metadata(storage.join("trees/0")).unwrap().mode() & 0o777,
        0o755
    );
    assert_eq!(tools.calls.load(Ordering::Relaxed), 2);
    drop(owner);
    assert_eq!(fixture.pool.reserved_bytes(), 0);
    assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
}

#[test]
fn prepared_modes_match_existing_materializer_and_producer_mutation_cannot_change_images() {
    let roots = ["node_modules"];
    let mut fixture = fixture(&roots, false);
    fs::write(
        fixture.input.path().join("node_modules/plain"),
        b"producer changed after snapshot",
    )
    .unwrap();
    let tools = tools(None);
    let stage = tempfile::tempdir().unwrap();
    let owner = prepared(&mut fixture, &tools, stage.path(), &roots);
    assert_eq!(tools.calls.load(Ordering::Relaxed), 2);
    assert_eq!(
        owner
            .source
            .root
            .open_file(Path::new("node_modules/plain"))
            .unwrap()
            .metadata()
            .unwrap()
            .mode()
            & 0o777,
        0o775
    );
    let expected = tempfile::tempdir().unwrap();
    for (name, mode) in [("plain", 0o644), ("run", 0o755)] {
        fs::write(expected.path().join(name), b"dependency").unwrap();
        fs::set_permissions(expected.path().join(name), Permissions::from_mode(mode)).unwrap();
    }
    let image = tempfile::tempdir().unwrap();
    let baseline = tools
        .toolchain
        .materialize(DependencyMaterializationRequest {
            source_tree: expected.path(),
            output_image: &image.path().join("baseline.erofs"),
            kind: crate::DependencyMaterializationKind::JavaScriptNodeModules,
            compatibility: compatibility(),
            limits: policies(&roots, &compatibility())[0].tree_limits,
            symlink_scope: DependencySymlinkScope::ClosedTree,
        })
        .unwrap();
    assert_eq!(
        owner.dependencies()[0].manifest().manifest_digest(),
        baseline.manifest.manifest_digest()
    );
    assert_eq!(
        owner.dependencies()[0].manifest().materialization_id(),
        baseline.manifest.materialization_id()
    );
    assert_eq!(
        owner.dependencies()[0]
            .image
            .file
            .metadata()
            .unwrap()
            .nlink(),
        1
    );
    assert_eq!(
        owner.dependencies()[0]
            .image
            .file
            .metadata()
            .unwrap()
            .mode()
            & 0o222,
        0
    );
    let mut reader = &owner.dependencies()[0].image.file;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert_eq!(
        owner.dependencies()[0].physical_blob().digest.value,
        blake3::hash(&bytes).to_hex().as_str()
    );
    assert!(fixture.pool.reserved_bytes() > 0);
    drop(owner);
    assert_eq!(fixture.pool.reserved_bytes(), 0);
    assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
}

#[test]
fn nested_files_at_exact_limit_match_existing_materializer() {
    let roots = ["node_modules"];
    let mut fixture = fixture_entries(&roots, false, true);
    let tools = tools(None);
    let expected = tempfile::tempdir().unwrap();
    fs::create_dir(expected.path().join("pkg")).unwrap();
    for (name, mode) in [("plain", 0o644), ("run", 0o755)] {
        let path = expected.path().join("pkg").join(name);
        fs::write(&path, b"dependency").unwrap();
        fs::set_permissions(path, Permissions::from_mode(mode)).unwrap();
    }
    let compatibility = compatibility();
    let mut policies = policies(&roots, &compatibility);
    policies[0].tree_limits.max_files = 2;
    let image = tempfile::tempdir().unwrap();
    let baseline = tools
        .toolchain
        .materialize(DependencyMaterializationRequest {
            source_tree: expected.path(),
            output_image: &image.path().join("baseline.erofs"),
            kind: policies[0].root.kind,
            compatibility: compatibility.clone(),
            limits: policies[0].tree_limits,
            symlink_scope: DependencySymlinkScope::ClosedTree,
        })
        .expect("existing materializer accepts two regular files in an implicit directory");
    let stage = tempfile::tempdir().unwrap();
    let mut caps = limits();
    caps.max_files = 2;
    let owner = prepare_source_v2_dependencies(
        fixture.source.take().unwrap(),
        tools.toolchain.clone(),
        stage.path(),
        &targets(),
        &policies,
        caps,
        runtime_limits(),
    )
    .expect("prepared materializer must preserve the exact existing file-count limit");
    assert_eq!(
        owner.dependencies()[0].manifest().manifest_digest(),
        baseline.manifest.manifest_digest()
    );
    assert_eq!(
        owner.dependencies()[0].manifest().materialization_id(),
        baseline.manifest.materialization_id()
    );
    drop(owner);
    assert_eq!(fixture.pool.reserved_bytes(), 0);
    assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
}

#[test]
fn equal_materializations_keep_one_inode_and_cross_root_symlinks_use_actual_sibling_trees() {
    let roots = ["node_modules", "other/node_modules"];
    for cross_link in [false, true] {
        let mut fixture = fixture(&roots, cross_link);
        let tools = tools(None);
        let stage = tempfile::tempdir().unwrap();
        let owner = prepared(&mut fixture, &tools, stage.path(), &roots);
        assert_eq!(tools.calls.load(Ordering::Relaxed), 4);
        assert_eq!(owner.dependencies().len(), 2);
        if !cross_link {
            let a = owner.dependencies()[0].image.file.metadata().unwrap();
            let b = owner.dependencies()[1].image.file.metadata().unwrap();
            assert_eq!((a.dev(), a.ino()), (b.dev(), b.ino()));
            assert_eq!(a.nlink(), 1);
        }
    }
}

#[test]
fn aggregate_caps_and_mismatched_policy_fail_before_any_tool() {
    let roots = ["node_modules", "other/node_modules"];
    for bound in [
        "roots",
        "files",
        "bytes",
        "nodes",
        "second-root",
        "target",
        "kind",
        "layer",
    ] {
        let mut fixture = fixture(&roots, false);
        let tools = tools(None);
        let stage = tempfile::tempdir().unwrap();
        let mut caps = limits();
        let mut compatibility = compatibility();
        if bound == "target" {
            compatibility["runtimeVersion"] = "node-22".into();
        }
        let mut policies = policies(&roots, &compatibility);
        match bound {
            "roots" => caps.max_roots = 1,
            "files" => caps.max_files = 3,
            "bytes" => caps.max_bytes = 39,
            "nodes" => caps.max_staging_nodes = 1,
            "second-root" => policies[1].tree_limits.max_files = 1,
            "kind" => {
                policies[0].root.kind = crate::DependencyMaterializationKind::PythonSitePackages
            }
            "layer" => policies[0].root.layer_name = "foreign",
            _ => {}
        }
        let result = prepare_source_v2_dependencies(
            fixture.source.take().unwrap(),
            tools.toolchain.clone(),
            stage.path(),
            &targets(),
            &policies,
            caps,
            runtime_limits(),
        );
        assert!(result.is_err(), "{bound}");
        assert_eq!(tools.calls.load(Ordering::Relaxed), 0, "{bound}");
        assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
        assert_eq!(fixture.pool.reserved_bytes(), 0);
    }
}

#[test]
fn exact_aggregate_limits_succeed_and_generated_image_cap_plus_one_releases_owner() {
    let roots = ["node_modules", "other/node_modules"];
    for one_short in [false, true] {
        let mut fixture = fixture(&roots, false);
        let tools = tools(None);
        let stage = tempfile::tempdir().unwrap();
        let caps = SourceV2DependencyLimits {
            max_roots: 2,
            max_files: 4,
            max_bytes: 40,
            max_staging_nodes: 13,
            max_image_bytes: 2 * b"one controlled dependency image".len() as u64
                - u64::from(one_short),
        };
        let result = prepare_source_v2_dependencies(
            fixture.source.take().unwrap(),
            tools.toolchain.clone(),
            stage.path(),
            &targets(),
            &policies(&roots, &compatibility()),
            caps,
            runtime_limits(),
        );
        assert_eq!(result.is_ok(), !one_short);
        assert_eq!(tools.calls.load(Ordering::Relaxed), 4);
        drop(result);
        assert_eq!(fixture.pool.reserved_bytes(), 0);
        assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
    }
}

#[test]
fn stopped_dependency_error_releases_source_and_stage_without_pending_retention() {
    let roots = ["node_modules"];
    let mut fixture = fixture(&roots, false);
    let tools = tools(Some(0));
    let stage = tempfile::tempdir().unwrap();
    let error = prepare_source_v2_dependencies(
        fixture.source.take().unwrap(),
        tools.toolchain.clone(),
        stage.path(),
        &targets(),
        &policies(&roots, &compatibility()),
        limits(),
        runtime_limits(),
    )
    .err()
    .unwrap();
    assert!(
        matches!(&error, SourceV2DependencyError::Materializer(DependencyMaterializerError::ToolExecution(tool)) if !tool.cleanup_pending())
    );
    assert_eq!(fixture.pool.reserved_bytes(), 0);
    assert!(tools.retained.lock().unwrap().is_empty());
    assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
}

#[test]
fn pending_dependency_tools_keep_entire_owner_after_error_and_caller_drop() {
    let roots = ["node_modules", "other/node_modules"];
    for fail_at in [1, 2, 3, 4] {
        let mut fixture = fixture(&roots, false);
        let tools = tools(Some(fail_at));
        let stage = tempfile::tempdir().unwrap();
        let result = prepare_source_v2_dependencies(
            fixture.source.take().unwrap(),
            tools.toolchain.clone(),
            stage.path(),
            &targets(),
            &policies(&roots, &compatibility()),
            limits(),
            runtime_limits(),
        );
        let error = result.err().unwrap();
        assert!(
            matches!(&error,SourceV2DependencyError::Materializer(DependencyMaterializerError::ToolExecution(tool)) if tool.cleanup_pending())
        );
        let retained = tools.retained.clone();
        let closed = tools.closed.clone();
        let executor = tools.executor.clone();
        let mut touched = tools.touched.lock().unwrap().clone();
        let work = fs::read_dir(stage.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let images = fs::read_dir(work.join("images"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(images.len(), if fail_at <= 2 { 1 } else { 2 });
        touched.extend(images);
        drop(error);
        drop(tools);
        assert_eq!(closed.load(Ordering::Relaxed), 1);
        assert!(fixture.pool.reserved_bytes() > 0);
        assert!(executor.upgrade().is_some());
        assert!(touched.iter().all(|path| path.exists()));
        retained.lock().unwrap().clear();
        assert_eq!(fixture.pool.reserved_bytes(), 0);
        assert!(executor.upgrade().is_none());
        assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
    }
}

struct AppTools {
    calls: Arc<AtomicUsize>,
    pending: bool,
    retained: Retained,
    closed: Arc<AtomicUsize>,
}
impl ApplicationErofsToolExecutor for AppTools {
    fn execute(
        &self,
        invocation: ApplicationToolInvocation<'_>,
    ) -> Result<ApplicationErofsToolOutput, ToolError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.pending {
            return Err(ToolError::with_pending_cleanup(
                "controlled pending app",
                PendingCleanup {
                    retained: self.retained.clone(),
                    handoff_closed: self.closed.clone(),
                },
            ));
        }
        match invocation.operation {
            ApplicationToolOperation::Generate { image_path, .. } => {
                let uuid = invocation
                    .arguments
                    .iter()
                    .find_map(|argument| argument.to_str()?.strip_prefix("-U"))
                    .unwrap();
                fs::write(
                    image_path,
                    metadata_only_image(
                        hex::decode(uuid.replace('-', ""))
                            .unwrap()
                            .try_into()
                            .unwrap(),
                    ),
                )
                .unwrap();
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
            stderr: Vec::new(),
        })
    }
}
fn app_tools(
    root: &Path,
    pending: bool,
    retained: Retained,
    closed: Arc<AtomicUsize>,
) -> (ApplicationErofsTools, Arc<AtomicUsize>) {
    let executable = root.join("app-tool");
    fs::write(&executable, b"controlled app tool").unwrap();
    fs::set_permissions(&executable, Permissions::from_mode(0o700)).unwrap();
    let hash = hex::encode(Sha256::digest(b"controlled app tool"));
    let calls = Arc::new(AtomicUsize::new(0));
    (
        ApplicationErofsTools::new(
            executable.clone(),
            executable,
            &hash,
            &hash,
            Arc::new(AppTools {
                calls: calls.clone(),
                pending,
                retained,
                closed,
            }),
        )
        .unwrap(),
        calls,
    )
}
fn app_limits() -> ApplicationMaterializationLimits {
    ApplicationMaterializationLimits {
        max_package_bytes: 16384,
        max_staging_file_bytes: 100_000,
        max_staging_nodes: 100,
        max_path_bytes: 4096,
        max_verification_memory_bytes: 1024 * 1024,
    }
}

#[derive(Clone)]
struct Selection {
    root: String,
    layer: String,
    mount: String,
    target: String,
    id: String,
    digest: String,
    physical: Option<BlobReference>,
}
fn finish(
    owner: PreparedSourceV2Dependencies,
    physical: bool,
    wrong: Option<&str>,
    stage: &Path,
    app: &ApplicationErofsTools,
) -> Result<MaterializedSourceV2Runtime, SourceV2MaterializationError> {
    let mut source = owner.source_manifest().manifest_blake3().to_owned();
    let size = owner.source_manifest().manifest_size_bytes();
    let mut selections = owner
        .dependencies()
        .iter()
        .map(|dependency| Selection {
            root: dependency.root().source_root.into(),
            layer: dependency.root().layer_name.into(),
            mount: dependency.mount_point().into(),
            target: dependency.root().build_runtime_target.into(),
            id: dependency.manifest().materialization_id().into(),
            digest: dependency.manifest().manifest_digest().into(),
            physical: physical.then(|| dependency.physical_blob().clone()),
        })
        .collect::<Vec<_>>();
    match wrong {
        Some("source") => source = "f".repeat(64),
        Some("missing") => {
            selections.pop();
        }
        Some("root") => selections[0].root = "other/node_modules".into(),
        Some("mount") => selections[0].mount = "/output/other/node_modules".into(),
        Some("manifest") => selections[0].digest = "f".repeat(64),
        Some("mixed") => selections[1].physical = None,
        Some("duplicate") => selections.push(selections[0].clone()),
        Some("layer") => selections[0].layer = "other".into(),
        Some("target") => selections[0].target = "node-22".into(),
        Some("id") => selections[0].id = "f".repeat(64),
        Some("physical") => selections[0].physical.as_mut().unwrap().digest.value = "f".repeat(64),
        _ => {}
    }
    let bindings = selections
        .iter()
        .map(|selection| DependencyGraphSelection {
            source_root: &selection.root,
            layer_name: &selection.layer,
            mount_point: &selection.mount,
            build_runtime_target: &selection.target,
            materialization_id: &selection.id,
            manifest_digest: &selection.digest,
            physical_blob: selection.physical.as_ref(),
        })
        .collect::<Vec<_>>();
    materialize_prepared_source_v2_runtime(
        owner,
        SourceV2DependencySelections {
            source_manifest_blake3: &source,
            source_manifest_size: size,
            dependencies: &bindings,
        },
        stage,
        app,
        app_limits(),
        runtime_limits(),
    )
}

#[test]
fn selected_physical_representation_drives_canonical_graph_and_runtime_keeps_images() {
    let roots = ["node_modules"];
    for physical in [false, true] {
        let mut fixture = fixture(&roots, false);
        let tools = tools(None);
        let stage = tempfile::tempdir().unwrap();
        let owner = prepared(&mut fixture, &tools, stage.path(), &roots);
        let image_identity = owner.dependencies()[0].image.file.metadata().unwrap();
        let app_stage = tempfile::tempdir().unwrap();
        let (app, calls) = app_tools(
            app_stage.path(),
            false,
            Retained::default(),
            Arc::new(AtomicUsize::new(0)),
        );
        let runtime = finish(owner, physical, None, app_stage.path(), &app).unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(runtime.graph().wire().dependencies.len(), 1);
        assert_eq!(
            runtime.graph().wire().dependencies[0]
                .physical_blob
                .is_some(),
            physical
        );
        let held = runtime.dependencies()[0].image.file.metadata().unwrap();
        assert_eq!(
            (held.dev(), held.ino()),
            (image_identity.dev(), image_identity.ino())
        );
        assert!(fixture.pool.reserved_bytes() > 0);
        drop(runtime);
        assert_eq!(fixture.pool.reserved_bytes(), 0);
        assert_eq!(fs::read_dir(stage.path()).unwrap().count(), 0);
    }
}

#[test]
fn wrong_selections_fail_closed_before_app_tools() {
    let roots = ["node_modules"];
    for wrong in [
        "source",
        "missing",
        "root",
        "mount",
        "manifest",
        "physical",
        "duplicate",
        "layer",
        "target",
        "id",
        "mixed",
    ] {
        let case_roots = if matches!(wrong, "mixed" | "duplicate") {
            vec!["node_modules", "other/node_modules"]
        } else {
            roots.to_vec()
        };
        let mut fixture = fixture(&case_roots, false);
        let tools = tools(None);
        let stage = tempfile::tempdir().unwrap();
        let owner = prepared(&mut fixture, &tools, stage.path(), &case_roots);
        let app_stage = tempfile::tempdir().unwrap();
        let (app, calls) = app_tools(
            app_stage.path(),
            false,
            Retained::default(),
            Arc::new(AtomicUsize::new(0)),
        );
        let result = finish(owner, true, Some(wrong), app_stage.path(), &app);
        assert!(
            matches!(
                result,
                Err(SourceV2MaterializationError::Dependency(
                    "physical selections differ from prepared dependencies"
                ))
            ),
            "{wrong}"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert_eq!(fixture.pool.reserved_bytes(), 0);
    }
}

#[test]
fn pending_app_tool_keeps_dependency_owner_and_existing_source_handoff() {
    let roots = ["node_modules"];
    let mut fixture = fixture(&roots, false);
    let tools = tools(None);
    let stage = tempfile::tempdir().unwrap();
    let owner = prepared(&mut fixture, &tools, stage.path(), &roots);
    let image = owner.storage._staging.path().join("images/0.erofs");
    let executor = tools.executor.clone();
    let retained = Retained::default();
    let closed = Arc::new(AtomicUsize::new(0));
    let app_stage = tempfile::tempdir().unwrap();
    let (app, calls) = app_tools(app_stage.path(), true, retained.clone(), closed.clone());
    let error = finish(owner, true, None, app_stage.path(), &app)
        .err()
        .unwrap();
    assert!(
        matches!(&error,SourceV2MaterializationError::Package(crate::application_package::ApplicationMaterializationError::Tool(tool)) if tool.cleanup_pending())
    );
    drop(error);
    drop(app);
    drop(tools);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(closed.load(Ordering::Relaxed), 1);
    assert!(fixture.pool.reserved_bytes() > 0);
    assert!(image.exists());
    assert!(executor.upgrade().is_some());
    retained.lock().unwrap().clear();
    assert_eq!(fixture.pool.reserved_bytes(), 0);
    assert!(!image.exists());
    assert!(executor.upgrade().is_none());
}
