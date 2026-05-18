use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, bail};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use uuid::Uuid;

use super::{FileEntry, hash_file_streaming};
use crate::build::manifest::{LayerTarget, Manifest};

pub(crate) const SOURCE_BUNDLE_SCHEMA_VERSION: &str = "SOURCE_BUNDLE_V1.0";
pub(crate) const SOURCE_BUNDLE_FORMAT: &str = "tar.zst";
pub(crate) const CLI_PROTOCOL_VERSION: &str = "source-bundle-v1-embedded-manifest";
const SOURCE_BUNDLE_METADATA_DIR: &str = ".__onreza";
const SOURCE_BUNDLE_METADATA_PREFIX: &str = ".__onreza/";
const SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH: &str = ".__onreza/logical-manifest.json";
pub(crate) const SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS: usize = 512;
pub(crate) const MULTIPART_THRESHOLD_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MULTIPART_CHUNK_BYTES: u64 = 16 * 1024 * 1024;

const ZSTD_LEVEL: i32 = 9;
const COPY_BUFFER_BYTES: usize = 64 * 1024;
const TAR_BLOCK_SIZE: usize = 512;
const TAR_NAME_LENGTH: usize = 100;
const TAR_MODE_OFFSET: usize = 100;
const TAR_UID_OFFSET: usize = 108;
const TAR_GID_OFFSET: usize = 116;
const TAR_SIZE_OFFSET: usize = 124;
const TAR_MTIME_OFFSET: usize = 136;
const TAR_CHECKSUM_OFFSET: usize = 148;
const TAR_CHECKSUM_END: usize = 156;
const TAR_TYPEFLAG_OFFSET: usize = 156;
const TAR_MAGIC_OFFSET: usize = 257;
const TAR_VERSION_OFFSET: usize = 263;
const TAR_FIELD_SIZE: usize = 8;
const TAR_SIZE_FIELD_SIZE: usize = 12;
const TAR_MAGIC_LENGTH: usize = 6;
const TAR_VERSION_LENGTH: usize = 2;
const TAR_LINKNAME_OFFSET: usize = 157;
const TAR_LINKNAME_LENGTH: usize = 100;
const TAR_MODE_OCTAL_WIDTH: usize = 7;
const TAR_SIZE_OCTAL_WIDTH: usize = 11;
const TAR_CHECKSUM_OCTAL_WIDTH: usize = 6;
const TAR_FILE_MODE: u64 = 0o644;
const TAR_SYMLINK_MODE: u64 = 0o777;
const TAR_SPACE: u8 = 0x20;

#[derive(Debug)]
pub(crate) struct SourceBundlePlan {
    #[cfg(test)]
    pub(crate) logical_manifest: SourceLogicalManifest,
    pub(crate) logical_manifest_summary: SourceLogicalManifestSummary,
    pub(crate) logical_manifest_sha256: String,
    pub(crate) source_sha256: String,
    pub(crate) source_size_bytes: u64,
    pub(crate) multipart: Option<SourceBundleMultipartDescriptor>,
    source_path: PathBuf,
}

impl SourceBundlePlan {
    pub(crate) fn source_size_string(&self) -> String {
        self.source_size_bytes.to_string()
    }

    pub(crate) async fn read_all(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::from(
            tokio::fs::read(&self.source_path).await.with_context(|| {
                format!(
                    "failed to read SOURCE_BUNDLE_V1 archive {}",
                    self.source_path.display()
                )
            })?,
        ))
    }

    pub(crate) async fn read_chunk(&self, offset: u64, size: u64) -> anyhow::Result<Bytes> {
        read_file_slice(&self.source_path, offset, size).await
    }
}

