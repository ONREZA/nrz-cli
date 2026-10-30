use super::*;
use crate::source_v2_preparation::{PreparationLimits, prepare_source_v2};
use nrz_runtime_artifact::ArtifactRoot;
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, admission::ManifestMemoryPool, encode_source_bundle_v2_manifest,
};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink};

fn limits() -> RuntimeVerificationLimits {
    RuntimeVerificationLimits {
        max_metadata_bytes: 1024 * 1024,
        max_native_file_bytes: 1024 * 1024,
        max_native_total_bytes: 4 * 1024 * 1024,
        max_path_nodes: 100,
        max_path_work: 100_000,
    }
}

#[test]
fn runtime_working_allowance_funds_native_index_and_managed_growth_separately() {
    use std::collections::HashSet;
    let contexts = [
        ManagedContext::Python,
        ManagedContext::Unlabelled,
        ManagedContext::Conflicting,
        ManagedContext::LinuxX64,
        ManagedContext::LinuxArm64,
        ManagedContext::DarwinX64,
        ManagedContext::DarwinArm64,
        ManagedContext::Win32X64,
        ManagedContext::Win32Arm64,
        ManagedContext::Win32Ia32,
    ];
    for files in [1_u64, 7, 64, 257, 1025] {
        let nodes = files * 2;
        let mut profile = limits();
        profile.max_path_nodes = nodes;
        if files == 1 {
            profile.max_native_file_bytes = 1;
        }
        let mut paths = HashSet::new();
        let mut directories = HashSet::new();
        let mut pending = Vec::new();
        for index in 0..files as usize {
            for context in contexts {
                paths.insert((index, context));
                pending.push((index, context));
            }
        }
        for index in 0..nodes as usize {
            for context in contexts {
                directories.insert((index, index + 1, context));
            }
        }
        let hash_growth = |capacity: usize, element: usize| {
            let buckets = (capacity + 1).next_power_of_two();
            (buckets + buckets / 2) * (element + 1) + 32
        };
        let managed = hash_growth(
            paths.capacity(),
            std::mem::size_of::<(usize, ManagedContext)>(),
        ) + hash_growth(
            directories.capacity(),
            std::mem::size_of::<(usize, usize, ManagedContext)>(),
        ) + 2 * pending.capacity() * std::mem::size_of::<(usize, ManagedContext)>();
        let index =
            nrz_source_bundle::SourceArchivePathIndex::working_memory_bytes(nodes, 4096).unwrap();
        let declared = profile.working_memory_bytes(files, 4096).unwrap();
        assert!(
            declared
                >= profile
                    .max_native_file_bytes
                    .min(profile.max_native_total_bytes)
                    .max(ADDON_HEADER_BYTES as u64)
                    + profile.max_metadata_bytes
                    + index
                    + managed as u64
        );
    }
    assert!(limits().working_memory_bytes(u64::MAX, 1).is_none());
    assert!(limits().working_memory_bytes(1, u64::MAX).is_none());
}

fn file(path: &str, role: &str, owner: Option<&str>) -> Value {
    let mut value = json!({"path":path, "entryType":"file", "role":role,
        "blobDigest":{"algorithm":"BLAKE3", "value":"a".repeat(64)},
        "size":1, "executable":false});
    if let Some(owner) = owner {
        value["layerName"] = json!(owner);
    }
    value
}

fn link(path: &str, target: &str, role: &str, owner: Option<&str>) -> Value {
    let mut value = file(path, role, owner);
    value["entryType"] = json!("symlink");
    value["linkTarget"] = json!(target);
    value["size"] = json!(0);
    value.as_object_mut().unwrap().remove("blobDigest");
    value.as_object_mut().unwrap().remove("executable");
    value
}

fn layer(name: &str, family: &str, target: &str, entry: &str) -> Value {
    json!({"name":name, "target":"COMPUTE", "entrypoint":entry,
        "runtimeConfig":{"applicationRuntime":{"family":family, "args":[]},
            "buildRuntimeVersion":target,
            "network":nrz_source_bundle::ComputeNetwork::public_default()}})
}

fn manifest(family: &str, target: &str, mut files: Vec<Value>) -> Value {
    files.insert(0, file("server", "compute", Some("app")));
    json!({"schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":["COMPUTE_NETWORK_V1"],
        "files":files, "layers":[layer("app", family, target, "server")],
        "routes":[{"pattern":"/", "layerName":"app"}], "entrypoints":["server"]})
}

