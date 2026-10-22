use super::*;
use crate::source_bundle_v2::{
    SourceBundleV2ManifestBudget, admission::ManifestMemoryPool, encode_source_bundle_v2_manifest,
};
use serde_json::{Value, json};

fn file(path: &str, role: &str) -> Value {
    let mut value = json!({"path":path,"entryType":"file","role":role,"blobDigest":{"algorithm":"BLAKE3","value":"1".repeat(64)},"size":7,"executable":false,"contentType":"text/x-target"});
    if matches!(role, "compute" | "prerender") {
        value["layerName"] = json!("server");
    }
    value
}

fn link(path: &str, role: &str, target: &str) -> Value {
    let mut value =
        json!({"path":path,"entryType":"symlink","role":role,"linkTarget":target,"size":0});
    if role == "compute" {
        value["layerName"] = json!("server");
    }
    value
}

fn admit(mut files: Vec<Value>, pool: &ManifestMemoryPool) -> AdmittedSourceBundleV2Manifest {
    files.push(file("server/main.js", "compute"));
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 10_000,
        max_logical_bytes: 1024 * 1024,
        max_static_file_bytes: 1024 * 1024,
    };
    let bytes = encode_source_bundle_v2_manifest(json!({
        "schemaVersion":"SOURCE_BUNDLE_V2.0","capabilities":["COMPUTE_NETWORK_V1"],"files":files,
        "layers":[{"name":"server","target":"COMPUTE","rootPath":"server","entrypoint":"server/main.js",
          "runtimeConfig":{"applicationRuntime":{"family":"BUN","args":[]},"buildRuntimeVersion":"bun-1.4.2","memoryMb":256,"network":crate::ComputeNetwork::public_default()}}],
        "routes":[{"pattern":"/api","layerName":"server"}],"entrypoints":["server/main.js"]
    }), budget).unwrap();
    let mut input = pool.reserve_input(bytes.len()).unwrap();
    input.append(&bytes).unwrap();
    input.verify(budget, u64::MAX).unwrap()
}

fn lookup<'a>(
    projection: &'a PreparedStaticProjection,
    path: &str,
) -> Option<&'a StaticFileMetadata> {
    projection
        .lookup(path, &mut PathWorkBudget::new(100_000))
        .unwrap()
}

#[test]
fn visibility_and_canonical_targets_survive_manifest_drop() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let admitted = admit(
        vec![
            file("d/file", "static"),
            file("server/vault/private", "compute"),
            file("server/only-private/file", "compute"),
            link("server/public", "compute", "../d/file"),
            link("d/private", "static", "../server/main.js"),
            link("d/vault", "static", "../server/vault"),
            link("d/only-private", "static", "../server/only-private"),
            link("server/vault/public", "compute", "../../d/file"),
            link("d/regular", "static", "file"),
            link("alias", "static", "d"),
            file("d/ёж", "static"),
            file("d/config", "config"),
            file("d/prerender", "prerender"),
        ],
        &pool,
    );
    let projection = admitted.prepare_static_projection(1_000_000).unwrap();
    let identity = admitted.verified().manifest_blake3().to_owned();
    drop(admitted);
    assert_eq!(projection.manifest_blake3(), identity);
    assert_eq!(pool.reserved_bytes(), projection.reserved_bytes());
    for path in ["server/public", "alias/file", "d/vault/public", "d/regular"] {
        let found = lookup(&projection, path).unwrap();
        assert_eq!(found.canonical_path(), "d/file");
        assert_eq!(found.size(), 7);
        assert_eq!(found.blob_digest().value, "1".repeat(64));
        assert_eq!(found.content_type(), Some("text/x-target"));
    }
    assert_eq!(
        lookup(&projection, "alias/ёж").unwrap().canonical_path(),
        "d/ёж"
    );
    for (request, target) in [
        ("d/config", "d/config"),
        ("alias/config", "d/config"),
        ("d/prerender", "d/prerender"),
        ("alias/prerender", "d/prerender"),
    ] {
        assert_eq!(
            lookup(&projection, request).unwrap().canonical_path(),
            target
        );
    }
    for path in [
        "server/main.js",
        "d/private",
        "d/only-private/file",
        "d/vault/private",
        "d/regular/suffix",
        "d/file/suffix",
        "d",
        "alias",
        "d/only-private",
        "d/vault",
        "missing/deep/suffix",
    ] {
        assert!(lookup(&projection, path).is_none(), "{path}");
    }
    drop(projection);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn deadness_search_handles_siblings_before_the_directory_prefix() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let admitted = admit(
        vec![
            file("a-b/file", "static"),
            file("a./file", "static"),
            file("a/file", "static"),
            link("alias", "static", "a"),
        ],
        &pool,
    );
    let projection = admitted.prepare_static_projection(1_000_000).unwrap();
    assert_eq!(
        lookup(&projection, "alias/file").unwrap().canonical_path(),
        "a/file"
    );
    assert!(lookup(&projection, "alias-b/file").is_none());
}