impl Drop for SourceBundlePlan {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.source_path);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLogicalManifest {
    pub(crate) schema_version: String,
    pub(crate) capabilities: Vec<String>,
    pub(crate) files: Vec<SourceLogicalManifestFile>,
    pub(crate) layers: Vec<SourceLogicalManifestLayer>,
    pub(crate) routes: Vec<SourceLogicalManifestRoute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) middleware: Option<Vec<SourceLogicalManifestMiddleware>>,
    pub(crate) entrypoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLogicalManifestFile {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
    #[serde(rename = "entryType", skip_serializing_if = "Option::is_none")]
    pub(crate) entry_type: Option<SourceLogicalManifestEntryType>,
    #[serde(rename = "linkTarget", skip_serializing_if = "Option::is_none")]
    pub(crate) link_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content_type: Option<String>,
    pub(crate) role: SourceLogicalManifestFileRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) layer_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SourceLogicalManifestEntryType {
    File,
    Symlink,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SourceLogicalManifestFileRole {
    Static,
    Compute,
    Isolate,
    Middleware,
    Prerender,
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLogicalManifestLayer {
    pub(crate) name: String,
    pub(crate) target: SourceLogicalManifestLayerTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) root_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum SourceLogicalManifestLayerTarget {
    Static,
    Compute,
    Isolate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLogicalManifestRoute {
    pub(crate) pattern: String,
    pub(crate) layer_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) methods: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLogicalManifestMiddleware {
    pub(crate) name: String,
    pub(crate) bundle_path: String,
    pub(crate) code_hash: String,
    pub(crate) matchers: Vec<String>,
    pub(crate) priority: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceBundleMultipartPart {
    pub(crate) part_number: u32,
    pub(crate) size_bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceBundleMultipartDescriptor {
    pub(crate) part_size_bytes: u64,
    pub(crate) part_count: u32,
    pub(crate) parts: Vec<SourceBundleMultipartPart>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceLogicalManifestSummary {
    pub(crate) file_count: u64,
    pub(crate) logical_static_bytes: String,
    pub(crate) artifact_size_bytes: String,
    pub(crate) max_static_file_size_bytes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresignedSourceSinglePut {
    pub(crate) url: String,
    pub(crate) content_length: u64,
    pub(crate) sha256: String,
    #[serde(default)]
    pub(crate) headers: crate::api::PresignedPutHeaders,
    #[serde(rename = "verifyHead")]
    pub(crate) verify_head: Option<crate::api::PresignedHeadVerify>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresignedSourceMultipart {
    #[serde(rename = "uploadId")]
    pub(crate) upload_id: String,
    #[serde(rename = "chunkSize")]
    pub(crate) chunk_size: u64,
    pub(crate) chunks: Vec<PresignedSourceMultipartChunk>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresignedSourceMultipartChunk {
    pub(crate) part_number: u32,
    pub(crate) url: String,
    pub(crate) content_length: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompletedMultipartPart {
    pub(crate) part_number: u32,
    pub(crate) e_tag: String,
}

#[derive(Debug)]
struct SourceBundleEntry {
    path: String,
    size: u64,
    sha256: String,
    kind: SourceBundleEntryKind,
}

#[derive(Debug)]
enum SourceBundleEntryKind {
    File {
        full_path: PathBuf,
    },
    Symlink {
        link_target: String,
        resolved_path: String,
    },
}

#[derive(Debug)]
struct SourceSymlinkTarget {
    link_target: String,
    resolved_path: String,
}

#[derive(Debug, Default)]
struct SourceArchivePathIndex {
    files: HashSet<String>,
    symlinks: HashMap<String, String>,
}

#[derive(Debug)]
struct LayerMatch<'a> {
    layer: &'a crate::build::manifest::Layer,
    root_path: String,
}

pub(crate) fn build_source_bundle_plan(
    output_dir: &Path,
    manifest: &Manifest,
    files: &[FileEntry],
) -> anyhow::Result<SourceBundlePlan> {
    let entries = source_entries(output_dir, files)?;
    let logical_manifest = build_logical_manifest(manifest, &entries)?;
    ensure_manifest_covers_entries(&logical_manifest, &entries)?;
    let logical_manifest_json = canonical_logical_manifest_json(&logical_manifest)?;
    let logical_manifest_sha256 = sha256_hex(logical_manifest_json.as_bytes());
    let logical_manifest_summary = summarize_logical_manifest(&logical_manifest);

    let source_path =
        std::env::temp_dir().join(format!("nrz-source-bundle-{}.tar.zst", Uuid::now_v7()));
    let write_result =
        write_source_bundle(&source_path, logical_manifest_json.as_bytes(), &entries);
    if write_result.is_err() {
        let _ = std::fs::remove_file(&source_path);
    }
    let (source_sha256, source_size_bytes) = write_result?;
    let multipart = if should_use_multipart(source_size_bytes) {
        Some(describe_multipart(&source_path, source_size_bytes)?)
    } else {
        None
    };

    Ok(SourceBundlePlan {
        #[cfg(test)]
        logical_manifest,
        logical_manifest_summary,
        logical_manifest_sha256,
        source_sha256,
        source_size_bytes,
        multipart,
        source_path,
    })
}

#[cfg(test)]
pub(crate) fn compute_logical_manifest_sha256(
    manifest: &SourceLogicalManifest,
) -> anyhow::Result<String> {
    Ok(sha256_hex(
        canonical_logical_manifest_json(manifest)?.as_bytes(),
    ))
}

fn canonical_logical_manifest_json(manifest: &SourceLogicalManifest) -> anyhow::Result<String> {
    let value = serde_json::to_value(manifest).context("failed to serialize logical manifest")?;
    stable_json_string(&value)
}

fn summarize_logical_manifest(manifest: &SourceLogicalManifest) -> SourceLogicalManifestSummary {
    let mut logical_static_bytes = 0_u64;
    let mut artifact_size_bytes = 0_u64;
    let mut max_static_file_size_bytes = 0_u64;
    for file in &manifest.files {
        if file.role == SourceLogicalManifestFileRole::Static {
            logical_static_bytes = logical_static_bytes.saturating_add(file.size);
        } else {
            artifact_size_bytes = artifact_size_bytes.saturating_add(file.size);
        }
        if matches!(
            file.role,
            SourceLogicalManifestFileRole::Static
                | SourceLogicalManifestFileRole::Prerender
                | SourceLogicalManifestFileRole::Config
        ) {
            max_static_file_size_bytes = max_static_file_size_bytes.max(file.size);
        }
    }
    SourceLogicalManifestSummary {
        file_count: manifest.files.len() as u64,
        logical_static_bytes: logical_static_bytes.to_string(),
        artifact_size_bytes: artifact_size_bytes.to_string(),
        max_static_file_size_bytes: max_static_file_size_bytes.to_string(),
    }
}

fn source_entries(
    output_dir: &Path,
    files: &[FileEntry],
) -> anyhow::Result<Vec<SourceBundleEntry>> {
    let canonical_base = std::fs::canonicalize(output_dir)
        .with_context(|| format!("failed to canonicalize {}", output_dir.display()))?;
    let mut entries = Vec::with_capacity(files.len());
    for file in files {
        validate_source_path(&file.path)?;
        let full_path = output_dir.join(&file.path);
        let metadata = std::fs::symlink_metadata(&full_path)
            .with_context(|| format!("failed to stat {}", full_path.display()))?;
        if metadata.file_type().is_symlink() {
            let symlink = read_source_symlink_target(&full_path, &file.path, &canonical_base)?;
            let sha256 = sha256_hex(symlink.link_target.as_bytes());
            if file.size != 0 || file.content_hash != sha256 {
                bail!(
                    "SOURCE_BUNDLE_V1 symlink changed during packaging: {}",
                    file.path
                );
            }
            entries.push(SourceBundleEntry {
                path: file.path.clone(),
                size: 0,
                sha256,
                kind: SourceBundleEntryKind::Symlink {
                    link_target: symlink.link_target,
                    resolved_path: symlink.resolved_path,
                },
            });
            continue;
        }
        if !metadata.is_file() {
            bail!(
                "SOURCE_BUNDLE_V1 only supports regular files in build output: {}",
                file.path
            );
        }
        let (size, sha256) = hash_file_streaming(&full_path)
            .with_context(|| format!("failed to hash SOURCE_BUNDLE_V1 file {}", file.path))?;
        if size != file.size || sha256 != file.content_hash {
            bail!(
                "SOURCE_BUNDLE_V1 file changed during packaging: {}",
                file.path
            );
        }
        entries.push(SourceBundleEntry {
            path: file.path.clone(),
            size,
            sha256,
            kind: SourceBundleEntryKind::File { full_path },
        });
    }
    let path_index = SourceArchivePathIndex::from_entries(&entries)?;
    for entry in &entries {
        if let SourceBundleEntryKind::Symlink {
            link_target,
            resolved_path,
        } = &entry.kind
        {
            ensure_symlink_target_in_archive(&entry.path, link_target, resolved_path, &path_index)?;
        }
    }
    entries.sort_by(|a, b| compare_utf8(&a.path, &b.path));
    Ok(entries)
}

fn build_logical_manifest(
    manifest: &Manifest,
    entries: &[SourceBundleEntry],
) -> anyhow::Result<SourceLogicalManifest> {
    let mut logical_files = Vec::with_capacity(entries.len());
    let middleware_paths = middleware_paths(manifest);
    let prerender_paths = prerender_paths(manifest);

    for entry in entries {
        validate_source_path(&entry.path)?;
        let matched_layer = best_layer_match(manifest, &entry.path)?;
        let (role, layer_name) = file_role(
            manifest,
            &entry.path,
            matched_layer.as_ref(),
            &middleware_paths,
            &prerender_paths,
        );
        let (entry_type, link_target) = match &entry.kind {
            SourceBundleEntryKind::File { .. } => (None, None),
            SourceBundleEntryKind::Symlink { link_target, .. } => (
                Some(SourceLogicalManifestEntryType::Symlink),
                Some(link_target.clone()),
            ),
        };
        logical_files.push(SourceLogicalManifestFile {
            path: entry.path.clone(),
            sha256: entry.sha256.clone(),
            size: entry.size,
            entry_type,
            link_target,
            content_type: content_type_from_path(&entry.path).map(str::to_string),
            role,
            layer_name,
        });
    }
    logical_files.sort_by(|a, b| compare_utf8(&a.path, &b.path));

    let layers = manifest
        .layers
        .iter()
        .map(source_layer_from_manifest)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let entrypoints = layers
        .iter()
        .filter_map(|layer| layer.entrypoint.clone())
        .collect();
    let routes = manifest
        .routes
        .iter()
        .map(|route| SourceLogicalManifestRoute {
            pattern: route.pattern.clone(),
            layer_name: route.layer.clone(),
            priority: route.priority,
            methods: route.methods.clone(),
        })
        .collect();
    let middleware = manifest
        .middleware
        .as_ref()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let code_hash = normalize_middleware_code_hash(&item.name, &item.code_hash)?;
                    let bundle_file_sha256 = logical_files
                        .iter()
                        .find(|file| file.path == item.bundle_path)
                        .map(|file| file.sha256.as_str())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "middleware '{}' bundlePath '{}' was not found in the output directory",
                                item.name,
                                item.bundle_path
                            )
                        })?;
                    if code_hash != bundle_file_sha256 {
                        anyhow::bail!(
                            "middleware '{}' codeHash does not match bundlePath '{}'",
                            item.name,
                            item.bundle_path
                        );
                    }
                    Ok(SourceLogicalManifestMiddleware {
                        name: item.name.clone(),
                        bundle_path: item.bundle_path.clone(),
                        code_hash,
                        matchers: item.matchers.clone(),
                        priority: item.priority.unwrap_or(0),
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()
        })
        .transpose()?;

    Ok(SourceLogicalManifest {
        schema_version: SOURCE_BUNDLE_SCHEMA_VERSION.to_string(),
        capabilities: Vec::new(),
        files: logical_files,
        layers,
        routes,
        middleware,
        entrypoints,
    })
}

fn normalize_middleware_code_hash(name: &str, value: &str) -> anyhow::Result<String> {
    let hash = value.strip_prefix("sha256-").unwrap_or(value);
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("middleware '{name}' codeHash must be a SHA-256 hex digest");
    }
    Ok(hash.to_ascii_lowercase())
}

fn ensure_manifest_covers_entries(
    manifest: &SourceLogicalManifest,
    entries: &[SourceBundleEntry],
) -> anyhow::Result<()> {
    if manifest.files.len() != entries.len() {
        bail!("SOURCE_BUNDLE_V1 logical manifest/file entry count mismatch");
    }
    for (file, entry) in manifest.files.iter().zip(entries) {
        let entry_type = match entry.kind {
            SourceBundleEntryKind::File { .. } => None,
            SourceBundleEntryKind::Symlink { .. } => Some(SourceLogicalManifestEntryType::Symlink),
        };
        let link_target = match &entry.kind {
            SourceBundleEntryKind::File { .. } => None,
            SourceBundleEntryKind::Symlink { link_target, .. } => Some(link_target.as_str()),
        };
        if file.path != entry.path
            || file.sha256 != entry.sha256
            || file.size != entry.size
            || file.entry_type != entry_type
            || file.link_target.as_deref() != link_target
        {
            bail!(
                "SOURCE_BUNDLE_V1 logical manifest does not match archive entry {}",
                entry.path
            );
        }
    }
    Ok(())
}

fn middleware_paths(manifest: &Manifest) -> Vec<String> {
    manifest
        .middleware
        .as_ref()
        .map(|items| items.iter().map(|item| item.bundle_path.clone()).collect())
        .unwrap_or_default()
}

fn prerender_paths(manifest: &Manifest) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(prerender) = &manifest.prerender {
        for page in prerender.pages.values() {
            paths.push(page.html.clone());
            if let Some(data) = &page.data {
                paths.push(data.clone());
            }
        }
    }
    paths
}

fn best_layer_match<'a>(
    manifest: &'a Manifest,
    path: &str,
) -> anyhow::Result<Option<LayerMatch<'a>>> {
    let mut matches = Vec::new();
    for layer in &manifest.layers {
        let root_path = normalize_layer_root(&layer.directory)?;
        if path_in_root(path, &root_path) {
            matches.push(LayerMatch { layer, root_path });
        }
    }
    matches.sort_by(|a, b| {
        b.root_path
            .len()
            .cmp(&a.root_path.len())
            .then_with(|| compare_utf8(&a.layer.name, &b.layer.name))
    });
    Ok(matches.into_iter().next())
}

fn file_role(
    manifest: &Manifest,
    path: &str,
    matched_layer: Option<&LayerMatch<'_>>,
    middleware_paths: &[String],
    prerender_paths: &[String],
) -> (SourceLogicalManifestFileRole, Option<String>) {
    if middleware_paths.iter().any(|candidate| candidate == path) {
        return (SourceLogicalManifestFileRole::Middleware, None);
    }
    if prerender_paths.iter().any(|candidate| candidate == path) {
        return (
            SourceLogicalManifestFileRole::Prerender,
            manifest
                .prerender
                .as_ref()
                .map(|prerender| prerender.layer.clone()),
        );
    }

    let Some(layer_match) = matched_layer else {
        let fallback_static = manifest
            .layers
            .iter()
            .find(|layer| layer.target == LayerTarget::Static)
            .map(|layer| layer.name.clone());
        return (SourceLogicalManifestFileRole::Static, fallback_static);
    };

    let role = match layer_match.layer.target {
        LayerTarget::Static if is_static_config_path(path) => SourceLogicalManifestFileRole::Config,
        LayerTarget::Static => SourceLogicalManifestFileRole::Static,
        LayerTarget::Compute => SourceLogicalManifestFileRole::Compute,
        LayerTarget::Isolate => SourceLogicalManifestFileRole::Isolate,
    };
    (role, Some(layer_match.layer.name.clone()))
}

fn source_layer_from_manifest(
    layer: &crate::build::manifest::Layer,
) -> anyhow::Result<SourceLogicalManifestLayer> {
    let root_path = normalize_layer_root(&layer.directory)?;
    let entrypoint = layer
        .entry
        .as_deref()
        .map(|entry| join_entrypoint(&root_path, entry))
        .transpose()?;
    let runtime_config = runtime_config_value(layer);
    Ok(SourceLogicalManifestLayer {
        name: layer.name.clone(),
        target: match layer.target {
            LayerTarget::Static => SourceLogicalManifestLayerTarget::Static,
            LayerTarget::Compute => SourceLogicalManifestLayerTarget::Compute,
            LayerTarget::Isolate => SourceLogicalManifestLayerTarget::Isolate,
        },
        root_path: (root_path != ".").then_some(root_path),
        entrypoint,
        runtime_config,
    })
}

fn runtime_config_value(layer: &crate::build::manifest::Layer) -> Option<serde_json::Value> {
    let runtime = layer.runtime.as_ref()?;
    let mut object = serde_json::Map::new();
    if let Some(value) = runtime.timeout_ms {
        object.insert("timeoutMs".to_string(), serde_json::json!(value));
    }
    if let Some(value) = runtime.memory_mb {
        object.insert("memoryMb".to_string(), serde_json::json!(value));
    }
    if let Some(value) = runtime.max_concurrency {
        object.insert("maxConcurrency".to_string(), serde_json::json!(value));
    }
    (!object.is_empty()).then_some(serde_json::Value::Object(object))
}

fn normalize_layer_root(path: &str) -> anyhow::Result<String> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        return Ok(".".to_string());
    }
    validate_source_path(trimmed)?;
    Ok(trimmed.to_string())
}