struct Fixture {
    prepared: PreparedSourceV2,
    _staging: tempfile::TempDir,
}

impl std::ops::Deref for Fixture {
    type Target = PreparedSourceV2;
    fn deref(&self) -> &Self::Target {
        &self.prepared
    }
}

fn prepare(mut value: Value, payloads: &[(&str, &[u8])]) -> Fixture {
    let input = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    for entry in value["files"].as_array_mut().unwrap() {
        let path = entry["path"].as_str().unwrap().to_owned();
        let physical = input.path().join(&path);
        fs::create_dir_all(physical.parent().unwrap()).unwrap();
        if entry["entryType"] == "symlink" {
            symlink(entry["linkTarget"].as_str().unwrap(), physical).unwrap();
        } else {
            let bytes = payloads
                .iter()
                .find(|(name, _)| *name == path)
                .map_or(b"x".as_slice(), |(_, bytes)| *bytes);
            fs::write(physical, bytes).unwrap();
            entry["size"] = json!(bytes.len());
            entry["blobDigest"]["value"] = json!(blake3::hash(bytes).to_hex().as_str());
        }
    }
    value["files"]
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: 100,
        max_logical_bytes: 4 * 1024 * 1024,
        max_static_file_bytes: 4 * 1024 * 1024,
    };
    let bytes = encode_source_bundle_v2_manifest(value, budget).unwrap();
    let pool = ManifestMemoryPool::new(64 * 1024 * 1024);
    let mut input_bytes = pool.reserve_input(bytes.len()).unwrap();
    input_bytes.append(&bytes).unwrap();
    let admitted = input_bytes.verify(budget, 1_000_000).unwrap();
    let prepared = prepare_source_v2(
        admitted,
        &ArtifactRoot::open(input.path()).unwrap(),
        staging.path(),
        PreparationLimits {
            max_file_bytes: 4 * 1024 * 1024,
            max_nodes: 100,
            max_path_bytes: 4096,
        },
    )
    .unwrap();
    Fixture {
        prepared,
        _staging: staging,
    }
}

fn target(target: &str) -> [LayerBuildTarget<'_>; 1] {
    [LayerBuildTarget {
        layer_name: "app",
        build_runtime_target: target,
    }]
}

fn dependency<'a>(
    root: &'a str,
    target: &'a str,
    kind: DependencyMaterializationKind,
) -> AdmittedDependencyRoot<'a> {
    AdmittedDependencyRoot {
        layer_name: "app",
        source_root: root,
        kind,
        build_runtime_target: target,
    }
}

#[test]
fn independent_targets_are_required_exact_and_checked_against_source_intent() {
    let source = prepare(manifest("NODE", "node-24", vec![]), &[]);
    assert!(verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_ok());
    assert!(verify_prepared_source_v2_runtime(&source, &[], &[], limits()).is_err());
    assert!(verify_prepared_source_v2_runtime(&source, &target("node-26"), &[], limits()).is_err());
    let duplicated = [
        LayerBuildTarget {
            layer_name: "app",
            build_runtime_target: "node-24",
        },
        LayerBuildTarget {
            layer_name: "app",
            build_runtime_target: "node-24",
        },
    ];
    assert!(verify_prepared_source_v2_runtime(&source, &duplicated, &[], limits()).is_err());
    assert!(
        verify_prepared_source_v2_runtime(
            &source,
            &[LayerBuildTarget {
                layer_name: "unknown",
                build_runtime_target: "node-24"
            }],
            &[],
            limits()
        )
        .is_err()
    );
}

#[test]
fn dependency_roots_require_exact_independent_kind_target_and_file_ownership() {
    let source = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![file("node_modules/a/index.js", "dependency", Some("app"))],
        ),
        &[],
    );
    let policy = dependency(
        "node_modules",
        "node-24",
        DependencyMaterializationKind::JavaScriptNodeModules,
    );
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("node-24"), &[policy], limits()).is_ok()
    );
    assert!(verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_err());
    for policy in [
        dependency(
            "node_modules",
            "node-26",
            DependencyMaterializationKind::JavaScriptNodeModules,
        ),
        dependency(
            "node_modules",
            "node-24",
            DependencyMaterializationKind::PythonSitePackages,
        ),
        dependency(
            "other/node_modules",
            "node-24",
            DependencyMaterializationKind::JavaScriptNodeModules,
        ),
    ] {
        assert!(
            verify_prepared_source_v2_runtime(&source, &target("node-24"), &[policy], limits())
                .is_err()
        );
    }
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("node-24"), &[policy, policy], limits())
            .is_err()
    );
    let collision = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![
                file("node_modules/a/index.js", "dependency", Some("app")),
                file("node_modules/a/data", "static", None),
            ],
        ),
        &[],
    );
    assert!(
        verify_prepared_source_v2_runtime(&collision, &target("node-24"), &[policy], limits())
            .is_err()
    );
}

