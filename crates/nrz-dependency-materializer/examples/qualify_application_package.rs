//! Explicit local qualification only: this executor provides no production sandbox.
//! Run with absolute mkfs/fsck paths followed by their expected lowercase SHA-256s.

use std::{
    error::Error,
    fs,
    io::Read,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    process::Command,
    sync::Arc,
};

use nrz_contract::application_package_v1::Role;
use nrz_dependency_materializer::{
    application_erofs_tools::{
        ApplicationErofsToolExecutor, ApplicationErofsToolOutput, ApplicationErofsTools,
        ApplicationToolInvocation, ToolError,
    },
    application_package::ApplicationMaterializationLimits,
    source_v2_materialization::materialize_source_v2_runtime,
    source_v2_preparation::{PreparationLimits, prepare_source_v2},
    source_v2_runtime::RuntimeVerificationLimits,
};
use nrz_runtime_artifact::{
    ArtifactRoot,
    application_package_v1::{ApplicationPackagePolicy, validate_application_package_descriptor},
    source_graph_v3::LayerBuildTarget,
};
use nrz_source_bundle::{
    PathWorkBudget,
    source_bundle_v2::{
        SourceBundleV2ManifestBudget, admission::ManifestMemoryPool,
        encode_source_bundle_v2_manifest, static_projection::PreparedStaticProjection,
    },
};
use serde_json::{Value, json};

const MIB: u64 = 1024 * 1024;
const PAYLOAD_BYTES: usize = 256 * 1024;

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    manifest: String,
    packages: Vec<Vec<u8>>,
}

struct TrustedQualificationExecutor;

impl ApplicationErofsToolExecutor for TrustedQualificationExecutor {
    fn execute(
        &self,
        invocation: ApplicationToolInvocation<'_>,
    ) -> Result<ApplicationErofsToolOutput, ToolError> {
        // Only these small, locally generated fixtures and explicitly pinned tools run here.
        let output = Command::new(invocation.executable)
            .args(invocation.arguments)
            .output()
            .map_err(|error| ToolError::new(error.to_string()))?;
        Ok(ApplicationErofsToolOutput {
            success: output.status.success(),
            status: output.status.to_string(),
            stderr: output.stderr,
        })
    }
}

fn regular(path: &str, role: &str, bytes: &[u8]) -> Value {
    let mut file = json!({"path":path, "entryType":"file", "role":role,
        "blobDigest":{"algorithm":"BLAKE3", "value":blake3::hash(bytes).to_hex().as_str()},
        "size":bytes.len(), "executable":false});
    if matches!(role, "compute" | "prerender") {
        file["layerName"] = json!("server");
    }
    file
}