fn join_entrypoint(root_path: &str, entry: &str) -> anyhow::Result<String> {
    let entry = normalize_relative_path(entry)?;
    if root_path == "." {
        Ok(entry)
    } else {
        Ok(format!("{root_path}/{entry}"))
    }
}

fn normalize_relative_path(path: &str) -> anyhow::Result<String> {
    let normalized =
        Path::new(path)
            .components()
            .try_fold(PathBuf::new(), |mut out, component| {
                match component {
                    Component::Normal(part) => out.push(part),
                    Component::CurDir => {}
                    Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                        bail!("unsafe SOURCE_BUNDLE_V1 path: {path}")
                    }
                }
                Ok::<_, anyhow::Error>(out)
            })?;
    let normalized = normalized.to_string_lossy().replace('\\', "/");
    validate_source_path(&normalized)?;
    Ok(normalized)
}

fn path_in_root(path: &str, root: &str) -> bool {
    root == "." || path == root || path.starts_with(&format!("{root}/"))
}

fn validate_source_path(path: &str) -> anyhow::Result<()> {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') || path.contains('\0') {
        bail!("invalid SOURCE_BUNDLE_V1 path: {path}");
    }
    if path == SOURCE_BUNDLE_METADATA_DIR || path.starts_with(SOURCE_BUNDLE_METADATA_PREFIX) {
        bail!(
            "SOURCE_BUNDLE_V1 reserves metadata namespace {SOURCE_BUNDLE_METADATA_PREFIX}: {path}"
        );
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            bail!("unsafe SOURCE_BUNDLE_V1 path: {path}");
        }
    }
    Ok(())
}