#[test]
fn dependency_symlinks_keep_same_layer_cross_root_traversal_and_reject_application_targets() {
    let files = vec![
        file("one/node_modules/a/index.js", "dependency", Some("app")),
        link(
            "one/node_modules/link",
            "../../two/node_modules/b",
            "dependency",
            Some("app"),
        ),
        file("two/node_modules/b/index.js", "dependency", Some("app")),
    ];
    let policies = [
        dependency(
            "one/node_modules",
            "node-24",
            DependencyMaterializationKind::JavaScriptNodeModules,
        ),
        dependency(
            "two/node_modules",
            "node-24",
            DependencyMaterializationKind::JavaScriptNodeModules,
        ),
    ];
    let source = prepare(manifest("NODE", "node-24", files.clone()), &[]);
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("node-24"), &policies, limits()).is_ok()
    );
    let mut escaped = files;
    escaped[1]["linkTarget"] = json!("../../server");
    let source = prepare(manifest("NODE", "node-24", escaped), &[]);
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("node-24"), &policies, limits())
            .is_err()
    );
}

#[test]
fn dependency_symlinks_cannot_use_foreign_layer_roots() {
    let mut value = manifest(
        "NODE",
        "node-24",
        vec![
            link(
                "node_modules/a",
                "../other/node_modules/b",
                "dependency",
                Some("app"),
            ),
            file("other/node_modules/b", "dependency", Some("other")),
            file("other/main.js", "compute", Some("other")),
        ],
    );
    value["layers"].as_array_mut().unwrap().push(layer(
        "other",
        "NODE",
        "node-24",
        "other/main.js",
    ));
    value["entrypoints"] = json!(["server", "other/main.js"]);
    value["routes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"pattern":"/other", "layerName":"other"}));
    let source = prepare(value, &[]);
    let targets = [
        LayerBuildTarget {
            layer_name: "app",
            build_runtime_target: "node-24",
        },
        LayerBuildTarget {
            layer_name: "other",
            build_runtime_target: "node-24",
        },
    ];
    let policies = [
        dependency(
            "node_modules",
            "node-24",
            DependencyMaterializationKind::JavaScriptNodeModules,
        ),
        AdmittedDependencyRoot {
            layer_name: "other",
            source_root: "other/node_modules",
            kind: DependencyMaterializationKind::JavaScriptNodeModules,
            build_runtime_target: "node-24",
        },
    ];
    assert!(verify_prepared_source_v2_runtime(&source, &targets, &policies, limits()).is_err());
}

#[test]
fn python_checks_compute_dependency_and_cross_role_file_targets_without_rejecting_plain_resources()
{
    let foreign = b"\xcf\xfa\xed\xfe\0\0\0\0";
    for path in [
        "server",
        ".onreza/python/3.13/site-packages/pkg/native.so",
        "public/native.so",
    ] {
        let files = vec![
            file(
                ".onreza/python/3.13/site-packages/pkg/native.so",
                "dependency",
                Some("app"),
            ),
            link("compute-alias", "public/native.so", "compute", Some("app")),
            link("directory-alias", "public", "compute", Some("app")),
            file("public/native.so", "static", None),
        ];
        let source = prepare(manifest("PYTHON", "python-3.13", files), &[(path, foreign)]);
        let roots = [dependency(
            ".onreza/python/3.13/site-packages",
            "python-3.13",
            DependencyMaterializationKind::PythonSitePackages,
        )];
        assert!(
            verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &roots, limits())
                .is_err(),
            "{path}"
        );
    }
    let files = vec![
        file("public/resource", "static", None),
        link("alias", "public", "compute", Some("app")),
    ];
    let source = prepare(
        manifest("PYTHON", "python-3.13", files),
        &[("server", b"print('ok')")],
    );
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[], limits()).is_ok()
    );
}

