use super::{RuntimeArtifactScan, native_dependencies::*, source_bundle_v1::*};
use crate::deploy::scan_dir;
use serde_json::json;
use std::{fs, path::Path};

fn package(root: &Path, path: &str, metadata: serde_json::Value) {
    let path = root.join(path);
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("package.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    fs::write(path.join("index.js"), "module.exports = 'ready'").unwrap();
}

fn target() -> RuntimePlatform {
    RuntimePlatform::from_values("linux", "x86_64", "glibc").unwrap()
}

#[test]
fn prunes_only_incompatible_optional_packages_before_manifest_identity_and_size() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        "",
        json!({"optionalDependencies":{"native-glibc":"1", "native-musl":"1", "unknown":"1"}}),
    );
    package(
        dir.path(),
        "node_modules/native-glibc",
        json!({"os":"linux","cpu":["x64"],"libc":["glibc"]}),
    );
    package(
        dir.path(),
        "node_modules/native-musl",
        json!({"os":["linux"],"cpu":"x64","libc":"musl"}),
    );
    package(dir.path(), "node_modules/unknown", json!({}));
    package(
        dir.path(),
        "node_modules/unreferenced",
        json!({"os":"darwin"}),
    );
    fs::write(dir.path().join("server.js"), "require('native-glibc')").unwrap();
    let before = scan_dir(dir.path()).unwrap();
    let original_bytes: u64 = before.iter().map(|file| file.size).sum();
    let result = prune_optional_native_dependencies(dir.path(), before, &target()).unwrap();
    assert_eq!(result.packages, 1);
    assert!(
        result
            .files
            .iter()
            .any(|file| file.path == "node_modules/native-glibc/index.js")
    );
    assert!(
        result
            .files
            .iter()
            .any(|file| file.path == "node_modules/unreferenced/index.js")
    );
    assert!(
        result
            .files
            .iter()
            .all(|file| !file.path.starts_with("node_modules/native-musl/"))
    );
    let manifest = serde_json::from_value(json!({"version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"server.js"}],"routes":[]})).unwrap();
    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &result.files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    assert_eq!(
        plan.logical_manifest
            .files
            .iter()
            .map(|file| file.size)
            .sum::<u64>(),
        original_bytes - result.bytes
    );
    assert!(
        plan.logical_manifest
            .files
            .iter()
            .all(|file| !file.path.starts_with("node_modules/native-musl/"))
    );
    // The source workspace is immutable; only the sealed file inventory changes.
    assert!(
        dir.path()
            .join("node_modules/native-musl/index.js")
            .exists()
    );
}

#[test]
fn required_and_optional_override_edges_match_npm_semantics() {
    for (metadata, should_fail) in [
        (json!({"dependencies":{"native":"1"}}), true),
        (
            json!({"dependencies":{"native":"1"},"optionalDependencies":{"native":"1"}}),
            false,
        ),
        (json!({"peerDependencies":{"native":"1"}}), true),
        (
            json!({"peerDependencies":{"native":"1"},"peerDependenciesMeta":{"native":{"optional":true}}}),
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        package(dir.path(), "", metadata);
        package(dir.path(), "node_modules/native", json!({"os":["!linux"]}));
        let result = prune_optional_native_dependencies(
            dir.path(),
            scan_dir(dir.path()).unwrap(),
            &target(),
        );
        assert_eq!(result.is_err(), should_fail);
        if should_fail {
            let error = result.err().unwrap();
            assert_eq!(
                error
                    .downcast_ref::<crate::output::CodedError>()
                    .unwrap()
                    .code,
                "RUNTIME_DEPENDENCY_INCOMPATIBLE"
            );
        } else {
            assert_eq!(result.unwrap().packages, 1);
        }
    }
}

#[test]
fn platform_allow_lists_negation_and_unknown_metadata_are_conservative() {
    for (constraint, removed) in [
        (json!("linux"), 0),
        (json!(["any"]), 0),
        (json!(["linux", "!linux"]), 1),
        (json!(["!darwin"]), 0),
        (json!(["darwin", "win32"]), 1),
        (json!([]), 0),
        (json!([1]), 0),
    ] {
        let dir = tempfile::tempdir().unwrap();
        package(
            dir.path(),
            "",
            json!({"optionalDependencies":{"native":"1"}}),
        );
        package(dir.path(), "node_modules/native", json!({"os":constraint}));
        let result = prune_optional_native_dependencies(
            dir.path(),
            scan_dir(dir.path()).unwrap(),
            &target(),
        )
        .unwrap();
        assert_eq!(result.packages, removed);
    }
}

#[cfg(unix)]
#[test]
fn pnpm_aliases_do_not_leave_dangling_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        "",
        json!({"optionalDependencies":{"native":"1"}}),
    );
    package(
        dir.path(),
        "node_modules/.pnpm/native@1/node_modules/native",
        json!({"libc":"musl"}),
    );
    std::os::unix::fs::symlink(
        ".pnpm/native@1/node_modules/native",
        dir.path().join("node_modules/native"),
    )
    .unwrap();
    let result =
        prune_optional_native_dependencies(dir.path(), scan_dir(dir.path()).unwrap(), &target())
            .unwrap();
    assert_eq!(result.packages, 1);
    assert!(
        result
            .files
            .iter()
            .all(|file| !file.path.starts_with("node_modules/"))
    );
}

#[cfg(unix)]
#[test]
fn shared_descendants_and_their_aliases_survive_archive_extraction() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        "",
        json!({"optionalDependencies":{"native":"1"}, "dependencies":{"shared":"1"}}),
    );
    package(dir.path(), "node_modules/native", json!({"libc":"musl"}));
    package(
        dir.path(),
        "node_modules/native/node_modules/shared",
        json!({}),
    );
    std::os::unix::fs::symlink(
        "native/node_modules/shared",
        dir.path().join("node_modules/shared"),
    )
    .unwrap();
    let result =
        prune_optional_native_dependencies(dir.path(), scan_dir(dir.path()).unwrap(), &target())
            .unwrap();
    assert_eq!(result.packages, 0);
    let manifest = serde_json::from_value(json!({"version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"index.js"}],"routes":[]})).unwrap();
    let plan = build_source_bundle_plan_with_scan(
        dir.path(),
        &manifest,
        &result.files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let unpacked = tempfile::tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(fs::File::open(plan.source_path()).unwrap()).unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
    assert_eq!(
        fs::read_to_string(unpacked.path().join("node_modules/shared/index.js")).unwrap(),
        "module.exports = 'ready'"
    );
}

#[cfg(unix)]
#[test]
fn an_application_alias_to_an_incompatible_optional_package_is_a_coded_failure() {
    let dir = tempfile::tempdir().unwrap();
    package(
        dir.path(),
        "",
        json!({"optionalDependencies":{"native":"1"}}),
    );
    package(dir.path(), "node_modules/native", json!({"libc":"musl"}));
    std::os::unix::fs::symlink("node_modules/native/index.js", dir.path().join("native.js"))
        .unwrap();
    let error =
        prune_optional_native_dependencies(dir.path(), scan_dir(dir.path()).unwrap(), &target())
            .err()
            .unwrap();
    assert_eq!(
        error
            .downcast_ref::<crate::output::CodedError>()
            .unwrap()
            .code,
        "RUNTIME_DEPENDENCY_INCOMPATIBLE"
    );
}

#[test]
#[ignore = "requires NRZ_NATIVE_FIXTURE_ROOT with sharp and both glibc/musl optional packages, plus node"]
fn real_sharp_loads_and_encodes_an_image_from_the_pruned_source_archive() {
    let fixture = std::path::PathBuf::from(
        std::env::var_os("NRZ_NATIVE_FIXTURE_ROOT").expect("native fixture"),
    );
    let files = scan_dir(&fixture).unwrap();
    let result = prune_optional_native_dependencies(&fixture, files, &target()).unwrap();
    assert!(
        result.packages >= 2,
        "fixture must include both incompatible sharp and libvips packages"
    );
    let manifest = serde_json::from_value(json!({"version":1,"layers":[{"name":"server","target":"COMPUTE","directory":".","entry":"server.js"}],"routes":[]})).unwrap();
    let plan = build_source_bundle_plan_with_scan(
        &fixture,
        &manifest,
        &result.files,
        &RuntimeArtifactScan::NodeRuntimeRoot,
        RuntimeDependencyPackaging::TrustedMaterialization,
        None,
    )
    .unwrap();
    let unpacked = tempfile::tempdir().unwrap();
    let decoder =
        zstd::stream::read::Decoder::new(fs::File::open(plan.source_path()).unwrap()).unwrap();
    tar::Archive::new(decoder).unpack(unpacked.path()).unwrap();
    let output = std::process::Command::new("node")
        .arg("server.js")
        .current_dir(unpacked.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "sharp-runtime-ok"
    );
}
