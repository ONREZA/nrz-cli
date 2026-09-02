// @generated vendored copy of platform crates/nrz-runtime-artifact/src/source_graph.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use nrz_source_bundle::{
    SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifest, SourceLogicalManifestLayer,
};
use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    RUNTIME_ARTIFACT_GRAPH_V2_SCHEMA_VERSION, RuntimeArtifactError, SOURCE_BUNDLE_MEDIA_TYPE,
    VerifiedDependencyMaterializationManifest, VerifiedRuntimeArtifactGraph,
    finalize_runtime_artifact_graph,
};

const SOURCE_BUNDLE_ARTIFACT_KIND: &str = "SOURCE_BUNDLE_V1";
const DEPENDENCY_FILE_ROLE: &str = "dependency";

pub struct SourceDependencyMaterialization<'a> {
    pub layer_name: &'a str,
    pub mount_point: &'a str,
    pub manifest: &'a VerifiedDependencyMaterializationManifest,
}

#[must_use]
pub fn compute_logical_artifact_id(
    kind: &str,
    schema_version: &str,
    logical_manifest_sha256: &str,
    artifact_blob_id: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update([0]);
    hasher.update(schema_version.as_bytes());
    hasher.update([0]);
    hasher.update(logical_manifest_sha256.as_bytes());
    hasher.update([0]);
    hasher.update(artifact_blob_id.as_bytes());
    hex::encode(hasher.finalize())
}

#[must_use]
pub fn compute_source_logical_artifact_id(
    logical_manifest_sha256: &str,
    source_sha256: &str,
) -> String {
    compute_logical_artifact_id(
        SOURCE_BUNDLE_ARTIFACT_KIND,
        SOURCE_BUNDLE_V1_SCHEMA_VERSION,
        logical_manifest_sha256,
        source_sha256,
    )
}

pub fn finalize_source_bundle_runtime_graph(
    logical_manifest_sha256: &str,
    source_sha256: &str,
    source_size_bytes: u64,
    manifest: &SourceLogicalManifest,
) -> Result<VerifiedRuntimeArtifactGraph, RuntimeArtifactError> {
    finalize_source_bundle_runtime_graph_with_dependencies(
        logical_manifest_sha256,
        source_sha256,
        source_size_bytes,
        manifest,
        &[],
    )
}