#[test]
fn family_only_python_source_keeps_concrete_admitted_dependency_minor_guard() {
    for (root_minor, allowed) in [("3.13", true), ("3.12", false)] {
        let root_path = format!(".onreza/python/{root_minor}/site-packages");
        let file_path = format!("{root_path}/a.py");
        let source = prepare(
            manifest(
                "PYTHON",
                "python",
                vec![file(&file_path, "dependency", Some("app"))],
            ),
            &[],
        );
        let root = dependency(
            &root_path,
            "python-3.13",
            DependencyMaterializationKind::PythonSitePackages,
        );
        assert_eq!(
            verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[root], limits())
                .is_ok(),
            allowed,
            "root minor {root_minor}"
        );
    }
}

#[test]
fn python_dependency_minor_is_not_inferred_from_source_claims() {
    let source = prepare(
        manifest(
            "PYTHON",
            "python-3.13",
            vec![file(
                ".onreza/python/3.12/site-packages/a.py",
                "dependency",
                Some("app"),
            )],
        ),
        &[],
    );
    let root = dependency(
        ".onreza/python/3.12/site-packages",
        "python-3.13",
        DependencyMaterializationKind::PythonSitePackages,
    );
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[root], limits())
            .is_err()
    );
}

fn elf(executable: bool, needed: Option<&str>) -> Vec<u8> {
    let mut bytes = vec![0; 1024];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&(if executable { 2u16 } else { 3u16 }).to_le_bytes());
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    if executable {
        bytes[24..32].copy_from_slice(&0x400000u64.to_le_bytes());
    }
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes[52..54].copy_from_slice(&64u16.to_le_bytes());
    bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
    bytes[56..58].copy_from_slice(&(if needed.is_some() { 3u16 } else { 1u16 }).to_le_bytes());
    bytes[64..68].copy_from_slice(&1u32.to_le_bytes());
    bytes[68..72].copy_from_slice(&5u32.to_le_bytes());
    bytes[80..88].copy_from_slice(&0x400000u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&1024u64.to_le_bytes());
    bytes[104..112].copy_from_slice(&1024u64.to_le_bytes());
    if let Some(library) = needed {
        bytes[120..124].copy_from_slice(&2u32.to_le_bytes());
        bytes[128..136].copy_from_slice(&256u64.to_le_bytes());
        bytes[136..144].copy_from_slice(&0x400100u64.to_le_bytes());
        bytes[152..160].copy_from_slice(&80u64.to_le_bytes());
        bytes[160..168].copy_from_slice(&80u64.to_le_bytes());
        for (index, (tag, value)) in [(5u64, 0x400200u64), (10, 128), (1, 1), (29, 32), (0, 0)]
            .iter()
            .enumerate()
        {
            let offset = 256 + index * 16;
            bytes[offset..offset + 8].copy_from_slice(&tag.to_le_bytes());
            bytes[offset + 8..offset + 16].copy_from_slice(&value.to_le_bytes());
        }
        bytes[513..513 + library.len()].copy_from_slice(library.as_bytes());
        bytes[544..555].copy_from_slice(b"$ORIGIN/lib");
        bytes[176..180].copy_from_slice(&3u32.to_le_bytes());
        bytes[184..192].copy_from_slice(&700u64.to_le_bytes());
        let interpreter = b"/lib64/ld-linux-x86-64.so.2\0";
        bytes[208..216].copy_from_slice(&(interpreter.len() as u64).to_le_bytes());
        bytes[700..700 + interpreter.len()].copy_from_slice(interpreter);
    }
    bytes
}

fn macho(cpu: u32) -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
    bytes[4..8].copy_from_slice(&cpu.to_le_bytes());
    bytes[12..16].copy_from_slice(&6u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&24u32.to_le_bytes());
    bytes[32..36].copy_from_slice(&0x1bu32.to_le_bytes());
    bytes[36..40].copy_from_slice(&24u32.to_le_bytes());
    bytes
}

#[test]
fn native_entries_and_transitive_providers_use_existing_closure_checks_and_layer_ownership() {
    let executable = elf(true, Some("libfixture.so"));
    let library = elf(false, None);
    for role in ["compute", "static"] {
        let source = prepare(
            manifest(
                "EXECUTABLE",
                nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
                vec![
                    file(
                        "lib/provider.so",
                        role,
                        (role == "compute").then_some("app"),
                    ),
                    link("lib/libfixture.so", "provider.so", "compute", Some("app")),
                ],
            ),
            &[("server", &executable), ("lib/provider.so", &library)],
        );
        let result = verify_prepared_source_v2_runtime(
            &source,
            &target(nrz_runtime_artifact::NATIVE_EXECUTION_TARGET),
            &[],
            limits(),
        );
        assert_eq!(result.is_ok(), role == "compute", "{result:?}");
    }
    let source = prepare(
        manifest(
            "EXECUTABLE",
            nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
            vec![],
        ),
        &[("server", &executable)],
    );
    assert!(
        verify_prepared_source_v2_runtime(
            &source,
            &target(nrz_runtime_artifact::NATIVE_EXECUTION_TARGET),
            &[],
            limits()
        )
        .is_err()
    );
}