impl SourceArchivePathIndex {
    fn from_entries(entries: &[SourceBundleEntry]) -> anyhow::Result<Self> {
        let mut index = Self::default();
        for entry in entries {
            index.insert_entry(entry)?;
        }
        Ok(index)
    }

    fn insert_entry(&mut self, entry: &SourceBundleEntry) -> anyhow::Result<()> {
        validate_source_path(&entry.path)?;
        match &entry.kind {
            SourceBundleEntryKind::File { .. } => {
                self.files.insert(entry.path.clone());
            }
            SourceBundleEntryKind::Symlink { link_target, .. } => {
                self.symlinks
                    .insert(entry.path.clone(), link_target.to_string());
            }
        }
        Ok(())
    }

    fn contains_resolvable_target(&self, path: &str, symlink_path: &str) -> anyhow::Result<bool> {
        validate_source_path(path)?;
        let mut seen = HashSet::new();
        seen.insert(symlink_path.to_string());
        self.path_resolves_to_file(path, &mut seen)
    }

    fn path_resolves_to_file(
        &self,
        path: &str,
        seen_symlinks: &mut HashSet<String>,
    ) -> anyhow::Result<bool> {
        if self.files.contains(path) {
            return Ok(true);
        }

        if let Some(link_target) = self.symlinks.get(path) {
            if !seen_symlinks.insert(path.to_string()) {
                return Ok(false);
            }
            let resolved = resolve_source_symlink_target(path, link_target)?;
            return self.path_resolves_to_file(&resolved, seen_symlinks);
        }

        if let Some((prefix, link_target)) = self.longest_symlink_prefix(path) {
            if !seen_symlinks.insert(prefix.to_string()) {
                return Ok(false);
            }
            let suffix = path
                .strip_prefix(prefix)
                .unwrap_or_default()
                .strip_prefix('/')
                .unwrap_or_default();
            let mut resolved = resolve_source_symlink_target(prefix, link_target)?;
            if !suffix.is_empty() {
                resolved.push('/');
                resolved.push_str(suffix);
                validate_source_path(&resolved)?;
            }
            return self.path_resolves_to_file(&resolved, seen_symlinks);
        }

        let descendant_prefix = format!("{path}/");
        for candidate in self.files.iter().chain(self.symlinks.keys()) {
            if !candidate.starts_with(&descendant_prefix) {
                continue;
            }
            let mut branch_seen = seen_symlinks.clone();
            if self.path_resolves_to_file(candidate, &mut branch_seen)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn longest_symlink_prefix<'a>(&'a self, path: &'a str) -> Option<(&'a str, &'a str)> {
        let mut prefix = path;
        loop {
            if let Some(link_target) = self.symlinks.get(prefix) {
                return Some((prefix, link_target.as_str()));
            }
            let (parent, _) = prefix.rsplit_once('/')?;
            prefix = parent;
        }
    }
}

fn read_source_symlink_target(
    path: &Path,
    rel_path: &str,
    canonical_base: &Path,
) -> anyhow::Result<SourceSymlinkTarget> {
    let target = std::fs::read_link(path)
        .with_context(|| format!("failed to read SOURCE_BUNDLE_V1 symlink {}", path.display()))?;
    let target = target.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "SOURCE_BUNDLE_V1 symlink target is not UTF-8: {}",
            path.display()
        )
    })?;
    let resolved_path = resolve_source_symlink_target(rel_path, target)?;
    match std::fs::canonicalize(path) {
        Ok(canonical) if canonical.starts_with(canonical_base) => Ok(SourceSymlinkTarget {
            link_target: target.to_string(),
            resolved_path,
        }),
        Ok(canonical) => bail!(
            "SOURCE_BUNDLE_V1 symlink escapes build output: {} -> {} resolved to {}",
            rel_path,
            target,
            canonical.display()
        ),
        Err(error) => bail!(
            "SOURCE_BUNDLE_V1 broken symlink in build output: {} -> {} ({})",
            rel_path,
            target,
            error
        ),
    }
}