#[test]
fn repeated_aliases_count_every_actual_expansion_across_suffixes() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let admitted = admit(
        vec![
            file("d/file", "static"),
            link("d/self", "static", "."),
            link("d/bounce", "static", "self/../d/self"),
            link("entry", "static", "d/self"),
        ],
        &pool,
    );
    let projection = admitted.prepare_static_projection(1_000_000).unwrap();
    #[cfg(target_os = "linux")]
    let native = {
        use std::{fs, os::unix::fs::symlink};
        let native = tempfile::tempdir().unwrap();
        fs::create_dir(native.path().join("d")).unwrap();
        fs::write(native.path().join("d/file"), b"fixture").unwrap();
        symlink(".", native.path().join("d/self")).unwrap();
        symlink("self/../d/self", native.path().join("d/bounce")).unwrap();
        symlink("d/self", native.path().join("entry")).unwrap();
        native
    };
    for (request, visible) in [
        (format!("d/{}file", "self/".repeat(40)), true),
        (format!("d/{}file", "self/".repeat(41)), false),
        (format!("d/{}file", "bounce/".repeat(13)), true),
        (format!("d/{}file", "bounce/".repeat(14)), false),
        (format!("entry/{}file", "self/".repeat(38)), true),
        (format!("entry/{}file", "self/".repeat(39)), false),
    ] {
        assert_eq!(
            lookup(&projection, &request).is_some(),
            visible,
            "{request}"
        );
        #[cfg(target_os = "linux")]
        {
            let opened = std::fs::read(native.path().join(&request));
            if visible {
                assert_eq!(
                    opened.unwrap_or_else(|error| panic!("{request}: {error}")),
                    b"fixture"
                );
            } else {
                assert_eq!(opened.unwrap_err().raw_os_error(), Some(40));
            }
        }
    }
    let bounce = find(
        &projection.nodes,
        "d/bounce",
        &mut PathWorkBudget::unlimited(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(projection.nodes[bounce].alias.unwrap().1, 3);
}

#[test]
fn fan_in_shares_one_long_canonical_target() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let target = "z".repeat(3000);
    let mut files = vec![file(&target, "static"), link("shared", "static", &target)];
    for number in 0..1001 {
        files.push(link(&format!("alias-{number:04}"), "static", "shared"));
    }
    let admitted = admit(files, &pool);
    let projection = admitted.prepare_static_projection(100_000_000).unwrap();
    assert_eq!(projection.targets.len(), 1);
    let first = lookup(&projection, "alias-0000").unwrap();
    let last = lookup(&projection, "alias-1000").unwrap();
    assert_eq!(first.canonical_path(), target);
    assert_eq!(
        first.canonical_path().as_ptr(),
        last.canonical_path().as_ptr()
    );
    assert_eq!(projection.targets[0].path.len(), 3000);
}