#[test]
fn native_total_budget_is_shared_across_layers_and_file_limit_precedes_parsing() {
    let executable = elf(true, None);
    let mut value = manifest(
        "EXECUTABLE",
        nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
        vec![file("second", "compute", Some("other"))],
    );
    value["layers"].as_array_mut().unwrap().push(layer(
        "other",
        "EXECUTABLE",
        nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
        "second",
    ));
    value["entrypoints"] = json!(["server", "second"]);
    value["routes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"pattern":"/other", "layerName":"other"}));
    let source = prepare(value, &[("server", &executable), ("second", &executable)]);
    let targets = [
        LayerBuildTarget {
            layer_name: "app",
            build_runtime_target: nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
        },
        LayerBuildTarget {
            layer_name: "other",
            build_runtime_target: nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
        },
    ];
    let mut bounded = limits();
    bounded.max_native_total_bytes = 2047;
    assert!(
        verify_prepared_source_v2_runtime(&source, &targets, &[], bounded)
            .unwrap_err()
            .to_string()
            .contains("aggregate byte limit")
    );
    bounded.max_native_total_bytes = 2048;
    assert!(verify_prepared_source_v2_runtime(&source, &targets, &[], bounded).is_ok());
    bounded.max_native_file_bytes = 1023;
    assert!(
        verify_prepared_source_v2_runtime(&source, &targets, &[], bounded)
            .unwrap_err()
            .to_string()
            .contains("file exceeds its byte limit")
    );
}

#[test]
fn borrowed_path_index_has_explicit_node_and_work_bounds() {
    let source = prepare(
        manifest(
            "PYTHON",
            "python-3.13",
            vec![link("alias", "server", "compute", Some("app"))],
        ),
        &[],
    );
    let mut bounded = limits();
    bounded.max_path_nodes = source.usage().nodes - 1;
    assert!(matches!(
        verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[], bounded),
        Err(RuntimeVerificationError::Limit("path node limit"))
    ));
    bounded.max_path_nodes = source.usage().nodes;
    bounded.max_path_work = 1;
    assert!(matches!(
        verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[], bounded),
        Err(RuntimeVerificationError::Path(PathWorkError::Exhausted))
    ));
    bounded.max_path_work = limits().max_path_work;
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[], bounded).is_ok()
    );
}

#[test]
fn dependency_links_cannot_traverse_application_aliases_before_reentering_their_root() {
    let source = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![
                link(
                    "node_modules/a",
                    "../aliases/back/package",
                    "dependency",
                    Some("app"),
                ),
                file("node_modules/package/index.js", "dependency", Some("app")),
                link("aliases/back", "../node_modules", "compute", Some("app")),
            ],
        ),
        &[],
    );
    let policy = dependency(
        "node_modules",
        "node-24",
        DependencyMaterializationKind::JavaScriptNodeModules,
    );
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("node-24"), &[policy], limits())
            .is_err()
    );
}

#[test]
fn native_provider_aliases_retain_compute_ownership_even_when_the_target_is_owned() {
    let executable = elf(true, Some("libfixture.so"));
    let library = elf(false, None);
    let source = prepare(
        manifest(
            "EXECUTABLE",
            nrz_runtime_artifact::NATIVE_EXECUTION_TARGET,
            vec![
                file("lib/provider.so", "compute", Some("app")),
                link("lib/libfixture.so", "provider.so", "static", None),
            ],
        ),
        &[("server", &executable), ("lib/provider.so", &library)],
    );
    assert!(
        verify_prepared_source_v2_runtime(
            &source,
            &target(nrz_runtime_artifact::NATIVE_EXECUTION_TARGET),
            &[],
            limits()
        )
        .unwrap_err()
        .to_string()
        .contains("aliases must belong")
    );
}