fn ensure_symlink_target_in_archive(
    rel_path: &str,
    link_target: &str,
    resolved_path: &str,
    path_index: &SourceArchivePathIndex,
) -> anyhow::Result<()> {
    if path_index.contains_resolvable_target(resolved_path, rel_path)? {
        return Ok(());
    }
    bail!(
        "SOURCE_BUNDLE_V1 symlink target is not included in archive: {} -> {} resolved to {}",
        rel_path,
        link_target,
        resolved_path
    );
}

fn resolve_source_symlink_target(rel_path: &str, target: &str) -> anyhow::Result<String> {
    if target.is_empty() || target.contains('\\') || target.contains('\0') {
        bail!("unsafe SOURCE_BUNDLE_V1 symlink target for {rel_path}: {target}");
    }
    if source_bundle_contract_characters(target) > SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS {
        bail!(
            "SOURCE_BUNDLE_V1 symlink target too long for {rel_path}: {target} (max {SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS} characters)"
        );
    }
    let target_path = Path::new(target);
    if target_path.is_absolute() {
        bail!("SOURCE_BUNDLE_V1 symlink has absolute target: {rel_path} -> {target}");
    }

    let mut resolved = PathBuf::new();
    if let Some(parent) = Path::new(rel_path).parent()
        && !parent.as_os_str().is_empty()
    {
        resolved.push(parent);
    }
    for component in target_path.components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    bail!("SOURCE_BUNDLE_V1 symlink escapes archive root: {rel_path} -> {target}");
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!("unsafe SOURCE_BUNDLE_V1 symlink target for {rel_path}: {target}");
            }
        }
    }

    let resolved = resolved.to_string_lossy().replace('\\', "/");
    validate_source_path(&resolved)?;
    Ok(resolved)
}

