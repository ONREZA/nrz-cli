use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::FileEntry;
use crate::api::PresignedPutHeaders;
use crate::build::manifest::{LayerTarget, Manifest};

pub(crate) const PACK_PART_TARGET_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MULTIPART_THRESHOLD_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MULTIPART_CHUNK_BYTES: u64 = 16 * 1024 * 1024;

const READ_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MultipartChunkDescriptor {
    pub(crate) part_number: u32,
    pub(crate) offset: u64,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackPathDescriptor {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content_type: Option<String>,
    pub(crate) part_index: u32,
    pub(crate) offset: u64,
    pub(crate) length: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackPartDescriptor {
    pub(crate) part_index: u32,
    pub(crate) size: u64,
    pub(crate) sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chunks: Option<Vec<MultipartChunkDescriptor>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestSummary {
    pub(crate) file_count: usize,
    pub(crate) total_logical_bytes: String,
    pub(crate) paths: Vec<PackPathDescriptor>,
    pub(crate) pack_parts: Vec<PackPartDescriptor>,
}

#[derive(Debug, Clone)]
pub(crate) struct PackPlan {
    pub(crate) summary: ManifestSummary,
    pub(crate) parts: Vec<PackPartPlan>,
    pub(crate) total_logical_bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct PackPartPlan {
    pub(crate) part_index: u32,
    pub(crate) size: u64,
    pub(crate) sha256: String,
    ranges: Vec<PackPartRange>,
}

#[derive(Debug, Clone)]
struct PackPartRange {
    path: String,
    offset: u64,
    size: u64,
    sha256: String,
}

#[derive(Debug, Clone)]
struct StoredPackRange {
    part_index: u32,
    offset: u64,
    length: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComputeBundleUpload {
    pub(crate) layer_name: String,
    pub(crate) bundle_sha256: String,
    pub(crate) size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chunks: Option<Vec<MultipartChunkDescriptor>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IsolateModuleUpload {
    pub(crate) layer_name: String,
    pub(crate) files: Vec<IsolateModuleFileUpload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IsolateModuleFileUpload {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) chunks: Option<Vec<MultipartChunkDescriptor>>,
}

#[derive(Debug, Clone)]
pub(crate) struct IsolateUploadPlan {
    pub(crate) modules: Vec<IsolateModuleUpload>,
    local_files: Vec<IsolateLocalFile>,
}

#[derive(Debug, Clone)]
struct IsolateLocalFile {
    layer_name: String,
    module_path: String,
    output_path: String,
    sha256: String,
}

impl IsolateUploadPlan {
    pub(crate) fn local_path_for_target(
        &self,
        layer_name: &str,
        module_path: &str,
        sha256: &str,
    ) -> Option<&str> {
        self.local_files
            .iter()
            .find(|f| {
                f.layer_name == layer_name && f.module_path == module_path && f.sha256 == sha256
            })
            .map(|f| f.output_path.as_str())
    }
}

pub(crate) fn static_layer_dirs(manifest: &Manifest) -> Vec<String> {
    manifest
        .layers
        .iter()
        .filter(|l| l.target == LayerTarget::Static)
        .map(|l| normalized_layer_dir(&l.directory))
        .collect()
}

pub(crate) fn files_in_dirs(files: &[FileEntry], dirs: &[String]) -> Vec<FileEntry> {
    files
        .iter()
        .filter(|f| is_in_layer_dirs(&f.path, dirs))
        .cloned()
        .collect()
}

pub(crate) fn build_static_pack_plan(
    output_dir: &Path,
    files: &[FileEntry],
) -> anyhow::Result<PackPlan> {
    let mut parts: Vec<PartBuilder> = Vec::new();
    let mut current = PartBuilder::new(0);
    let mut stored_ranges: HashMap<(String, u64), StoredPackRange> = HashMap::new();
    let mut summary_paths = Vec::with_capacity(files.len());
    let mut total_logical_bytes = 0u64;

    for file in files {
        total_logical_bytes = total_logical_bytes
            .checked_add(file.size)
            .context("static logical size overflow")?;

        let content_key = (file.content_hash.clone(), file.size);
        if let Some(stored) = stored_ranges.get(&content_key) {
            summary_paths.push(PackPathDescriptor {
                path: file.path.clone(),
                sha256: stored.sha256.clone(),
                size: file.size,
                content_type: content_type_from_path(&file.path).map(str::to_string),
                part_index: stored.part_index,
                offset: stored.offset,
                length: stored.length,
            });
            continue;
        }

        if file.size > PACK_PART_TARGET_BYTES {
            if !current.is_empty() {
                parts.push(current);
            }
            let mut oversized = PartBuilder::new(parts.len() as u32);
            let stored = StoredPackRange {
                part_index: oversized.part_index,
                offset: 0,
                length: file.size,
                sha256: file.content_hash.clone(),
            };
            oversized.push(file.clone());
            parts.push(oversized);
            summary_paths.push(PackPathDescriptor {
                path: file.path.clone(),
                sha256: file.content_hash.clone(),
                size: file.size,
                content_type: content_type_from_path(&file.path).map(str::to_string),
                part_index: stored.part_index,
                offset: stored.offset,
                length: stored.length,
            });
            stored_ranges.insert(content_key, stored);
            current = PartBuilder::new(parts.len() as u32);
            continue;
        }

        if !current.is_empty() && current.size.saturating_add(file.size) > PACK_PART_TARGET_BYTES {
            parts.push(current);
            current = PartBuilder::new(parts.len() as u32);
        }
        let stored = StoredPackRange {
            part_index: current.part_index,
            offset: current.size,
            length: file.size,
            sha256: file.content_hash.clone(),
        };
        current.push(file.clone());
        summary_paths.push(PackPathDescriptor {
            path: file.path.clone(),
            sha256: file.content_hash.clone(),
            size: file.size,
            content_type: content_type_from_path(&file.path).map(str::to_string),
            part_index: stored.part_index,
            offset: stored.offset,
            length: stored.length,
        });
        stored_ranges.insert(content_key, stored);
    }

    if !current.is_empty() || parts.is_empty() {
        parts.push(current);
    }

    let mut summary_parts = Vec::with_capacity(parts.len());
    let mut part_plans = Vec::with_capacity(parts.len());

    for builder in parts {
        let part_index = builder.part_index;
        let mut ranges = Vec::with_capacity(builder.files.len());
        let mut offset = 0u64;
        for file in builder.files {
            ranges.push(PackPartRange {
                path: file.path,
                offset,
                size: file.size,
                sha256: file.content_hash,
            });
            offset = offset
                .checked_add(file.size)
                .context("pack part size overflow")?;
        }

        let (sha256, chunks) = hash_pack_part(output_dir, &ranges, offset)
            .with_context(|| format!("failed to hash PACK_V1 part {part_index}"))?;
        summary_parts.push(PackPartDescriptor {
            part_index,
            size: offset,
            sha256: sha256.clone(),
            chunks,
        });
        part_plans.push(PackPartPlan {
            part_index,
            size: offset,
            sha256,
            ranges,
        });
    }

    summary_paths.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    summary_parts.sort_unstable_by_key(|p| p.part_index);
    part_plans.sort_unstable_by_key(|p| p.part_index);

    Ok(PackPlan {
        summary: ManifestSummary {
            file_count: summary_paths.len(),
            total_logical_bytes: total_logical_bytes.to_string(),
            paths: summary_paths,
            pack_parts: summary_parts,
        },
        parts: part_plans,
        total_logical_bytes,
    })
}

pub(crate) fn build_isolate_upload_plan(
    output_dir: &Path,
    manifest: &Manifest,
    all_files: &[FileEntry],
) -> anyhow::Result<IsolateUploadPlan> {
    let mut modules = Vec::new();
    let mut local_files = Vec::new();

    for layer in manifest
        .layers
        .iter()
        .filter(|l| l.target == LayerTarget::Isolate)
    {
        let layer_dir = normalized_layer_dir(&layer.directory);
        let mut module_files = Vec::new();

        for file in files_in_dirs(all_files, std::slice::from_ref(&layer_dir)) {
            let module_path = strip_layer_prefix(&file.path, &layer_dir)
                .with_context(|| format!("failed to resolve ISOLATE path for {}", file.path))?;
            let chunks = multipart_chunks_for_file(
                &output_dir.join(&file.path),
                file.size,
                MULTIPART_THRESHOLD_BYTES,
            )
            .with_context(|| format!("failed to hash multipart chunks for {}", file.path))?;
            module_files.push(IsolateModuleFileUpload {
                path: module_path.clone(),
                sha256: file.content_hash.clone(),
                size: file.size,
                chunks,
            });
            local_files.push(IsolateLocalFile {
                layer_name: layer.name.clone(),
                module_path,
                output_path: file.path,
                sha256: file.content_hash,
            });
        }

        module_files.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        if module_files.is_empty() {
            bail!(
                "ISOLATE layer '{}' has no files under directory '{}'",
                layer.name,
                layer.directory
            );
        }
        modules.push(IsolateModuleUpload {
            layer_name: layer.name.clone(),
            files: module_files,
        });
    }

    modules.sort_unstable_by(|a, b| a.layer_name.cmp(&b.layer_name));

    Ok(IsolateUploadPlan {
        modules,
        local_files,
    })
}

pub(crate) fn build_compute_bundle_uploads(
    manifest: &Manifest,
    bundle_sha256: &str,
    bundle_size: u64,
    bundle_bytes: Option<&[u8]>,
) -> anyhow::Result<Vec<ComputeBundleUpload>> {
    let chunks = match bundle_bytes {
        Some(bytes) if bundle_size >= MULTIPART_THRESHOLD_BYTES => {
            Some(multipart_chunks_for_bytes(bytes, MULTIPART_CHUNK_BYTES)?)
        }
        _ => None,
    };

    let mut out: Vec<_> = manifest
        .layers
        .iter()
        .filter(|l| l.target == LayerTarget::Compute)
        .map(|layer| ComputeBundleUpload {
            layer_name: layer.name.clone(),
            bundle_sha256: bundle_sha256.to_string(),
            size: bundle_size.to_string(),
            chunks: chunks.clone(),
        })
        .collect();
    out.sort_unstable_by(|a, b| a.layer_name.cmp(&b.layer_name));
    Ok(out)
}

pub(crate) async fn read_pack_part_bytes(
    output_dir: &Path,
    part: &PackPartPlan,
) -> anyhow::Result<Bytes> {
    let capacity = usize::try_from(part.size).context("pack part too large for this platform")?;
    let mut bytes = Vec::with_capacity(capacity);
    for range in &part.ranges {
        let path = output_dir.join(&range.path);
        let data = tokio::fs::read(&path)
            .await
            .with_context(|| format!("failed to read {}", path.display()))?;
        if data.len() as u64 != range.size {
            bail!(
                "size drifted between scan and upload for {} (scanned {} bytes, now {} bytes)",
                range.path,
                range.size,
                data.len()
            );
        }
        bytes.extend_from_slice(&data);
    }
    verify_pack_part_bytes(part, &bytes)?;
    Ok(Bytes::from(bytes))
}

pub(crate) async fn read_pack_part_chunk_bytes(
    output_dir: &Path,
    part: &PackPartPlan,
    offset: u64,
    size: u64,
) -> anyhow::Result<Bytes> {
    let end = offset.checked_add(size).context("chunk range overflow")?;
    if end > part.size {
        bail!(
            "multipart chunk range [{offset}, {end}) exceeds pack part {} size {}",
            part.part_index,
            part.size
        );
    }
    let mut bytes =
        Vec::with_capacity(usize::try_from(size).context("chunk too large for this platform")?);
    for range in &part.ranges {
        let range_end = range
            .offset
            .checked_add(range.size)
            .context("pack range overflow")?;
        let overlap_start = offset.max(range.offset);
        let overlap_end = end.min(range_end);
        if overlap_start >= overlap_end {
            continue;
        }
        let local_offset = overlap_start - range.offset;
        let local_size = overlap_end - overlap_start;
        bytes.extend_from_slice(
            &read_file_slice(&output_dir.join(&range.path), local_offset, local_size).await?,
        );
    }
    if bytes.len() as u64 != size {
        bail!(
            "multipart chunk materialization for pack part {} produced {} bytes, expected {}",
            part.part_index,
            bytes.len(),
            size
        );
    }
    Ok(Bytes::from(bytes))
}

pub(crate) async fn read_file_slice(path: &Path, offset: u64, size: u64) -> anyhow::Result<Bytes> {
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

fn verify_pack_part_bytes(part: &PackPartPlan, bytes: &[u8]) -> anyhow::Result<()> {
    if bytes.len() as u64 != part.size {
        bail!(
            "pack part {} materialized {} bytes, expected {}",
            part.part_index,
            bytes.len(),
            part.size
        );
    }
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    if sha256 != part.sha256 {
        bail!(
            "pack part {} SHA drifted between planning and upload (planned {}, now {})",
            part.part_index,
            part.sha256,
            sha256
        );
    }
    Ok(())
}

fn hash_pack_part(
    output_dir: &Path,
    ranges: &[PackPartRange],
    part_size: u64,
) -> anyhow::Result<(String, Option<Vec<MultipartChunkDescriptor>>)> {
    let mut part_hasher = Sha256::new();
    let mut chunk_builder = if part_size >= MULTIPART_THRESHOLD_BYTES {
        Some(ChunkHasher::new())
    } else {
        None
    };

    for range in ranges {
        let path = output_dir.join(&range.path);
        let mut file = std::fs::File::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut file_hasher = Sha256::new();
        let mut remaining = range.size;
        let mut buf = vec![0u8; READ_CHUNK_BYTES];

        while remaining > 0 {
            let read_cap = remaining.min(buf.len() as u64) as usize;
            let n = file
                .read(&mut buf[..read_cap])
                .with_context(|| format!("failed to read {}", path.display()))?;
            if n == 0 {
                bail!(
                    "file {} ended early while hashing pack part (expected {} more bytes)",
                    range.path,
                    remaining
                );
            }
            let chunk = &buf[..n];
            part_hasher.update(chunk);
            file_hasher.update(chunk);
            if let Some(builder) = chunk_builder.as_mut() {
                builder.update(chunk);
            }
            remaining -= n as u64;
        }

        let file_sha = format!("{:x}", file_hasher.finalize());
        if file_sha != range.sha256 {
            bail!(
                "file {} SHA drifted between scan and pack planning (scanned {}, now {})",
                range.path,
                range.sha256,
                file_sha
            );
        }
    }

    let chunks = chunk_builder.map(ChunkHasher::finish).transpose()?;
    Ok((format!("{:x}", part_hasher.finalize()), chunks))
}

pub(crate) fn multipart_chunks_for_bytes(
    bytes: &[u8],
    chunk_size: u64,
) -> anyhow::Result<Vec<MultipartChunkDescriptor>> {
    let mut chunks = Vec::new();
    let mut offset = 0usize;
    let chunk_size = usize::try_from(chunk_size).context("multipart chunk size too large")?;
    while offset < bytes.len() {
        let end = bytes.len().min(offset + chunk_size);
        chunks.push(MultipartChunkDescriptor {
            part_number: (chunks.len() + 1)
                .try_into()
                .context("too many multipart chunks")?,
            offset: offset as u64,
            size: (end - offset) as u64,
            sha256: format!("{:x}", Sha256::digest(&bytes[offset..end])),
        });
        offset = end;
    }
    Ok(chunks)
}

fn multipart_chunks_for_file(
    path: &Path,
    size: u64,
    threshold: u64,
) -> anyhow::Result<Option<Vec<MultipartChunkDescriptor>>> {
    if size < threshold {
        return Ok(None);
    }
    let mut builder = ChunkHasher::new();
    let mut file =
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut remaining = size;
    let mut buf = vec![0u8; READ_CHUNK_BYTES];
    while remaining > 0 {
        let read_cap = remaining.min(buf.len() as u64) as usize;
        let n = file
            .read(&mut buf[..read_cap])
            .with_context(|| format!("failed to read {}", path.display()))?;
        if n == 0 {
            bail!(
                "file {} ended early while hashing multipart chunks",
                path.display()
            );
        }
        builder.update(&buf[..n]);
        remaining -= n as u64;
    }
    Ok(Some(builder.finish()?))
}

struct ChunkHasher {
    chunks: Vec<MultipartChunkDescriptor>,
    current: Sha256,
    current_offset: u64,
    current_size: u64,
}

impl ChunkHasher {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            current: Sha256::new(),
            current_offset: 0,
            current_size: 0,
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let remaining_in_chunk = (MULTIPART_CHUNK_BYTES - self.current_size) as usize;
            let take = bytes.len().min(remaining_in_chunk);
            self.current.update(&bytes[..take]);
            self.current_size += take as u64;
            bytes = &bytes[take..];
            if self.current_size == MULTIPART_CHUNK_BYTES {
                self.flush_current();
            }
        }
    }

    fn finish(mut self) -> anyhow::Result<Vec<MultipartChunkDescriptor>> {
        if self.current_size > 0 {
            self.flush_current();
        }
        if self.chunks.is_empty() {
            bail!("multipart target produced zero chunks");
        }
        Ok(self.chunks)
    }

    fn flush_current(&mut self) {
        let hasher = std::mem::take(&mut self.current);
        self.chunks.push(MultipartChunkDescriptor {
            part_number: self.chunks.len() as u32 + 1,
            offset: self.current_offset,
            size: self.current_size,
            sha256: format!("{:x}", hasher.finalize()),
        });
        self.current_offset += self.current_size;
        self.current_size = 0;
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub(crate) enum PresignedUpload {
    Single {
        url: String,
        #[serde(rename = "contentLength")]
        content_length: u64,
        sha256: String,
        #[serde(default)]
        headers: PresignedPutHeaders,
    },
    Multipart {
        #[serde(rename = "uploadId")]
        upload_id: String,
        #[allow(dead_code)]
        #[serde(rename = "chunkSize")]
        chunk_size: u64,
        chunks: Vec<PresignedMultipartChunk>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresignedMultipartChunk {
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

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum MultipartCompleteTarget {
    PackPart {
        #[serde(rename = "partIndex")]
        part_index: u32,
        #[serde(rename = "uploadId")]
        upload_id: String,
        parts: Vec<CompletedMultipartPart>,
    },
    ComputeBundle {
        #[serde(rename = "layerName")]
        layer_name: String,
        #[serde(rename = "bundleSha256")]
        bundle_sha256: String,
        #[serde(rename = "uploadId")]
        upload_id: String,
        parts: Vec<CompletedMultipartPart>,
    },
    IsolateModule {
        #[serde(rename = "layerName")]
        layer_name: String,
        path: String,
        #[serde(rename = "uploadId")]
        upload_id: String,
        parts: Vec<CompletedMultipartPart>,
    },
}

#[derive(Default)]
struct PartBuilder {
    part_index: u32,
    size: u64,
    files: Vec<FileEntry>,
}

impl PartBuilder {
    fn new(part_index: u32) -> Self {
        Self {
            part_index,
            size: 0,
            files: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    fn push(&mut self, file: FileEntry) {
        self.size += file.size;
        self.files.push(file);
    }
}

fn normalized_layer_dir(dir: &str) -> String {
    let trimmed = dir.trim_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        ".".to_string()
    } else {
        format!("{trimmed}/")
    }
}

fn strip_layer_prefix(path: &str, layer_dir: &str) -> anyhow::Result<String> {
    if layer_dir == "." {
        return Ok(path.to_string());
    }
    path.strip_prefix(layer_dir)
        .map(str::to_string)
        .with_context(|| format!("{path} is not under layer directory {layer_dir}"))
}

fn is_in_layer_dirs(rel_path: &str, dirs: &[String]) -> bool {
    dirs.iter().any(|d| {
        if d == "." {
            true
        } else {
            rel_path.starts_with(d.as_str())
        }
    })
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