#[test]
fn new_generation_refusal_and_last_reader_share_the_input_pool() {
    let fixture = || vec![file("d/file", "static"), link("alias", "static", "d")];
    let probe_pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let probe = admit(fixture(), &probe_pool);
    let counts = memory::Counts::inspect(
        &probe.verified().wire().files,
        &mut PathWorkBudget::unlimited(),
    )
    .unwrap();
    let scratch = counts.scratch_bytes().unwrap();
    let one = probe.prepare_static_projection(u64::MAX).unwrap();
    let pool = ManifestMemoryPool::new(probe.reserved_bytes() + scratch + one.reserved_bytes());
    let admitted = admit(fixture(), &pool);
    let projection = Arc::new(admitted.prepare_static_projection(u64::MAX).unwrap());
    let retained = pool.reserved_bytes();
    assert!(matches!(
        admitted.prepare_static_projection(u64::MAX),
        Err(StaticProjectionError::MemoryUnavailable { .. })
    ));
    assert_eq!(pool.reserved_bytes(), retained);
    let reader = Arc::clone(&projection);
    drop(admitted);
    drop(projection);
    assert_eq!(pool.reserved_bytes(), reader.reserved_bytes());
    assert_eq!(
        lookup(&reader, "alias/file").unwrap().canonical_path(),
        "d/file"
    );
    drop(reader);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn fuel_refusal_and_invalid_requests_leave_no_new_reservation() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let admitted = admit(vec![file("file", "static")], &pool);
    let before = pool.reserved_bytes();
    assert!(matches!(
        admitted.prepare_static_projection(0),
        Err(StaticProjectionError::PathWorkLimit)
    ));
    assert_eq!(pool.reserved_bytes(), before);
    let mut refused_after_preflight = false;
    for fuel in [700, 800, 900] {
        let preflight = memory::Counts::inspect(
            &admitted.verified().wire().files,
            &mut PathWorkBudget::new(fuel),
        );
        let result = admitted.prepare_static_projection(fuel);
        refused_after_preflight |=
            preflight.is_ok() && matches!(result, Err(StaticProjectionError::PathWorkLimit));
        drop(result);
        assert_eq!(pool.reserved_bytes(), before);
    }
    assert!(
        refused_after_preflight,
        "exercise cleanup after scratch admission"
    );
    let projection = admitted.prepare_static_projection(100_000).unwrap();
    for path in [
        "",
        "/file",
        "a//file",
        "a/./file",
        "a/../file",
        "back\\slash",
        "nul\0",
    ] {
        assert_eq!(
            projection
                .lookup(path, &mut PathWorkBudget::new(0))
                .unwrap_err(),
            StaticProjectionError::PathWorkLimit
        );
        assert_eq!(
            projection
                .lookup(path, &mut PathWorkBudget::new(100_000))
                .unwrap_err(),
            StaticProjectionError::InvalidRequestPath
        );
    }
}

#[test]
fn estimates_reject_overflow() {
    let counts = memory::Counts::default();
    assert_eq!(
        counts.owned_bytes(u64::MAX, 0).unwrap_err(),
        StaticProjectionError::UnsupportedEstimate
    );
    assert_eq!(
        counts.owned_bytes(0, u64::MAX).unwrap_err(),
        StaticProjectionError::UnsupportedEstimate
    );
    let mut counts = memory::Counts::inspect(&[], &mut PathWorkBudget::unlimited()).unwrap();
    counts.selected_nodes = u64::MAX;
    assert_eq!(
        counts.owned_bytes(0, 0).unwrap_err(),
        StaticProjectionError::UnsupportedEstimate
    );
    let mut counts = memory::Counts::inspect(&[], &mut PathWorkBudget::unlimited()).unwrap();
    counts.public_files = u64::MAX;
    counts.aliases = 1;
    assert_eq!(
        counts.scratch_bytes().unwrap_err(),
        StaticProjectionError::UnsupportedEstimate
    );
}

#[test]
fn deep_chain_work_charges_only_the_growing_nodes_children() {
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let target = format!("{}file", "d/".repeat(2048));
    let admitted = admit(
        vec![file(&target, "static"), link("alias", "static", &target)],
        &pool,
    );
    let projection = admitted.prepare_static_projection(1_000_000).unwrap();
    assert_eq!(
        lookup(&projection, "alias").unwrap().canonical_path(),
        target
    );
}