pub(crate) fn source_bundle_contract_characters(value: &str) -> usize {
    value.encode_utf16().count()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_source_bundle(
    source_path: &Path,
    logical_manifest_json: &[u8],
    entries: &[SourceBundleEntry],
) -> anyhow::Result<(String, u64)> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(source_path)
        .with_context(|| format!("failed to create {}", source_path.display()))?;
    let writer = HashingWriter::new(file);
    let mut encoder =
        zstd::stream::Encoder::new(writer, ZSTD_LEVEL).context("failed to create zstd encoder")?;
    encoder
        .include_checksum(true)
        .context("failed to enable zstd checksum")?;

    write_tar_metadata_entry(
        &mut encoder,
        SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
        logical_manifest_json,
    )?;
    for entry in entries {
        write_tar_entry(&mut encoder, entry)?;
    }
    encoder.write_all(&[0u8; TAR_BLOCK_SIZE])?;
    encoder.write_all(&[0u8; TAR_BLOCK_SIZE])?;

    let writer = encoder.finish().context("failed to finalize zstd stream")?;
    Ok(writer.finish())
}

fn write_tar_metadata_entry<W: Write>(
    writer: &mut W,
    path: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    writer.write_all(&tar_header(path, body.len() as u64, b'0', None))?;
    writer.write_all(body)?;
    write_tar_padding(writer, body.len() as u64)?;
    Ok(())
}

