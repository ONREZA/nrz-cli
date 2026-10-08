use nrz_runtime_artifact::{
    ExecutionRuntimeFamily as Family, ExecutionRuntimeTarget, VerifiedRuntimeArtifactGraph,
    finalize_runtime_artifact_graph, verify_execution_runtime_compatibility,
};
use serde_json::json;

fn graph(
    layers: &[(&str, &str, &str, &str)],
    architecture: &str,
    libc: &str,
) -> VerifiedRuntimeArtifactGraph {
    let digest = "a".repeat(64);
    let dependencies = layers.iter().enumerate().map(|(index, (_, _, kind, family))| {
        let runtime = if *family == "javascript" { "node-24" } else if *family == "python" { "python-3.14" } else { "1.4.2" };
        json!({
            "materializationId":format!("{index:064x}"),"kind":kind,
            "mountPoint":format!("/output/dep{index}"),"manifestDigest":digest,
            "compatibility":{"runtimeFamily":family,"runtimeVersion":runtime,
                "os":"linux","architecture":architecture,"libc":libc,"abi":"glibc-2.42",
                "packageManager":"fixture","packageManagerVersion":"1",
                "runnerRootfsDigest":format!("sha256:{digest}"),"buildPolicyGeneration":1},
            "blobDescriptor":{"mediaType":"application/vnd.onreza.dependency.erofs.v1","digest":format!("sha256:{digest}"),"size":4096}
        })
    }).collect::<Vec<_>>();
    let runtime_layers = layers
        .iter()
        .enumerate()
        .map(|(index, (name, profile, _, _))| {
            json!({
                "layerName":name,"applicationRoot":".","entrypoint":"server.js",
                "launch":{"profile":profile,"args":[],"cwd":"."},"runtimeConfig":{},
                "dependencyMaterializationIds":[format!("{index:064x}")]
            })
        })
        .collect::<Vec<_>>();
    finalize_runtime_artifact_graph(json!({
        "schemaVersion":"RUNTIME_ARTIFACT_GRAPH_V2.0",
        "application":{"artifactId":digest,"manifestDigest":digest,"blobDescriptor":{
            "mediaType":"application/vnd.onreza.source-bundle.tar+zstd.v1","digest":format!("sha256:{digest}"),"size":1024}},
        "dependencies":dependencies,"runtimeLayers":runtime_layers
    }), &[]).unwrap()
}
fn target(family: Family) -> ExecutionRuntimeTarget<'static> {
    ExecutionRuntimeTarget {
        family,
        os: "linux",
        architecture: "x86_64",
        libc: "glibc",
    }
}

#[test]
fn rejects_cross_family_bun_dependencies_that_immutable_graph_verification_accepts() {
    let graph = graph(
        &[("web", "BUN", "PYTHON_SITE_PACKAGES", "python")],
        "x86_64",
        "glibc",
    );
    assert!(verify_execution_runtime_compatibility(&graph, "web", target(Family::Bun)).is_err());
}

#[test]
fn rejects_known_platform_conflicts_before_a_candidate_probe() {
    for (architecture, libc) in [("aarch64", "glibc"), ("x86_64", "musl")] {
        let graph = graph(
            &[("web", "NODE_24", "JAVASCRIPT_NODE_MODULES", "javascript")],
            architecture,
            libc,
        );
        assert!(
            verify_execution_runtime_compatibility(&graph, "web", target(Family::Node)).is_err()
        );
    }
}

#[test]
fn checks_only_layer_owned_dependencies_in_a_mixed_application() {
    let graph = graph(
        &[
            ("web", "BUN", "JAVASCRIPT_NODE_MODULES", "bun"),
            ("worker", "CPYTHON_3_14", "PYTHON_SITE_PACKAGES", "python"),
        ],
        "x86_64",
        "glibc",
    );
    verify_execution_runtime_compatibility(&graph, "web", target(Family::Bun)).unwrap();
    verify_execution_runtime_compatibility(&graph, "worker", target(Family::Python)).unwrap();
    assert!(verify_execution_runtime_compatibility(&graph, "web", target(Family::Python)).is_err());
    assert!(verify_execution_runtime_compatibility(&graph, "absent", target(Family::Bun)).is_err());
}

#[test]
fn a_node_24_build_can_attempt_a_node_successor_without_rewriting_frozen_provenance() {
    let graph = graph(
        &[("web", "NODE_24", "JAVASCRIPT_NODE_MODULES", "javascript")],
        "x86_64",
        "glibc",
    );
    // A separately verified Node26 revision projects the same family/platform.
    // Neither build major nor the absence/presence of ELF proves successor ABI.
    verify_execution_runtime_compatibility(&graph, "web", target(Family::Node)).unwrap();
    assert_eq!(
        graph.wire().dependencies[0]
            .compatibility
            .runtime_version
            .as_str(),
        "node-24"
    );
}
