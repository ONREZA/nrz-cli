use super::*;
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, verify_source_bundle_v2_manifest,
};
use serde_json::json;

fn source(entries: Vec<serde_json::Value>) -> VerifiedSourceBundleV2Manifest {
    let entry = entries
        .iter()
        .find(|entry| entry["role"] == "compute")
        .map(|entry| entry["path"].clone());
    let (capabilities, layers, routes, entrypoints) = if let Some(entry) = entry {
        (
            json!(["COMPUTE_NETWORK_V1"]),
            json!([{"name":"app","target":"COMPUTE","entrypoint":entry,
            "runtimeConfig":{"applicationRuntime":{"family":"NODE","args":[]},"buildRuntimeVersion":"node-24",
            "network":nrz_source_bundle::ComputeNetwork::public_default()}}]),
            json!([{"pattern":"/","layerName":"app"}]),
            json!([entry]),
        )
    } else {
        (json!([]), json!([]), json!([]), json!([]))
    };
    verify_source_bundle_v2_manifest(
        json!({
            "schemaVersion":"SOURCE_BUNDLE_V2.0", "capabilities":capabilities,
            "files":entries, "layers":layers, "routes":routes, "entrypoints":entrypoints,
        }),
        SourceBundleV2ManifestBudget {
            max_file_count: 10_000,
            max_logical_bytes: 9_007_199_254_740_991,
            max_static_file_bytes: 9_007_199_254_740_991,
        },
    )
    .unwrap()
}
fn file(path: &str, role: &str, size: u64) -> serde_json::Value {
    let mut entry = json!({"entryType":"file","path":path,"role":role,"blobDigest":{"algorithm":"BLAKE3","value":"00".repeat(32)},
        "size":size,"executable":false});
    if role == "compute" || role == "dependency" {
        entry["layerName"] = json!("app");
    }
    entry
}

#[test]
fn small_inventory_reserves_measured_shape_instead_of_package_policy_maximum() {
    let entries = (0..995)
        .map(|n| file(&format!("app/{n:04}.js"), "compute", 6000))
        .collect();
    let plan = source_v2_storage_plan(&source(entries)).unwrap();
    assert_eq!(plan.source_bytes, 995 * 8192);
    assert_eq!(plan.source_nodes, 998);
    assert_eq!(plan.application_package_bytes[0], 0);
    assert!(plan.application_package_bytes[1] > 995 * 6000);
    assert!(plan.application_package_bytes[1] < 64 * 1024 * 1024);
    assert!(plan.staging_bytes < 128 * 1024 * 1024);
    assert_eq!(plan.dependency_count, 0);
}

#[test]
fn roles_are_independent_and_identical_contents_still_fund_distinct_inodes() {
    let one = source_v2_storage_plan(&source(vec![file("a", "static", 4097)])).unwrap();
    let two = source_v2_storage_plan(&source(vec![
        file("a", "static", 4097),
        file("b", "compute", 4097),
    ]))
    .unwrap();
    assert_eq!(two.source_bytes, one.source_bytes * 2);
    assert_eq!(
        two.application_package_bytes[0],
        one.application_package_bytes[0]
    );
    assert_eq!(
        two.application_package_bytes[1],
        one.application_package_bytes[0]
    );
    assert!(two.staging_bytes > one.staging_bytes);
}

#[test]
fn larger_files_include_cluster_index_and_exact_verity_growth() {
    let small = source_v2_storage_plan(&source(vec![file("a", "compute", 1)])).unwrap();
    let large =
        source_v2_storage_plan(&source(vec![file("a", "compute", 1024 * 1024 * 1024)])).unwrap();
    assert!(large.application_package_bytes[1] > 2 * 1024 * 1024 * 1024);
    assert!(large.application_package_bytes[1] > small.application_package_bytes[1]);
    assert_eq!(large.application_package_bytes[1] % BLOCK, 0);
}

#[test]
fn all_bound_arithmetic_refuses_overflow() {
    assert!(rounded(u64::MAX).is_err());
    assert!(mul(u64::MAX, 2).is_err());
    assert!(add(u64::MAX, 1).is_err());
}

#[test]
fn nested_directories_symlinks_and_distinct_dependency_roots_are_funded() {
    let manifest = source(vec![
        file("app/deep/main.js", "compute", 4097),
        json!({"entryType":"symlink","path":"app/link","role":"compute","layerName":"app",
            "linkTarget":"deep/main.js","size":0}),
        file("node_modules/pkg/first", "dependency", 7),
        file("vendor/node_modules/pkg/second", "dependency", 7),
    ]);
    let plan = source_v2_storage_plan(&manifest).unwrap();
    assert_eq!(plan.source_bytes, 5 * BLOCK);
    assert_eq!(plan.dependency_count, 2);
    assert!(plan.dependency_image_bytes > 2 * BLOCK);
    assert_eq!(plan.application_package_bytes[0], 0);
    assert!(plan.application_package_bytes[1] > 0);
    assert!(plan.staging_nodes > plan.source_nodes * 2);
    // Repeating a root with a second file creates another file, not another image.
    let same_root = source_v2_storage_plan(&source(vec![
        file("app/main.js", "compute", 4097),
        file("node_modules/pkg/first", "dependency", 7),
        file("node_modules/pkg/second", "dependency", 7),
    ]))
    .unwrap();
    assert_eq!(same_root.dependency_count, 1);
    assert!(same_root.dependency_image_bytes < plan.dependency_image_bytes);
}