fn write_tar_entry<W: Write>(writer: &mut W, entry: &SourceBundleEntry) -> anyhow::Result<()> {
    let path_bytes = entry.path.as_bytes();
    let needs_pax_path = path_bytes.len() > TAR_NAME_LENGTH;
    let link_target = match &entry.kind {
        SourceBundleEntryKind::File { .. } => None,
        SourceBundleEntryKind::Symlink { link_target, .. } => Some(link_target.as_str()),
    };
    let needs_pax_link_target =
        link_target.is_some_and(|target| target.len() > TAR_LINKNAME_LENGTH);
    if needs_pax_path || needs_pax_link_target {
        let mut fields = Vec::new();
        if needs_pax_path {
            fields.push(("path", entry.path.as_str()));
        }
        if let Some(link_target) = link_target
            && needs_pax_link_target
        {
            fields.push(("linkpath", link_target));
        }
        let pax_body = build_pax_record(&fields).into_bytes();
        writer.write_all(&tar_header(
            ".__onreza/pax",
            pax_body.len() as u64,
            b'x',
            None,
        ))?;
        writer.write_all(&pax_body)?;
        write_tar_padding(writer, pax_body.len() as u64)?;
    }

    if let SourceBundleEntryKind::Symlink { link_target, .. } = &entry.kind {
        writer.write_all(&tar_header(
            if needs_pax_path { "file" } else { &entry.path },
            0,
            b'2',
            (!needs_pax_link_target).then_some(link_target.as_str()),
        ))?;
        return Ok(());
    }

    writer.write_all(&tar_header(
        if needs_pax_path { "file" } else { &entry.path },
        entry.size,
        b'0',
        None,
    ))?;
    let SourceBundleEntryKind::File { full_path } = &entry.kind else {
        unreachable!("symlink entries return before file copy")
    };
    let mut file = std::fs::File::open(full_path)
        .with_context(|| format!("failed to open {}", full_path.display()))?;
    let mut remaining = entry.size;
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    while remaining > 0 {
        let limit = remaining.min(COPY_BUFFER_BYTES as u64) as usize;
        let read = file
            .read(&mut buffer[..limit])
            .with_context(|| format!("failed to read {}", full_path.display()))?;
        if read == 0 {
            bail!(
                "SOURCE_BUNDLE_V1 file truncated during packaging: {}",
                entry.path
            );
        }
        writer.write_all(&buffer[..read])?;
        remaining = remaining.saturating_sub(read as u64);
    }
    let mut extra = [0u8; 1];
    if file
        .read(&mut extra)
        .with_context(|| format!("failed to re-check {}", full_path.display()))?
        != 0
    {
        bail!(
            "SOURCE_BUNDLE_V1 file grew during packaging: {}",
            entry.path
        );
    }
    write_tar_padding(writer, entry.size)?;
    Ok(())
}

fn tar_header(
    path: &str,
    size: u64,
    type_flag: u8,
    link_name: Option<&str>,
) -> [u8; TAR_BLOCK_SIZE] {
    let mut header = [0u8; TAR_BLOCK_SIZE];
    write_ascii(&mut header, 0, TAR_NAME_LENGTH, path);
    if let Some(link_name) = link_name {
        write_ascii(
            &mut header,
            TAR_LINKNAME_OFFSET,
            TAR_LINKNAME_LENGTH,
            link_name,
        );
    }
    let mode = if type_flag == b'2' {
        TAR_SYMLINK_MODE
    } else {
        TAR_FILE_MODE
    };
    write_ascii(
        &mut header,
        TAR_MODE_OFFSET,
        TAR_FIELD_SIZE,
        &octal(mode, TAR_MODE_OCTAL_WIDTH),
    );
    write_ascii(
        &mut header,
        TAR_UID_OFFSET,
        TAR_FIELD_SIZE,
        &octal(0, TAR_MODE_OCTAL_WIDTH),
    );
    write_ascii(
        &mut header,
        TAR_GID_OFFSET,
        TAR_FIELD_SIZE,
        &octal(0, TAR_MODE_OCTAL_WIDTH),
    );
    write_ascii(
        &mut header,
        TAR_SIZE_OFFSET,
        TAR_SIZE_FIELD_SIZE,
        &octal(size, TAR_SIZE_OCTAL_WIDTH),
    );
    write_ascii(
        &mut header,
        TAR_MTIME_OFFSET,
        TAR_SIZE_FIELD_SIZE,
        &octal(0, TAR_SIZE_OCTAL_WIDTH),
    );
    for byte in header
        .iter_mut()
        .take(TAR_CHECKSUM_END)
        .skip(TAR_CHECKSUM_OFFSET)
    {
        *byte = TAR_SPACE;
    }
    header[TAR_TYPEFLAG_OFFSET] = type_flag;
    write_ascii(&mut header, TAR_MAGIC_OFFSET, TAR_MAGIC_LENGTH, "ustar");
    write_ascii(&mut header, TAR_VERSION_OFFSET, TAR_VERSION_LENGTH, "00");
    let checksum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    write_ascii(
        &mut header,
        TAR_CHECKSUM_OFFSET,
        TAR_FIELD_SIZE,
        &format!("{checksum:0width$o}\0 ", width = TAR_CHECKSUM_OCTAL_WIDTH),
    );
    header
}

