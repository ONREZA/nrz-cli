use nrz_runtime_artifact::{
    finalize_runtime_artifact_graph, verify_dependency_materialization_manifest,
    verify_runtime_artifact_graph,
};
use serde_json::{Value, json};

const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn compatibility() -> Value {
    json!({
        "runtimeFamily": "bun",
        "runtimeVersion": "1.4.2",
        "os": "linux",
        "architecture": "x86_64",
        "libc": "glibc",
        "abi": "glibc-2.42",
        "packageManager": "bun",
        "packageManagerVersion": "1.4.2",
        "runnerRootfsDigest": format!("sha256:{HEX_D}"),
        "buildPolicyGeneration": 1,
    })
}

fn materialization_manifest() -> Value {
    json!({
        "schemaVersion": "DEPENDENCY_MATERIALIZATION_V1.0",
        "kind": "JAVASCRIPT_NODE_MODULES",
        "compatibility": compatibility(),
        "logicalTreeDigest": HEX_A,
        "expandedFileCount": 12,
        "expandedBytes": 8192,
        "regularFileCount": 10,
        "symlinkCount": 2,
        "nativeObjectCount": 1,
        "canonicalizationPolicyDigest": format!("sha256:{HEX_C}"),
        "generatorDigest": format!("sha256:{HEX_D}"),
        "blobDescriptor": {
            "mediaType": "application/vnd.onreza.dependency.erofs.v1",
            "digest": format!("sha256:{HEX_B}"),
            "size": 4096,
        },
    })
}

fn graph() -> Value {
    json!({
        "schemaVersion": "RUNTIME_ARTIFACT_GRAPH_V2.0",
        "application": {
            "artifactId": HEX_A,
            "manifestDigest": HEX_A,
            "blobDescriptor": {
                "mediaType": "application/vnd.onreza.source-bundle.tar+zstd.v1",
                "digest": format!("sha256:{HEX_A}"),
                "size": 1024,
            },
        },
        "dependencies": [{
            "materializationId": HEX_B,
            "kind": "JAVASCRIPT_NODE_MODULES",
            "mountPoint": "/output/node_modules",
            "compatibility": compatibility(),
            "manifestDigest": HEX_C,
            "blobDescriptor": {
                "mediaType": "application/vnd.onreza.dependency.erofs.v1",
                "digest": format!("sha256:{HEX_B}"),
                "size": 4096,
            },
        }],
        "runtimeLayers": [{
            "layerName": "server",
            "applicationRoot": "server",
            "dependencyMaterializationIds": [HEX_B],
            "entrypoint": "server.js",
            "runtimeConfig": { "memoryMb": 512 },
        }],
        "graphDigest": "9c165aef0057a3b3691c46ceb35fef804801e17fe152a9a4fa588a0eda121d7f",
    })
}

#[test]
fn locks_typescript_identity_vectors() {
    let verified = verify_dependency_materialization_manifest(materialization_manifest()).unwrap();

    assert_eq!(
        verified.materialization_id(),
        "eeb6e41a55fb6c67cbd66aab6490a46a5dcf78e50f1b72117153cb7ce46f2755"
    );
    assert_eq!(
        verified.manifest_digest(),
        "c6a1ba372a8f75323f84f8a510d2b6eadcd3fbeaad4daba29963fe8190eb2916"
    );
    verify_runtime_artifact_graph(graph(), &["server/server.js".into()]).unwrap();
}

#[test]
fn rejects_graph_digest_and_application_ownership_mismatches() {
    let mut changed = graph();
    changed["application"]["blobDescriptor"]["size"] = json!(1025);
    assert!(verify_runtime_artifact_graph(changed, &[]).is_err());

    let error =
        verify_runtime_artifact_graph(graph(), &["node_modules/pkg/index.js".into()]).unwrap_err();
    assert!(error.to_string().contains("collides with dependency mount"));
}

#[test]
fn accepts_python_runtime_family_and_rejects_unknown_families() {
    let mut python = graph();
    python.as_object_mut().unwrap().remove("graphDigest");
    python["runtimeLayers"][0]["runtimeConfig"]["runtimeFamily"] = json!("PYTHON");
    finalize_runtime_artifact_graph(python, &["server/server.js".into()]).unwrap();

    let mut unknown = graph();
    unknown["runtimeLayers"][0]["runtimeConfig"]["runtimeFamily"] = json!("RUBY");
    assert!(verify_runtime_artifact_graph(unknown, &[]).is_err());
}

#[test]
fn rejects_cross_field_invariants_missing_from_json_schema() {
    let mut counters = materialization_manifest();
    counters["nativeObjectCount"] = json!(11);
    assert!(verify_dependency_materialization_manifest(counters).is_err());

    let mut unused = graph();
    unused["runtimeLayers"][0]["dependencyMaterializationIds"] = json!([]);
    assert!(verify_runtime_artifact_graph(unused, &[]).is_err());

    let mut mount = graph();
    mount["dependencies"][0]["mountPoint"] = json!("/output//node_modules");
    assert!(verify_runtime_artifact_graph(mount, &[]).is_err());
}