#[test]
fn python_directory_alias_checks_foreign_payloads_reachable_through_static_descendants() {
    let mut pe = vec![0; 68];
    pe[..2].copy_from_slice(b"MZ");
    pe[60..64].copy_from_slice(&64u32.to_le_bytes());
    pe[64..].copy_from_slice(b"PE\0\0");
    for payload in [b"\xcf\xfa\xed\xfe\0\0\0\0".as_slice(), pe.as_slice()] {
        for chained in [false, true] {
            let mut files = vec![link("alias", "public", "compute", Some("app"))];
            if chained {
                files.push(link("public/assets", "../assets", "static", None));
            }
            let path = if chained {
                "assets/native.so"
            } else {
                "public/native.so"
            };
            files.push(file(path, "static", None));
            let source = prepare(manifest("PYTHON", "python-3.13", files), &[(path, payload)]);
            assert!(
                verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[], limits())
                    .is_err(),
                "directory alias chain={chained}"
            );
        }
    }
}

#[test]
fn python_directory_alias_traversal_terminates_on_cycles_and_accepts_plain_resources() {
    let source = prepare(
        manifest(
            "PYTHON",
            "python-3.13",
            vec![
                link("alias", "public", "compute", Some("app")),
                link("public/again", "../public", "static", None),
                link("public/other", "../assets", "static", None),
                link("assets/back", "../public", "static", None),
                file("assets/resource", "static", None),
                file("public/resource", "static", None),
            ],
        ),
        &[],
    );
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("python-3.13"), &[], limits()).is_ok()
    );
}

#[test]
fn embedded_javascript_addons_require_linux_x86_64_elf_even_through_aliases() {
    let linux = elf(false, None);
    let mut arm = linux.clone();
    arm[18..20].copy_from_slice(&183u16.to_le_bytes());
    for (payload, accepted) in [
        (linux.as_slice(), true),
        (arm.as_slice(), false),
        (b"plain resource".as_slice(), false),
        (b"\xcf\xfa\xed\xfe\0\0\0\0".as_slice(), false),
    ] {
        for (family, target_name) in [("NODE", "node-24"), ("BUN", "bun-1.4.2")] {
            for alias in [0, 1, 2] {
                let files = match alias {
                    0 => vec![file("node_modules/pkg/native.node", "compute", Some("app"))],
                    1 => vec![
                        link(
                            "node_modules/pkg/native.node",
                            "../../assets/payload",
                            "compute",
                            Some("app"),
                        ),
                        file("assets/payload", "static", None),
                    ],
                    _ => vec![
                        link("node_modules/pkg", "../assets", "compute", Some("app")),
                        file("assets/native.node", "static", None),
                    ],
                };
                let path = match alias {
                    0 => "node_modules/pkg/native.node",
                    1 => "assets/payload",
                    _ => "assets/native.node",
                };
                let source = prepare(manifest(family, target_name, files), &[(path, payload)]);
                assert_eq!(
                    verify_prepared_source_v2_runtime(&source, &target(target_name), &[], limits())
                        .is_ok(),
                    accepted,
                    "family={family} alias={alias}"
                );
            }
        }
    }
}

#[test]
fn embedded_javascript_keeps_plain_and_foreign_non_addon_resources() {
    let source = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![
                file("node_modules/pkg/index.js", "compute", Some("app")),
                file("node_modules/pkg/resource", "compute", Some("app")),
                link(
                    "node_modules/pkg/assets",
                    "../../assets",
                    "compute",
                    Some("app"),
                ),
                link(
                    "node_modules/pkg/cache.node",
                    "../../assets",
                    "compute",
                    Some("app"),
                ),
                file("assets/index.js", "static", None),
                link("assets/cycle", "../assets", "static", None),
                file("assets/resource", "static", None),
            ],
        ),
        &[("node_modules/pkg/resource", b"\xcf\xfa\xed\xfe\0\0\0\0")],
    );
    assert!(verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_ok());
}

#[test]
fn trusted_javascript_addons_keep_independent_root_and_native_platform_checks() {
    let linux = elf(false, None);
    let mut arm = linux.clone();
    arm[18..20].copy_from_slice(&183u16.to_le_bytes());
    let root = dependency(
        "node_modules",
        "node-24",
        DependencyMaterializationKind::JavaScriptNodeModules,
    );
    for (payload, accepted) in [(linux.as_slice(), true), (arm.as_slice(), false)] {
        let source = prepare(
            manifest(
                "NODE",
                "node-24",
                vec![file(
                    "node_modules/pkg/native.node",
                    "dependency",
                    Some("app"),
                )],
            ),
            &[("node_modules/pkg/native.node", payload)],
        );
        assert_eq!(
            verify_prepared_source_v2_runtime(&source, &target("node-24"), &[root], limits())
                .is_ok(),
            accepted
        );
        assert!(
            verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_err()
        );
    }
}