pub fn finalize_source_bundle_runtime_graph_with_dependencies(
    logical_manifest_sha256: &str,
    source_sha256: &str,
    source_size_bytes: u64,
    manifest: &SourceLogicalManifest,
    dependencies: &[SourceDependencyMaterialization<'_>],
) -> Result<VerifiedRuntimeArtifactGraph, RuntimeArtifactError> {
    let source_logical_artifact_id =
        compute_source_logical_artifact_id(logical_manifest_sha256, source_sha256);
    let application_paths = manifest
        .files
        .iter()
        .filter(|file| file.role != DEPENDENCY_FILE_ROLE)
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();
    let dependency_layers = dependency_layers(manifest, dependencies)?;
    let runtime_layers = runtime_layers(manifest, &dependency_layers)?;
    let dependencies = dependencies
        .iter()
        .map(runtime_dependency)
        .collect::<Result<Vec<_>, _>>()?;
    finalize_runtime_artifact_graph(
        json!({
            "schemaVersion": RUNTIME_ARTIFACT_GRAPH_V2_SCHEMA_VERSION,
            "application": {
                "artifactId": source_logical_artifact_id,
                "manifestDigest": logical_manifest_sha256,
                "blobDescriptor": {
                    "mediaType": SOURCE_BUNDLE_MEDIA_TYPE,
                    "digest": format!("sha256:{source_sha256}"),
                    "size": source_size_bytes
                }
            },
            "dependencies": dependencies,
            "runtimeLayers": runtime_layers
        }),
        &application_paths,
    )
}

fn dependency_layers(
    manifest: &SourceLogicalManifest,
    dependencies: &[SourceDependencyMaterialization<'_>],
) -> Result<HashMap<String, Vec<String>>, RuntimeArtifactError> {
    let compute_layers = manifest
        .layers
        .iter()
        .filter(|layer| layer.target == "COMPUTE")
        .map(|layer| layer.name.as_str())
        .collect::<HashSet<_>>();
    let dependency_file_layers = manifest
        .files
        .iter()
        .filter(|file| file.role == DEPENDENCY_FILE_ROLE)
        .map(|file| {
            file.layer_name.as_deref().ok_or_else(|| {
                RuntimeArtifactError::Invariant(format!(
                    "dependency file '{}' has no layerName",
                    file.path
                ))
            })
        })
        .collect::<Result<HashSet<_>, _>>()?;

    for layer_name in &dependency_file_layers {
        if !compute_layers.contains(layer_name) {
            return Err(RuntimeArtifactError::Invariant(format!(
                "dependency files reference unknown compute layer '{layer_name}'"
            )));
        }
    }

    let mut result = HashMap::<String, Vec<String>>::new();
    for dependency in dependencies {
        if !compute_layers.contains(dependency.layer_name) {
            return Err(RuntimeArtifactError::Invariant(format!(
                "dependency materialization references unknown compute layer '{}'",
                dependency.layer_name
            )));
        }
        if !dependency_file_layers.contains(dependency.layer_name) {
            return Err(RuntimeArtifactError::Invariant(format!(
                "dependency materialization for layer '{}' has no dependency-owned source files",
                dependency.layer_name
            )));
        }
        result
            .entry(dependency.layer_name.to_string())
            .or_default()
            .push(dependency.manifest.materialization_id().to_string());
    }

    for layer_name in dependency_file_layers {
        if !result.contains_key(layer_name) {
            return Err(RuntimeArtifactError::Invariant(format!(
                "dependency-owned source files for layer '{layer_name}' have no materialization"
            )));
        }
    }

    Ok(result)
}

fn runtime_dependency(
    dependency: &SourceDependencyMaterialization<'_>,
) -> Result<Value, RuntimeArtifactError> {
    let manifest = serde_json::to_value(dependency.manifest.wire())?;
    let manifest = manifest.as_object().ok_or_else(|| {
        RuntimeArtifactError::Invariant("dependency manifest is not an object".into())
    })?;
    Ok(json!({
        "materializationId": dependency.manifest.materialization_id(),
        "kind": manifest.get("kind").cloned(),
        "mountPoint": dependency.mount_point,
        "compatibility": manifest.get("compatibility").cloned(),
        "manifestDigest": dependency.manifest.manifest_digest(),
        "blobDescriptor": manifest.get("blobDescriptor").cloned(),
    }))
}

fn runtime_layers(
    manifest: &SourceLogicalManifest,
    dependency_layers: &HashMap<String, Vec<String>>,
) -> Result<Vec<Value>, RuntimeArtifactError> {
    let mut runtime_layers = Vec::new();
    for layer in &manifest.layers {
        match layer.target.as_str() {
            "STATIC" => {}
            "COMPUTE" => runtime_layers.push(runtime_layer(
                manifest,
                layer,
                dependency_layers
                    .get(layer.name.as_str())
                    .cloned()
                    .unwrap_or_default(),
            )?),
            target => {
                return Err(RuntimeArtifactError::Invariant(format!(
                    "unsupported source layer target '{target}'"
                )));
            }
        }
    }
    Ok(runtime_layers)
}

fn runtime_layer(
    manifest: &SourceLogicalManifest,
    layer: &SourceLogicalManifestLayer,
    dependency_materialization_ids: Vec<String>,
) -> Result<Value, RuntimeArtifactError> {
    let application_root = layer.root_path.as_deref().unwrap_or(".");
    let entrypoint = layer.entrypoint.as_deref().ok_or_else(|| {
        RuntimeArtifactError::Invariant(format!("compute layer '{}' has no entrypoint", layer.name))
    })?;
    let entrypoint_file = manifest
        .files
        .iter()
        .find(|file| file.path == entrypoint)
        .ok_or_else(|| {
            RuntimeArtifactError::Invariant(format!(
                "compute layer '{}' entrypoint is not an application file",
                layer.name
            ))
        })?;
    if entrypoint_file.role != "compute" {
        return Err(RuntimeArtifactError::Invariant(format!(
            "compute layer '{}' entrypoint is not owned by compute",
            layer.name
        )));
    }
    let relative_entrypoint = if application_root == "." {
        entrypoint
    } else {
        entrypoint
            .strip_prefix(application_root)
            .and_then(|suffix| suffix.strip_prefix('/'))
            .ok_or_else(|| {
                RuntimeArtifactError::Invariant(format!(
                    "compute layer '{}' entrypoint is outside applicationRoot",
                    layer.name
                ))
            })?
    };
    Ok(json!({
        "layerName": layer.name,
        "applicationRoot": application_root,
        "dependencyMaterializationIds": dependency_materialization_ids,
        "entrypoint": relative_entrypoint,
        "runtimeConfig": layer.runtime_config.clone().unwrap_or_else(|| json!({}))
    }))
}

#[cfg(test)]
mod tests {
    use nrz_source_bundle::{
        SourceLogicalManifestEntryType, SourceLogicalManifestFile, SourceLogicalManifestLayer,
    };
    use serde_json::json;

    use super::*;

    fn manifest() -> SourceLogicalManifest {
        SourceLogicalManifest {
            schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
            files: vec![SourceLogicalManifestFile {
                path: "server/server.js".to_string(),
                sha256: "c".repeat(64),
                size: 42,
                content_type: None,
                role: "compute".to_string(),
                layer_name: Some("server".to_string()),
                entry_type: SourceLogicalManifestEntryType::File,
                link_target: None,
                executable: false,
            }],
            layers: vec![SourceLogicalManifestLayer {
                name: "server".to_string(),
                target: "COMPUTE".to_string(),
                root_path: Some("server".to_string()),
                entrypoint: Some("server/server.js".to_string()),
                runtime_config: Some(json!({ "memoryMb": 256 })),
            }],
            capabilities: Vec::new(),
            routes: Vec::new(),
            entrypoints: Vec::new(),
        }
    }

    fn dependency_manifest() -> VerifiedDependencyMaterializationManifest {
        crate::verify_dependency_materialization_manifest(json!({
            "schemaVersion": "DEPENDENCY_MATERIALIZATION_V1.0",
            "kind": "JAVASCRIPT_NODE_MODULES",
            "compatibility": {
                "runtimeFamily": "bun",
                "runtimeVersion": "1.4.0",
                "os": "linux",
                "architecture": "x86_64",
                "libc": "glibc",
                "abi": "glibc-2.42",
                "packageManager": "bun",
                "packageManagerVersion": "1.4.0",
                "runnerRootfsDigest": format!("sha256:{}", "d".repeat(64)),
                "buildPolicyGeneration": 1
            },
            "logicalTreeDigest": "a".repeat(64),
            "expandedFileCount": 1,
            "expandedBytes": 42,
            "regularFileCount": 1,
            "symlinkCount": 0,
            "nativeObjectCount": 0,
            "canonicalizationPolicyDigest": format!("sha256:{}", "c".repeat(64)),
            "generatorDigest": format!("sha256:{}", "d".repeat(64)),
            "blobDescriptor": {
                "mediaType": "application/vnd.onreza.dependency.erofs.v1",
                "digest": format!("sha256:{}", "b".repeat(64)),
                "size": 4096
            }
        }))
        .unwrap()
    }

    fn manifest_with_dependency() -> SourceLogicalManifest {
        let mut manifest = manifest();
        manifest.files.push(SourceLogicalManifestFile {
            path: "node_modules/pkg/index.js".to_string(),
            sha256: "e".repeat(64),
            size: 42,
            content_type: None,
            role: DEPENDENCY_FILE_ROLE.to_string(),
            layer_name: Some("server".to_string()),
            entry_type: SourceLogicalManifestEntryType::File,
            link_target: None,
            executable: false,
        });
        manifest
    }

    #[test]
    fn source_graph_uses_workspace_neutral_logical_artifact_identity() {
        let logical_manifest_sha256 = "a".repeat(64);
        let source_sha256 = "b".repeat(64);
        let graph = finalize_source_bundle_runtime_graph(
            &logical_manifest_sha256,
            &source_sha256,
            1024,
            &manifest(),
        )
        .unwrap();

        assert_eq!(
            graph.wire().application.artifact_id.as_str(),
            compute_source_logical_artifact_id(&logical_manifest_sha256, &source_sha256)
        );
        assert_eq!(
            graph.wire().application.artifact_id.as_str(),
            "98c58f83802d7723aae94159ae0156586551f113c6fc21292d3442f8f9047c26"
        );
        assert_eq!(
            graph.wire().runtime_layers[0].entrypoint.as_str(),
            "server.js"
        );
    }

    #[test]
    fn source_graph_rejects_an_entrypoint_outside_its_application_root() {
        let mut manifest = manifest();
        manifest.layers[0].root_path = Some("worker".to_string());

        let error =
            finalize_source_bundle_runtime_graph(&"a".repeat(64), &"b".repeat(64), 1024, &manifest)
                .unwrap_err();

        assert!(error.to_string().contains("outside applicationRoot"));
    }

    #[test]
    fn source_graph_assigns_dependency_ownership_to_the_runtime_layer() {
        let dependency_manifest = dependency_manifest();
        let graph = finalize_source_bundle_runtime_graph_with_dependencies(
            &"a".repeat(64),
            &"b".repeat(64),
            1024,
            &manifest_with_dependency(),
            &[SourceDependencyMaterialization {
                layer_name: "server",
                mount_point: "/output/node_modules",
                manifest: &dependency_manifest,
            }],
        )
        .unwrap();

        assert_eq!(graph.wire().dependencies.len(), 1);
        assert_eq!(
            graph.wire().runtime_layers[0].dependency_materialization_ids[0].as_str(),
            dependency_manifest.materialization_id()
        );
    }

    #[test]
    fn source_graph_rejects_dependency_files_without_a_materialization() {
        let error = finalize_source_bundle_runtime_graph(
            &"a".repeat(64),
            &"b".repeat(64),
            1024,
            &manifest_with_dependency(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("have no materialization"));
    }

    #[test]
    fn source_graph_rejects_a_materialization_without_owned_files() {
        let dependency_manifest = dependency_manifest();
        let error = finalize_source_bundle_runtime_graph_with_dependencies(
            &"a".repeat(64),
            &"b".repeat(64),
            1024,
            &manifest(),
            &[SourceDependencyMaterialization {
                layer_name: "server",
                mount_point: "/output/node_modules",
                manifest: &dependency_manifest,
            }],
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("has no dependency-owned source files")
        );
    }
}