fn run(
    tools: &ApplicationErofsTools,
    fsck: &Path,
    input: &Path,
    staging: &Path,
    main: &[u8],
    route: &str,
) -> Result<Snapshot, Box<dyn Error>> {
    fs::write(input.join("app/main.js"), main)?;
    let payload = vec![b'z'; PAYLOAD_BYTES];
    let mut files = vec![
        regular("app/main.js", "compute", main),
        json!({"path":"app/static-link", "entryType":"symlink", "role":"compute",
            "layerName":"server", "linkTarget":"../web/index.html", "size":0}),
        json!({"path":"web/alias", "entryType":"symlink", "role":"static",
            "linkTarget":"index.html", "size":0}),
        json!({"path":"web/assets-alias", "entryType":"symlink", "role":"static",
            "linkTarget":"assets", "size":0}),
        json!({"path":"web/self", "entryType":"symlink", "role":"static",
            "linkTarget":".", "size":0}),
        json!({"path":"web/bounce", "entryType":"symlink", "role":"static",
            "linkTarget":"self/../web/self", "size":0}),
        regular("web/assets/repeated.txt", "static", &payload),
        regular("web/config.json", "config", b"{}"),
        regular("web/empty", "prerender", b""),
        regular("web/index.html", "static", b"<h1>fixture</h1>"),
        json!({"path":"web/private-link", "entryType":"symlink", "role":"static",
            "linkTarget":"../app/main.js", "size":0}),
    ];
    files
        .iter_mut()
        .find(|file| file["path"] == "web/index.html")
        .unwrap()["contentType"] = json!("text/x-fixture");
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 32,
        max_logical_bytes: MIB,
        max_static_file_bytes: MIB,
    };
    let bytes = encode_source_bundle_v2_manifest(
        json!({
            "schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":["COMPUTE_NETWORK_V1"],
            "files":files, "layers":[
                {"name":"web", "target":"STATIC", "rootPath":"web"},
                {"name":"server", "target":"COMPUTE", "rootPath":"app", "entrypoint":"app/main.js",
                 "runtimeConfig":{"applicationRuntime":{"family":"BUN", "args":[]},
                    "buildRuntimeVersion":"bun-1.4.2", "memoryMb":256,
                    "network":nrz_source_bundle::ComputeNetwork::public_default()}}
            ], "routes":[{"pattern":route, "layerName":"web"}, {"pattern":"/api", "layerName":"server"}], "entrypoints":["app/main.js"]
        }),
        budget,
    )?;
    let pool = ManifestMemoryPool::new(16 * MIB);
    let mut body = pool.reserve_input(bytes.len())?;
    body.append(&bytes)?;
    let admitted = body.verify(budget, 100_000)?;
    let projection = admitted.prepare_static_projection(1_000_000)?;
    let reserved = pool.reserved_bytes();
    assert!(reserved > projection.reserved_bytes());
    let prepared = prepare_source_v2(
        admitted,
        &ArtifactRoot::open(input)?,
        staging,
        PreparationLimits {
            max_file_bytes: MIB,
            max_nodes: 64,
            max_path_bytes: 1024,
        },
    )?;
    assert_eq!(pool.reserved_bytes(), reserved);
    let runtime = materialize_source_v2_runtime(
        prepared,
        staging,
        tools,
        &[LayerBuildTarget {
            layer_name: "server",
            build_runtime_target: "bun-1.4.2",
        }],
        &[],
        ApplicationMaterializationLimits {
            max_package_bytes: MIB,
            max_staging_file_bytes: 8 * MIB,
            max_staging_nodes: 128,
            max_path_bytes: 1024,
            max_verification_memory_bytes: 2 * MIB,
        },
        RuntimeVerificationLimits {
            max_metadata_bytes: 1024 * 1024,
            max_native_file_bytes: MIB,
            max_native_total_bytes: 2 * MIB,
            max_path_nodes: 64,
            max_path_work: 100_000,
        },
    )?;
    let packages = runtime.packages();
    assert_eq!(
        runtime.graph().wire().application.manifest.digest.value,
        packages.manifest().manifest_blake3()
    );
    assert_eq!(runtime.graph().wire().application.components.len(), 2);
    assert_eq!(pool.reserved_bytes(), reserved);
    // Successful materialization retains packages, not the prepared source copy.
    assert_eq!(fs::read_dir(staging)?.count(), 1);
    let manifest = packages.manifest().manifest_blake3().to_owned();
    let policy = ApplicationPackagePolicy {
        canonicalization_profile:
            nrz_dependency_materializer::application_package::APPLICATION_EROFS_PROFILE_V1.into(),
        generator_identity: tools.generator_identity().into(),
    };
    let mut contents = Vec::new();
    let mut static_erofs_size = 0;
    assert_eq!(packages.descriptors().len(), 2);
    for (expected_role, descriptor) in [Role::Static, Role::Compute]
        .into_iter()
        .zip(packages.descriptors())
    {
        let wire = descriptor.wire();
        assert_eq!(wire.role, expected_role);
        validate_application_package_descriptor(wire, &policy)?;
        let mut bytes = Vec::new();
        packages
            .open_package(expected_role)?
            .take(MIB + 1)
            .read_to_end(&mut bytes)?;
        assert_eq!(bytes.len() as u64, wire.blob.size);
        assert!(wire.blob.size <= MIB);
        assert_eq!(
            blake3::hash(&bytes).to_hex().as_str(),
            wire.blob.digest.value
        );
        if expected_role == Role::Static {
            assert!(wire.erofs_size < PAYLOAD_BYTES as u64);
            static_erofs_size = usize::try_from(wire.erofs_size)?;
        }
        contents.push(bytes);
    }
    assert_eq!(pool.reserved_bytes(), reserved);
    drop(runtime);
    assert_eq!(pool.reserved_bytes(), projection.reserved_bytes());
    assert_eq!(projection.manifest_blake3(), manifest);
    qualify_static_opening(fsck, &projection, &contents[0][..static_erofs_size])?;
    drop(projection);
    assert_eq!(pool.reserved_bytes(), 0);
    assert_eq!(fs::read_dir(staging)?.count(), 0);
    Ok(Snapshot {
        manifest,
        packages: contents,
    })
}