#[test]
fn labelled_foreign_addons_remain_opaque_without_weakening_host_or_alias_checks() {
    let linux = elf(false, None);
    let mut arm = linux.clone();
    arm[18..20].copy_from_slice(&183u16.to_le_bytes());
    let darwin_arm = macho(0x0100_000c);
    let darwin_x64 = macho(0x0100_0007);
    let mut windows = vec![0; 512];
    windows[..2].copy_from_slice(b"MZ");
    windows[60..64].copy_from_slice(&128u32.to_le_bytes());
    windows[128..132].copy_from_slice(b"PE\0\0");
    windows[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    windows[134..136].copy_from_slice(&1u16.to_le_bytes());
    windows[148..150].copy_from_slice(&112u16.to_le_bytes());
    windows[150..152].copy_from_slice(&0x2000u16.to_le_bytes());
    windows[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
    let mut windows_ia32 = windows.clone();
    windows_ia32[132..134].copy_from_slice(&0x014cu16.to_le_bytes());
    windows_ia32[148..150].copy_from_slice(&96u16.to_le_bytes());
    windows_ia32[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
    let mut windows_ia32_wrong_magic = windows_ia32.clone();
    windows_ia32_wrong_magic[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
    let mut windows_x64_wrong_magic = windows.clone();
    windows_x64_wrong_magic[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
    let root = dependency(
        "node_modules",
        "node-24",
        DependencyMaterializationKind::JavaScriptNodeModules,
    );
    let baseline = [
        ("node_modules/pkg/arm64-darwin.node", darwin_arm.as_slice()),
        ("node_modules/pkg/x64-darwin.node", darwin_x64.as_slice()),
        ("node_modules/pkg/x64-win32.node", windows.as_slice()),
        (
            "node_modules/bufferutil/prebuilds/win32-ia32/bufferutil.node",
            windows_ia32.as_slice(),
        ),
        ("node_modules/pkg/arm64-linux.node", arm.as_slice()),
        ("node_modules/pkg/x64-linux.node", linux.as_slice()),
    ];
    for (extra, payload, accepted) in [
        ("node_modules/pkg/binding.node", linux.as_slice(), true),
        (
            "node_modules/pkg/binding.node",
            darwin_arm.as_slice(),
            false,
        ),
        (
            "node_modules/pkg/x64-linux-bad.node",
            darwin_x64.as_slice(),
            false,
        ),
        (
            "node_modules/pkg/arm64-darwin-bad.node",
            darwin_x64.as_slice(),
            false,
        ),
        ("node_modules/pkg/arm64-linux-bad.node", &arm[..20], false),
        (
            "node_modules/pkg/x64-win32-bad.node",
            b"version https://git-lfs.github.com/spec/v1".as_slice(),
            false,
        ),
        (
            "node_modules/pkg/ia32-win32-bad.node",
            &windows_ia32[..153],
            false,
        ),
        (
            "node_modules/pkg/ia32-win32-bad.node",
            windows_ia32_wrong_magic.as_slice(),
            false,
        ),
        (
            "node_modules/pkg/x64-win32-bad.node",
            windows_x64_wrong_magic.as_slice(),
            false,
        ),
        (
            "node_modules/pkg/ia32-win32-bad.node",
            windows.as_slice(),
            false,
        ),
        (
            "node_modules/pkg/x64-win32-bad.node",
            windows_ia32.as_slice(),
            false,
        ),
    ] {
        let mut files = baseline
            .iter()
            .map(|(path, _)| file(path, "dependency", Some("app")))
            .collect::<Vec<_>>();
        files.push(file(extra, "dependency", Some("app")));
        files.push(link(
            "node_modules/pkg/darwin-arm64.node",
            "arm64-darwin.node",
            "dependency",
            Some("app"),
        ));
        let mut payloads = baseline.to_vec();
        payloads.push((extra, payload));
        let source = prepare(manifest("NODE", "node-24", files), &payloads);
        assert_eq!(
            verify_prepared_source_v2_runtime(&source, &target("node-24"), &[root], limits())
                .is_ok(),
            accepted,
            "{extra}"
        );
    }
    let source = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![
                file(
                    "node_modules/pkg/arm64-darwin.node",
                    "dependency",
                    Some("app"),
                ),
                link(
                    "node_modules/pkg/binding.node",
                    "arm64-darwin.node",
                    "dependency",
                    Some("app"),
                ),
            ],
        ),
        &[("node_modules/pkg/arm64-darwin.node", darwin_arm.as_slice())],
    );
    assert!(
        verify_prepared_source_v2_runtime(&source, &target("node-24"), &[root], limits()).is_err()
    );
}

#[test]
fn javascript_directory_aliases_keep_selected_platform_checks_after_prior_foreign_visits() {
    let darwin = macho(0x0100_000c);
    for alias in ["node_modules/pkg/linux-x64", "node_modules/pkg/unlabelled"] {
        for chained in [false, true] {
            let mut files = vec![
                link(
                    "aaa-darwin-arm64",
                    "assets/darwin-arm64",
                    "compute",
                    Some("app"),
                ),
                file("assets/darwin-arm64/binding.node", "static", None),
                link(alias, "../../assets/darwin-arm64", "compute", Some("app")),
            ];
            if chained {
                files.push(link("assets/bridge", "darwin-arm64", "static", None));
                files[2] = link(alias, "../../assets/bridge", "compute", Some("app"));
            }
            let source = prepare(
                manifest("NODE", "node-24", files),
                &[("assets/darwin-arm64/binding.node", darwin.as_slice())],
            );
            assert!(
                verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits())
                    .is_err(),
                "alias={alias} chained={chained}"
            );
        }
    }
}

#[test]
fn javascript_directory_aliases_preserve_foreign_labels_and_reject_conflicts() {
    let darwin = macho(0x0100_000c);
    for (alias, directory, name, accepted) in [
        ("darwin-arm64", "assets/opaque", "binding.node", true),
        ("unlabelled", "assets/opaque", "darwin-arm64.node", true),
        ("linux-x64", "assets/opaque", "darwin-arm64.node", false),
        ("darwin-arm64", "assets/linux-x64", "binding.node", false),
        ("darwin-arm64", "assets/darwin-x64", "binding.node", false),
    ] {
        let payload = format!("{directory}/{name}");
        let source = prepare(
            manifest(
                "NODE",
                "node-24",
                vec![
                    link(alias, directory, "compute", Some("app")),
                    file(&payload, "static", None),
                ],
            ),
            &[(&payload, darwin.as_slice())],
        );
        assert_eq!(
            verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_ok(),
            accepted,
            "alias={alias} physical={payload}"
        );
    }
    let source = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![
                link("darwin-arm64", "assets/opaque", "compute", Some("app")),
                link("assets/opaque/bridge", "../payload", "static", None),
                link("assets/opaque/cycle", ".", "static", None),
                file("assets/payload/binding.node", "static", None),
            ],
        ),
        &[("assets/payload/binding.node", darwin.as_slice())],
    );
    assert!(verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_ok());
}