fn octal(value: u64, width: usize) -> String {
    format!("{value:0width$o}\0")
}

fn write_ascii(target: &mut [u8], offset: usize, length: usize, value: &str) {
    let bytes = value.as_bytes();
    let end = bytes.len().min(length);
    target[offset..offset + end].copy_from_slice(&bytes[..end]);
}

fn write_tar_padding<W: Write>(writer: &mut W, size: u64) -> anyhow::Result<()> {
    let rem = (size as usize) % TAR_BLOCK_SIZE;
    if rem != 0 {
        writer.write_all(&vec![0u8; TAR_BLOCK_SIZE - rem])?;
    }
    Ok(())
}

fn build_pax_record(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(key, value)| build_pax_key_value_record(key, value))
        .collect()
}

fn build_pax_key_value_record(key: &str, value: &str) -> String {
    let body = format!("{key}={value}\n");
    let mut length = format!("0 {body}").len();
    loop {
        let record = format!("{length} {body}");
        let byte_length = record.len();
        if byte_length == length {
            return record;
        }
        length = byte_length;
    }
}

pub(crate) fn should_use_multipart(source_size_bytes: u64) -> bool {
    source_size_bytes >= MULTIPART_THRESHOLD_BYTES
}

fn describe_multipart(
    source_path: &Path,
    source_size_bytes: u64,
) -> anyhow::Result<SourceBundleMultipartDescriptor> {
    let mut file = std::fs::File::open(source_path)
        .with_context(|| format!("failed to open {}", source_path.display()))?;
    let mut parts = Vec::new();
    let mut remaining = source_size_bytes;
    let mut part_number = 1u32;
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];

    while remaining > 0 {
        let part_size = remaining.min(MULTIPART_CHUNK_BYTES);
        let mut hasher = Sha256::new();
        let mut read_for_part = 0u64;
        while read_for_part < part_size {
            let limit = (part_size - read_for_part).min(COPY_BUFFER_BYTES as u64) as usize;
            let read = file
                .read(&mut buffer[..limit])
                .context("failed to read SOURCE_BUNDLE_V1 multipart chunk")?;
            if read == 0 {
                bail!("SOURCE_BUNDLE_V1 archive truncated while describing multipart upload");
            }
            hasher.update(&buffer[..read]);
            read_for_part += read as u64;
        }
        parts.push(SourceBundleMultipartPart {
            part_number,
            size_bytes: part_size,
            sha256: format!("{:x}", hasher.finalize()),
        });
        remaining -= part_size;
        part_number += 1;
    }

    Ok(SourceBundleMultipartDescriptor {
        part_size_bytes: MULTIPART_CHUNK_BYTES,
        part_count: parts.len() as u32,
        parts,
    })
}

struct HashingWriter<W> {
    inner: W,
    hasher: Sha256,
    bytes_written: u64,
}

impl<W> HashingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes_written: 0,
        }
    }

    fn finish(self) -> (String, u64) {
        (format!("{:x}", self.hasher.finalize()), self.bytes_written)
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        self.bytes_written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn stable_json_string(value: &serde_json::Value) -> anyhow::Result<String> {
    Ok(match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => serde_json::to_string(value)?,
        serde_json::Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(stable_json_string)
                .collect::<anyhow::Result<Vec<_>>>()?
                .join(",")
        ),
        serde_json::Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            let mut out = String::from("{");
            for (idx, (key, entry_value)) in entries.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key)?);
                out.push(':');
                out.push_str(&stable_json_string(entry_value)?);
            }
            out.push('}');
            out
        }
    })
}

fn compare_utf8(a: &str, b: &str) -> Ordering {
    a.as_bytes().cmp(b.as_bytes())
}

fn content_type_from_path(path: &str) -> Option<&'static str> {
    let ext = PathBuf::from(path)
        .extension()
        .and_then(|e| e.to_str())?
        .to_ascii_lowercase();
    Some(match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" | "cjs" => "application/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "pdf" => "application/pdf",
        _ => return None,
    })
}

fn is_static_config_path(path: &str) -> bool {
    path == "_headers" || path == "_redirects"
}

async fn read_file_slice(path: &Path, offset: u64, size: u64) -> anyhow::Result<Bytes> {
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("failed to open {}", path.display()))?;
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .with_context(|| format!("failed to seek {}", path.display()))?;
    let mut buf =
        vec![0u8; usize::try_from(size).context("file slice too large for this platform")?];
    file.read_exact(&mut buf)
        .await
        .with_context(|| format!("failed to read {} bytes from {}", size, path.display()))?;
    Ok(Bytes::from(buf))
}
