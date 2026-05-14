use std::cmp::Ordering;
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
pub(crate) const CLI_PROTOCOL_VERSION: &str = "source-bundle-v1";
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
const TAR_MODE_OCTAL_WIDTH: usize = 7;
const TAR_SIZE_OCTAL_WIDTH: usize = 11;
const TAR_CHECKSUM_OCTAL_WIDTH: usize = 6;
const TAR_FILE_MODE: u64 = 0o644;
const TAR_SPACE: u8 = 0x20;

#[derive(Debug)]
pub(crate) struct SourceBundlePlan {
    pub(crate) logical_manifest: SourceLogicalManifest,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content_type: Option<String>,
    pub(crate) role: SourceLogicalManifestFileRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) layer_name: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) priority: Option<i32>,
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
    full_path: PathBuf,
    size: u64,
    sha256: String,
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
    let logical_manifest = build_logical_manifest(manifest, files)?;
    ensure_manifest_covers_entries(&logical_manifest, &entries)?;
    let logical_manifest_sha256 = compute_logical_manifest_sha256(&logical_manifest)?;

    let source_path =
        std::env::temp_dir().join(format!("nrz-source-bundle-{}.tar.zst", Uuid::now_v7()));
    let write_result = write_source_bundle(&source_path, &entries);
    if write_result.is_err() {
        let _ = std::fs::remove_file(&source_path);
    }
    let (source_sha256, source_size_bytes) = write_result?;
    let multipart = if source_size_bytes > MULTIPART_THRESHOLD_BYTES {
        Some(describe_multipart(&source_path, source_size_bytes)?)
    } else {
        None
    };

    Ok(SourceBundlePlan {
        logical_manifest,
        logical_manifest_sha256,
        source_sha256,
        source_size_bytes,
        multipart,
        source_path,
    })
}

pub(crate) fn compute_logical_manifest_sha256(
    manifest: &SourceLogicalManifest,
) -> anyhow::Result<String> {
    let value = serde_json::to_value(manifest).context("failed to serialize logical manifest")?;
    Ok(format!(
        "{:x}",
        Sha256::digest(stable_json_string(&value)?.as_bytes())
    ))
}

fn source_entries(
    output_dir: &Path,
    files: &[FileEntry],
) -> anyhow::Result<Vec<SourceBundleEntry>> {
    let mut entries = Vec::with_capacity(files.len());
    for file in files {
        validate_source_path(&file.path)?;
        let full_path = output_dir.join(&file.path);
        let metadata = std::fs::symlink_metadata(&full_path)
            .with_context(|| format!("failed to stat {}", full_path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "SOURCE_BUNDLE_V1 does not support symlinks in build output: {}",
                file.path
            );
        }
        if !metadata.is_file() {
            bail!(
                "SOURCE_BUNDLE_V1 only supports regular files in build output: {}",
                file.path
            );
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() > 1 {
                bail!(
                    "SOURCE_BUNDLE_V1 does not support hardlinked files in build output: {}",
                    file.path
                );
            }
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
            full_path,
            size,
            sha256,
        });
    }
    entries.sort_by(|a, b| compare_utf8(&a.path, &b.path));
    Ok(entries)
}

fn build_logical_manifest(
    manifest: &Manifest,
    files: &[FileEntry],
) -> anyhow::Result<SourceLogicalManifest> {
    let mut logical_files = Vec::with_capacity(files.len());
    let middleware_paths = middleware_paths(manifest);
    let prerender_paths = prerender_paths(manifest);

    for file in files {
        validate_source_path(&file.path)?;
        let matched_layer = best_layer_match(manifest, &file.path)?;
        let (role, layer_name) = file_role(
            manifest,
            &file.path,
            matched_layer.as_ref(),
            &middleware_paths,
            &prerender_paths,
        );
        logical_files.push(SourceLogicalManifestFile {
            path: file.path.clone(),
            sha256: file.content_hash.clone(),
            size: file.size,
            content_type: content_type_from_path(&file.path).map(str::to_string),
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
    let middleware = manifest.middleware.as_ref().map(|items| {
        items
            .iter()
            .map(|item| SourceLogicalManifestMiddleware {
                name: item.name.clone(),
                bundle_path: item.bundle_path.clone(),
                code_hash: item.code_hash.clone(),
                matchers: item.matchers.clone(),
                priority: item.priority,
            })
            .collect()
    });

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

fn ensure_manifest_covers_entries(
    manifest: &SourceLogicalManifest,
    entries: &[SourceBundleEntry],
) -> anyhow::Result<()> {
    if manifest.files.len() != entries.len() {
        bail!("SOURCE_BUNDLE_V1 logical manifest/file entry count mismatch");
    }
    for (file, entry) in manifest.files.iter().zip(entries) {
        if file.path != entry.path || file.sha256 != entry.sha256 || file.size != entry.size {
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
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            bail!("unsafe SOURCE_BUNDLE_V1 path: {path}");
        }
    }
    Ok(())
}

fn write_source_bundle(
    source_path: &Path,
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

    for entry in entries {
        write_tar_entry(&mut encoder, entry)?;
    }
    encoder.write_all(&[0u8; TAR_BLOCK_SIZE])?;
    encoder.write_all(&[0u8; TAR_BLOCK_SIZE])?;

    let writer = encoder.finish().context("failed to finalize zstd stream")?;
    Ok(writer.finish())
}

fn write_tar_entry<W: Write>(writer: &mut W, entry: &SourceBundleEntry) -> anyhow::Result<()> {
    let path_bytes = entry.path.as_bytes();
    let needs_pax_path = path_bytes.len() > TAR_NAME_LENGTH;
    if needs_pax_path {
        let pax_body = build_pax_path_record(&entry.path).into_bytes();
        writer.write_all(&tar_header(".__onreza/pax", pax_body.len() as u64, b'x'))?;
        writer.write_all(&pax_body)?;
        write_tar_padding(writer, pax_body.len() as u64)?;
    }

    writer.write_all(&tar_header(
        if needs_pax_path { "file" } else { &entry.path },
        entry.size,
        b'0',
    ))?;
    let mut file = std::fs::File::open(&entry.full_path)
        .with_context(|| format!("failed to open {}", entry.full_path.display()))?;
    let mut remaining = entry.size;
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    while remaining > 0 {
        let limit = remaining.min(COPY_BUFFER_BYTES as u64) as usize;
        let read = file
            .read(&mut buffer[..limit])
            .with_context(|| format!("failed to read {}", entry.full_path.display()))?;
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
        .with_context(|| format!("failed to re-check {}", entry.full_path.display()))?
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

fn tar_header(path: &str, size: u64, type_flag: u8) -> [u8; TAR_BLOCK_SIZE] {
    let mut header = [0u8; TAR_BLOCK_SIZE];
    write_ascii(&mut header, 0, TAR_NAME_LENGTH, path);
    write_ascii(
        &mut header,
        TAR_MODE_OFFSET,
        TAR_FIELD_SIZE,
        &octal(TAR_FILE_MODE, TAR_MODE_OCTAL_WIDTH),
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

fn build_pax_path_record(path: &str) -> String {
    let body = format!("path={path}\n");
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