fn qualify_static_opening(
    fsck: &Path,
    projection: &PreparedStaticProjection,
    image: &[u8],
) -> Result<(), Box<dyn Error>> {
    let scratch = tempfile::Builder::new()
        .prefix("nrz-static-opening-qualification-")
        .tempdir_in("/var/tmp")?;
    let image_path = scratch.path().join("static.erofs");
    let extracted = scratch.path().join("static");
    fs::write(&image_path, image)?;
    fs::create_dir(&extracted)?;
    let output = Command::new(fsck)
        .arg(format!("--extract={}", extracted.display()))
        .args(["--no-preserve-owner", "--preserve-perms"])
        .arg(&image_path)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let root = ArtifactRoot::open(&extracted)?;
    assert_eq!(
        root.file_type(Path::new("app"), false).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert_eq!(
        root.open_file(Path::new("app/static-link"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    let mut work = PathWorkBudget::new(100_000);
    let mut requests = vec![
        ("app/static-link", Some("web/index.html")),
        ("web/private-link", None),
        (
            "web/assets-alias/repeated.txt",
            Some("web/assets/repeated.txt"),
        ),
        ("web/assets-alias", None),
        ("web/index.html/suffix", None),
    ]
    .into_iter()
    .map(|(request, expected)| (request.to_owned(), expected))
    .collect::<Vec<_>>();
    for (alias, count, visible) in [
        ("self", 40, true),
        ("self", 41, false),
        ("bounce", 13, true),
        ("bounce", 14, false),
    ] {
        let suffix = format!("{alias}/").repeat(count);
        let requested = format!("web/{suffix}index.html");
        let native = fs::read(extracted.join(&requested));
        if visible {
            assert_eq!(native?, b"<h1>fixture</h1>");
        } else {
            assert_eq!(native.unwrap_err().raw_os_error(), Some(libc::ELOOP));
        }
        requests.push((requested, visible.then_some("web/index.html")));
    }
    for (requested, expected_target) in requests {
        let metadata = projection.lookup(&requested, &mut work)?;
        assert_eq!(metadata.map(|file| file.canonical_path()), expected_target);
        let Some(metadata) = metadata else {
            continue;
        };
        let canonical = metadata.canonical_path();
        assert_ne!(requested, canonical);
        let mut file = root.open_file(Path::new(canonical))?;
        let on_disk = file.metadata()?;
        assert!(on_disk.is_file());
        assert_eq!(on_disk.len(), metadata.size());
        assert_eq!(on_disk.permissions().mode() & 0o777, 0o775);
        let mut bytes = Vec::new();
        file.by_ref()
            .take(metadata.size() + 1)
            .read_to_end(&mut bytes)?;
        assert_eq!(bytes.len() as u64, metadata.size());
        assert_eq!(
            blake3::hash(&bytes).to_hex().as_str(),
            metadata.blob_digest().value
        );
        if canonical == "web/index.html" {
            assert_eq!(bytes, b"<h1>fixture</h1>");
            assert_eq!(metadata.content_type(), Some("text/x-fixture"));
        } else {
            assert_eq!(bytes, vec![b'z'; PAYLOAD_BYTES]);
            assert_eq!(metadata.content_type(), None);
        }
    }
    drop(root);
    let scratch_path = scratch.path().to_owned();
    scratch.close()?;
    assert!(!scratch_path.exists());
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: qualify_application_package MKFS_PATH FSCK_PATH MKFS_SHA256 FSCK_SHA256".into(),
        );
    }
    let tools = ApplicationErofsTools::new(
        args[0].clone().into(),
        args[1].clone().into(),
        args[2].to_str().ok_or("MKFS_SHA256 must be UTF-8")?,
        args[3].to_str().ok_or("FSCK_SHA256 must be UTF-8")?,
        Arc::new(TrustedQualificationExecutor),
    )?;
    let fixture = tempfile::Builder::new()
        .prefix("nrz-application-qualification-")
        .tempdir_in("/var/tmp")?;
    let input = fixture.path().join("input");
    let staging = fixture.path().join("staging");
    fs::create_dir_all(input.join("app"))?;
    fs::create_dir_all(input.join("web/assets"))?;
    fs::create_dir(&staging)?;
    fs::write(
        input.join("web/assets/repeated.txt"),
        vec![b'z'; PAYLOAD_BYTES],
    )?;
    fs::write(input.join("web/config.json"), b"{}")?;
    fs::write(input.join("web/empty"), b"")?;
    fs::write(input.join("web/index.html"), b"<h1>fixture</h1>")?;
    symlink("index.html", input.join("web/alias"))?;
    symlink("assets", input.join("web/assets-alias"))?;
    symlink(".", input.join("web/self"))?;
    symlink("self/../web/self", input.join("web/bounce"))?;
    symlink("../app/main.js", input.join("web/private-link"))?;
    symlink("../web/index.html", input.join("app/static-link"))?;
    let fsck = Path::new(&args[1]);
    let baseline = run(&tools, fsck, &input, &staging, b"export default 1;", "/")?;
    assert_eq!(
        baseline,
        run(&tools, fsck, &input, &staging, b"export default 1;", "/")?
    );
    let route_only = run(
        &tools,
        fsck,
        &input,
        &staging,
        b"export default 1;",
        "/changed",
    )?;
    assert_ne!(baseline.manifest, route_only.manifest);
    assert_eq!(baseline.packages, route_only.packages);
    let compute_changed = run(&tools, fsck, &input, &staging, b"export default 2;", "/")?;
    assert_ne!(baseline.manifest, compute_changed.manifest);
    assert_eq!(baseline.packages[0], compute_changed.packages[0]);
    assert_ne!(baseline.packages[1], compute_changed.packages[1]);
    let fixture_path = fixture.path().to_owned();
    fixture.close()?;
    assert!(!fixture_path.exists());
    println!(
        "PASS: STATIC/COMPUTE content, compression, deterministic bytes, role reuse, canonical STATIC opening, reservation lifetime and cleanup"
    );
    println!(
        "Local trusted-tool qualification only; production sandbox and host mount qualification remain separate."
    );
    Ok(())
}
