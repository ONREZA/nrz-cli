use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::build::manifest::{LayerTarget, Manifest};

pub(crate) mod source_bundle_v1;
#[cfg(test)]
mod source_bundle_v1_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BuildManifestSource {
    File,
    Generated,
    Absent,
}

impl BuildManifestSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Generated => "generated",
            Self::Absent => "absent",
        }
    }
}

#[derive(Debug)]
pub(crate) struct BuildArtifact {
    pub(crate) output_dir: PathBuf,
    pub(crate) manifest: Option<Manifest>,
    pub(crate) manifest_source: BuildManifestSource,
    pub(crate) detection: crate::detect::types::DetectionResult,
}

#[derive(Debug)]
pub(crate) struct RuntimeArtifact {
    pub(crate) root_dir: PathBuf,
    pub(crate) manifest: Manifest,
    pub(crate) scan: RuntimeArtifactScan,
}

#[derive(Debug, Clone)]
pub(crate) enum RuntimeArtifactScan {
    All,
    Selected { roots: Vec<String> },
}

impl RuntimeArtifactScan {
    pub(crate) fn explain(&self) -> serde_json::Value {
        match self {
            Self::All => serde_json::json!({ "mode": "all" }),
            Self::Selected { roots } => serde_json::json!({
                "mode": "selected",
                "roots": roots,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactFileKind {
    #[default]
    File,
    Symlink,
}

/// Per-file identity entry used by deployment-create bodies and by
/// SOURCE_BUNDLE_V1 logical manifest/archive construction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntry {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) content_hash: String,
    #[serde(skip)]
    pub(crate) kind: ArtifactFileKind,
    #[serde(skip)]
    pub(crate) symlink_resolved_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactFileRole {
    Static,
    Compute,
    Prerender,
    Platform,
    SymlinkTarget,
    BuildOnly,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactFile {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) content_hash: String,
    pub(crate) kind: ArtifactFileKind,
    pub(crate) role: ArtifactFileRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) symlink_resolved_path: Option<String>,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArtifactFileSummary {
    pub(crate) scanned_files: usize,
    pub(crate) deployable_files: usize,
    pub(crate) pruned_files: usize,
    pub(crate) deployable_bytes: u64,
    pub(crate) pruned_bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ArtifactFileCollection {
    pub(crate) files: Vec<ArtifactFile>,
    pub(crate) summary: ArtifactFileSummary,
}

impl ArtifactFileCollection {
    pub(crate) fn deployable_entries(&self) -> Vec<FileEntry> {
        self.files
            .iter()
            .filter(|file| {
                !matches!(
                    file.role,
                    ArtifactFileRole::BuildOnly | ArtifactFileRole::Platform
                )
            })
            .map(|file| FileEntry {
                path: file.path.clone(),
                size: file.size,
                content_hash: file.content_hash.clone(),
                kind: file.kind,
                symlink_resolved_path: file.symlink_resolved_path.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactRootScope {
    ProjectRoot,
    BuildOutput,
}

impl ArtifactRootScope {
    fn prunes_static_root_metadata(self) -> bool {
        self == Self::ProjectRoot
    }
}

pub(crate) fn classify_artifact_files(
    manifest: &Manifest,
    files: Vec<FileEntry>,
    detection: &crate::detect::types::DetectionResult,
    root_scope: ArtifactRootScope,
) -> ArtifactFileCollection {
    let mut classified = Vec::with_capacity(files.len());
    let prerender_paths = prerender_paths(manifest);
    let scanned_files = files.len();

    for file in files {
        let (role, layer, reason) = classify_file_role(
            manifest,
            detection,
            root_scope,
            &file.path,
            &prerender_paths,
        );
        classified.push(ArtifactFile {
            path: file.path,
            size: file.size,
            content_hash: file.content_hash,
            kind: file.kind,
            role,
            layer,
            symlink_resolved_path: file.symlink_resolved_path,
            reason,
        });
    }

    preserve_build_only_symlink_targets(&mut classified);
    let summary = summarize_artifact_files(scanned_files, &classified);

    ArtifactFileCollection {
        files: classified,
        summary,
    }
}

fn preserve_build_only_symlink_targets(files: &mut [ArtifactFile]) {
    let mut preserved = HashMap::<String, String>::new();
    for file in files
        .iter()
        .filter(|file| file.kind == ArtifactFileKind::Symlink && deployable_role(file.role))
    {
        let Some(target) = file.symlink_resolved_path.as_deref() else {
            continue;
        };
        for candidate in files.iter().filter(|candidate| {
            candidate.role == ArtifactFileRole::BuildOnly
                && (candidate.path == target || candidate.path.starts_with(&format!("{target}/")))
        }) {
            preserved
                .entry(candidate.path.clone())
                .or_insert_with(|| file.path.clone());
        }
    }

    for file in files {
        if let Some(symlink_path) = preserved.get(&file.path) {
            file.role = ArtifactFileRole::SymlinkTarget;
            file.reason = format!("required by deployable symlink '{symlink_path}'");
        }
    }
}

fn summarize_artifact_files(scanned_files: usize, files: &[ArtifactFile]) -> ArtifactFileSummary {
    let mut summary = ArtifactFileSummary {
        scanned_files,
        deployable_files: 0,
        pruned_files: 0,
        deployable_bytes: 0,
        pruned_bytes: 0,
    };
    for file in files {
        if deployable_role(file.role) {
            summary.deployable_files += 1;
            summary.deployable_bytes = summary.deployable_bytes.saturating_add(file.size);
        } else {
            summary.pruned_files += 1;
            summary.pruned_bytes = summary.pruned_bytes.saturating_add(file.size);
        }
    }
    summary
}

fn deployable_role(role: ArtifactFileRole) -> bool {
    !matches!(
        role,
        ArtifactFileRole::BuildOnly | ArtifactFileRole::Platform
    )
}

fn classify_file_role(
    manifest: &Manifest,
    detection: &crate::detect::types::DetectionResult,
    root_scope: ArtifactRootScope,
    path: &str,
    prerender_paths: &HashSet<String>,
) -> (ArtifactFileRole, Option<String>, String) {
    if is_platform_build_only_path(path) {
        return (
            ArtifactFileRole::Platform,
            None,
            "platform metadata is not part of runtime artifact".to_string(),
        );
    }
    if is_static_root_metadata_path(manifest, root_scope, path) {
        return (
            ArtifactFileRole::BuildOnly,
            None,
            "static root metadata is build-only".to_string(),
        );
    }
    if is_framework_build_only_path(manifest, detection, path) {
        return (
            ArtifactFileRole::BuildOnly,
            None,
            "framework cache/build-only path".to_string(),
        );
    }
    if prerender_paths.contains(path) {
        return (
            ArtifactFileRole::Prerender,
            manifest
                .prerender
                .as_ref()
                .map(|prerender| prerender.layer.clone()),
            "listed in manifest prerender pages".to_string(),
        );
    }

    let Some(layer) = best_layer_match(manifest, path) else {
        return (
            ArtifactFileRole::Static,
            static_fallback_layer(manifest),
            "not covered by a layer; served as static fallback".to_string(),
        );
    };
    match layer.target {
        LayerTarget::Static => (
            ArtifactFileRole::Static,
            Some(layer.name.clone()),
            format!("under STATIC layer '{}'", layer.name),
        ),
        LayerTarget::Compute => (
            ArtifactFileRole::Compute,
            Some(layer.name.clone()),
            format!("under COMPUTE layer '{}'", layer.name),
        ),
    }
}

fn prerender_paths(manifest: &Manifest) -> HashSet<String> {
    let mut paths = HashSet::new();
    let Some(prerender) = &manifest.prerender else {
        return paths;
    };
    let Some(layer) = manifest
        .layers
        .iter()
        .find(|layer| layer.name == prerender.layer)
    else {
        return paths;
    };
    let root = normalize_layer_root(&layer.directory);
    for page in prerender.pages.values() {
        paths.insert(join_layer_path(&root, &page.html));
        if let Some(data) = &page.data {
            paths.insert(join_layer_path(&root, data));
        }
    }
    paths
}

fn best_layer_match<'a>(
    manifest: &'a Manifest,
    path: &str,
) -> Option<&'a crate::build::manifest::Layer> {
    manifest
        .layers
        .iter()
        .filter(|layer| path_in_root(path, &normalize_layer_root(&layer.directory)))
        .max_by(|left, right| {
            normalize_layer_root(&left.directory)
                .len()
                .cmp(&normalize_layer_root(&right.directory).len())
                .then_with(|| right.name.cmp(&left.name))
        })
}

fn static_fallback_layer(manifest: &Manifest) -> Option<String> {
    manifest
        .layers
        .iter()
        .find(|layer| layer.target == LayerTarget::Static)
        .map(|layer| layer.name.clone())
}

fn normalize_layer_root(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        ".".to_string()
    } else {
        trimmed.to_string()
    }
}

fn join_layer_path(root: &str, path: &str) -> String {
    let path = path.trim_matches('/');
    if root == "." {
        path.to_string()
    } else {
        format!("{root}/{path}")
    }
}

fn path_in_root(path: &str, root: &str) -> bool {
    root == "." || path == root || path.starts_with(&format!("{root}/"))
}

fn manifest_has_compute_layer(manifest: &Manifest) -> bool {
    manifest
        .layers
        .iter()
        .any(|layer| layer.target == LayerTarget::Compute)
}

fn manifest_is_static_root_only(manifest: &Manifest) -> bool {
    !manifest_has_compute_layer(manifest)
        && manifest.layers.iter().any(|layer| {
            layer.target == LayerTarget::Static
                && matches!(layer.directory.trim_matches('/'), "" | ".")
        })
}

fn is_platform_build_only_path(path: &str) -> bool {
    path == ".onreza" || path.starts_with(".onreza/")
}

fn is_static_root_metadata_path(
    manifest: &Manifest,
    root_scope: ArtifactRootScope,
    path: &str,
) -> bool {
    if !root_scope.prunes_static_root_metadata() || !manifest_is_static_root_only(manifest) {
        return false;
    }

    path == "onreza.toml"
        || path == "package.json"
        || path == "package-lock.json"
        || path == "npm-shrinkwrap.json"
        || path == "pnpm-lock.yaml"
        || path == "pnpm-workspace.yaml"
        || path == "yarn.lock"
        || path == "bun.lock"
        || path == "bun.lockb"
        || path == "turbo.json"
        || path == "nx.json"
        || path == ".npmrc"
        || path == ".yarnrc"
        || path == ".yarnrc.yml"
        || path == ".pnp.cjs"
        || path == ".pnp.loader.mjs"
        || path == ".DS_Store"
        || path == "node_modules"
        || path.starts_with("node_modules/")
        || path == ".env"
        || path.starts_with(".env.")
}

fn is_framework_build_only_path(
    manifest: &Manifest,
    detection: &crate::detect::types::DetectionResult,
    path: &str,
) -> bool {
    if !manifest_has_compute_layer(manifest) {
        return false;
    }
    if !matches!(
        detection.framework.as_str(),
        "nextjs" | "blitzjs" | "payload"
    ) {
        return false;
    }

    path == ".next/cache" || path.starts_with(".next/cache/") || path.contains("/.next/cache/")
}