#[test]
fn javascript_file_aliases_cannot_hide_foreign_addons_by_changing_the_extension() {
    let darwin = macho(0x0100_000c);
    let source = prepare(
        manifest(
            "NODE",
            "node-24",
            vec![
                link(
                    "binding",
                    "assets/darwin-arm64.node",
                    "compute",
                    Some("app"),
                ),
                file("assets/darwin-arm64.node", "static", None),
            ],
        ),
        &[("assets/darwin-arm64.node", darwin.as_slice())],
    );
    assert!(verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_err());
}

#[test]
fn javascript_file_aliases_keep_selected_platform_and_physical_agreement() {
    let darwin = macho(0x0100_000c);
    for (alias, payload, accepted) in [
        ("darwin-arm64.node", "assets/opaque.node", true),
        ("darwin-arm64.node", "assets/linux-x64.node", false),
        ("darwin-arm64.node", "assets/darwin-x64.node", false),
        ("linux-x64.node", "assets/darwin-arm64.node", false),
        ("unlabelled.node", "assets/darwin-arm64.node", false),
    ] {
        let source = prepare(
            manifest(
                "NODE",
                "node-24",
                vec![
                    link(alias, payload, "compute", Some("app")),
                    file(payload, "static", None),
                ],
            ),
            &[(payload, darwin.as_slice())],
        );
        assert_eq!(
            verify_prepared_source_v2_runtime(&source, &target("node-24"), &[], limits()).is_ok(),
            accepted,
            "alias={alias} physical={payload}"
        );
    }
}