#[test]
fn finalizes_runtime_graph_with_the_canonical_digest() {
    let mut input = graph();
    input.as_object_mut().unwrap().remove("graphDigest");

    let finalized = finalize_runtime_artifact_graph(input, &["server/server.js".into()]).unwrap();

    assert_eq!(
        finalized.graph_digest(),
        "9c165aef0057a3b3691c46ceb35fef804801e17fe152a9a4fa588a0eda121d7f"
    );
}

#[test]
fn explicit_bindings_enforce_layer_ownership_and_every_application_mount() {
    let mut value = graph();
    value["schemaVersion"] = json!("RUNTIME_ARTIFACT_GRAPH_V2.1");
    value["runtimeLayers"][0]["dependencyBindings"] = json!({"mounts": [
        {"materializationId": HEX_B, "mountPoint": "/output/node_modules"},
        {"materializationId": HEX_B, "mountPoint": "/output/nested/node_modules"}
    ]});
    let verified = finalize_runtime_artifact_graph(value.clone(), &[]).unwrap();
    let restored =
        verify_runtime_artifact_graph(serde_json::to_value(verified.wire()).unwrap(), &[]).unwrap();
    assert!(restored.dependency_mounts("absent").is_err());
    let mut unbound_descriptor = value.clone();
    unbound_descriptor["dependencies"][0]["mountPoint"] = json!("/output/absent");
    assert!(finalize_runtime_artifact_graph(unbound_descriptor, &[]).is_err());
    assert!(
        finalize_runtime_artifact_graph(value.clone(), &["nested".into()])
            .unwrap_err()
            .to_string()
            .contains("collides with dependency mount")
    );
    assert_eq!(restored.dependency_mounts("server").unwrap().len(), 2);
    assert!(
        finalize_runtime_artifact_graph(
            value.clone(),
            &["nested/node_modules/pkg/index.js".into()]
        )
        .unwrap_err()
        .to_string()
        .contains("collides with dependency mount")
    );

    for bindings in [
        json!({"mounts": []}),
        json!({"mounts": [{"materializationId": HEX_C, "mountPoint": "/output/node_modules"}]}),
        json!({"mounts": [
            {"materializationId": HEX_B, "mountPoint": "/output/node_modules"},
            {"materializationId": HEX_B, "mountPoint": "/output/node_modules/nested"}
        ]}),
    ] {
        let mut invalid = value.clone();
        invalid["runtimeLayers"][0]["dependencyBindings"] = bindings;
        assert!(finalize_runtime_artifact_graph(invalid, &[]).is_err());
    }
    let mut old_version = value.clone();
    old_version["schemaVersion"] = json!("RUNTIME_ARTIFACT_GRAPH_V2.0");
    assert!(finalize_runtime_artifact_graph(old_version, &[]).is_err());
    value["runtimeLayers"][0]
        .as_object_mut()
        .unwrap()
        .remove("dependencyBindings");
    assert!(finalize_runtime_artifact_graph(value, &[]).is_err());
}

#[test]
fn launch_is_digest_bound_and_rejects_unsafe_or_contradictory_execution() {
    let mut value = graph();
    value["runtimeLayers"][0]["launch"] = json!({
        "profile": "BUN", "args": ["--flag"], "cwd": ".",
        "readiness": { "protocol": "HTTP", "path": "/ready" }
    });
    let finalized = finalize_runtime_artifact_graph(value.clone(), &[]).unwrap();
    assert_ne!(
        finalized.graph_digest(),
        graph()["graphDigest"].as_str().unwrap()
    );
    let mut tampered = serde_json::to_value(finalized.wire()).unwrap();
    tampered["runtimeLayers"][0]["launch"]["args"] = json!(["--other"]);
    assert!(verify_runtime_artifact_graph(tampered, &[]).is_err());
    for launch in [
        json!({ "profile": "BUN", "args": [], "cwd": "../escape" }),
        json!({ "profile": "BUN", "args": ["a\0b"], "cwd": "." }),
        json!({ "profile": "BUN", "args": [], "cwd": ".", "readiness": {"protocol":"HTTP"} }),
        json!({ "profile": "BUN", "args": [], "cwd": ".", "readiness": {"protocol":"TCP", "path":"/"} }),
        json!({ "profile": "SHELL", "args": [], "cwd": "." }),
    ] {
        value["runtimeLayers"][0]["launch"] = launch;
        assert!(finalize_runtime_artifact_graph(value.clone(), &[]).is_err());
    }
    value["runtimeLayers"][0]["launch"] =
        json!({ "profile": "CPYTHON_3_14", "args": [], "cwd": "." });
    value["runtimeLayers"][0]["runtimeConfig"]["runtimeFamily"] = json!("JAVASCRIPT");
    assert!(finalize_runtime_artifact_graph(value, &[]).is_err());
}
