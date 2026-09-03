// @generated vendored copy of platform crates/nrz-source-bundle/src/verifier.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_compression::tokio::bufread::ZstdDecoder;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::io::StreamReader;

use crate::manifest::{
    SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceBundleSummary, SourceLogicalManifest,
    SourceLogicalManifestEntryType, SourceLogicalManifestFile,
    canonical_source_logical_manifest_json, compute_logical_manifest_sha256,
    compute_source_artifact_id, normalize_source_path, sha256_hex, source_runtime_readiness,
    summarize_logical_manifest,
};

const TAR_BLOCK_SIZE: usize = 512;
const TAR_NAME_LENGTH: usize = 100;
const TAR_SIZE_OFFSET: usize = 124;
const TAR_SIZE_LENGTH: usize = 12;
const TAR_MODE_OFFSET: usize = 100;
const TAR_MODE_LENGTH: usize = 8;
const TAR_FILE_MODE: usize = 0o644;
const TAR_EXECUTABLE_FILE_MODE: usize = 0o755;
const TAR_SYMLINK_MODE: usize = 0o777;
const TAR_TYPEFLAG_OFFSET: usize = 156;
const TAR_LINKNAME_OFFSET: usize = 157;
const TAR_LINKNAME_LENGTH: usize = 100;
const TAR_PREFIX_OFFSET: usize = 345;
const TAR_PREFIX_LENGTH: usize = 155;
const TAR_END_BLOCK_COUNT: u64 = 2;
const METADATA_PREFIX: &str = ".__onreza/";
pub const SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH: &str = ".__onreza/logical-manifest.json";
const MAX_METADATA_OVERHEAD_BYTES: u64 = 32 * 1024 * 1024;
const MAX_LOGICAL_MANIFEST_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SourceBundleVerificationInput {
    pub owner_workspace_id: String,
    pub source_artifact_id: String,
    pub source_sha256: String,
    pub logical_manifest_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceBundleVerificationSummary {
    pub file_count: i32,
    pub logical_static_bytes: i64,
    pub artifact_size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBundleVerificationResult {
    pub summary: SourceBundleVerificationSummary,
    pub logical_manifest: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBundleVerificationFailure {
    pub error_code: String,
    pub message: String,
    pub details: Option<Value>,
}

impl fmt::Display for SourceBundleVerificationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_code, self.message)
    }
}

impl std::error::Error for SourceBundleVerificationFailure {}

pub async fn verify_source_bundle_bytes(
    input: SourceBundleVerificationInput,
    compressed_bytes: Bytes,
) -> Result<SourceBundleVerificationResult, SourceBundleVerificationFailure> {
    verify_source_bundle_stream(
        input,
        futures::stream::iter([Ok::<Bytes, io::Error>(compressed_bytes)]),
    )
    .await
}

pub async fn verify_source_bundle_stream<S, E>(
    input: SourceBundleVerificationInput,
    compressed_stream: S,
) -> Result<SourceBundleVerificationResult, SourceBundleVerificationFailure>
where
    S: Stream<Item = Result<Bytes, E>> + Send + Unpin + 'static,
    E: fmt::Display + Send + Sync + 'static,
{
    let source_hasher = Arc::new(Mutex::new(Sha256::new()));
    let hasher = Arc::clone(&source_hasher);
    let stream = compressed_stream.map(move |item| match item {
        Ok(bytes) => {
            hasher
                .lock()
                .expect("source bundle hasher mutex poisoned")
                .update(&bytes);
            Ok::<Bytes, io::Error>(bytes)
        }
        Err(error) => Err(io::Error::other(error.to_string())),
    });
    let reader = StreamReader::new(stream);
    let mut decoder = ZstdDecoder::new(BufReader::new(reader));
    let mut tar_verifier = StreamingTarVerifier::new(input.clone());
    let mut buffer = vec![0_u8; 64 * 1024];

    loop {
        match decoder.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => tar_verifier.push(&buffer[..n])?,
            Err(error) => {
                if error.kind() == io::ErrorKind::Other {
                    return Err(failure(
                        "SOURCE_OBJECT_READ_FAILED",
                        "SOURCE_BUNDLE_V1 source object stream failed during verification",
                    ));
                }
                return Err(failure(
                    "SOURCE_ZSTD_DECODE_FAILED",
                    format!("SOURCE_BUNDLE_V1 archive is not valid zstd: {error}"),
                ));
            }
        }
    }

    let actual_source_sha = {
        let hasher = source_hasher
            .lock()
            .expect("source bundle hasher mutex poisoned");
        hex::encode(hasher.clone().finalize())
    };
    if actual_source_sha != input.source_sha256 {
        return Err(failure(
            "SOURCE_SHA_MISMATCH",
            "Uploaded source object SHA-256 does not match prepare-upload metadata",
        ));
    }

    tar_verifier.finish()
}

struct PreparedVerification {
    logical_manifest: Value,
    summary: SourceBundleSummary,
    expected: HashMap<String, SourceLogicalManifestFile>,
    max_expanded_bytes: u64,
}

fn prepare_verification(
    input: &SourceBundleVerificationInput,
    logical_manifest: Value,
    logical_manifest_body_bytes: u64,
) -> Result<PreparedVerification, SourceBundleVerificationFailure> {
    let actual_logical_manifest_sha = compute_logical_manifest_sha256(&logical_manifest);
    if actual_logical_manifest_sha != input.logical_manifest_sha256 {
        return Err(failure(
            "SOURCE_LOGICAL_MANIFEST_SHA_MISMATCH",
            "Logical manifest SHA-256 does not match manifest payload",
        ));
    }

    let manifest: SourceLogicalManifest = serde_json::from_value(logical_manifest.clone())
        .map_err(|error| {
            failure(
                "SOURCE_LOGICAL_MANIFEST_INVALID",
                format!("Logical manifest payload is invalid: {error}"),
            )
        })?;
    if manifest.schema_version != SOURCE_BUNDLE_V1_SCHEMA_VERSION {
        return Err(failure(
            "SOURCE_LOGICAL_MANIFEST_SCHEMA_UNSUPPORTED",
            format!(
                "Unsupported logical manifest schemaVersion: {}",
                manifest.schema_version
            ),
        ));
    }
    source_runtime_readiness(&manifest).map_err(|message| {
        failure(
            "SOURCE_RUNTIME_READINESS_INVALID",
            format!("Logical manifest runtime readiness is invalid: {message}"),
        )
    })?;
    let computed_source_artifact_id = compute_source_artifact_id(
        &input.owner_workspace_id,
        &input.logical_manifest_sha256,
        &input.source_sha256,
        Some(&manifest.schema_version),
    );
    if computed_source_artifact_id != input.source_artifact_id {
        return Err(failure(
            "SOURCE_ARTIFACT_ID_MISMATCH",
            "Source artifact id does not match server recomputation",
        ));
    }

    let summary = summarize_logical_manifest(&manifest);
    let mut expected = HashMap::new();
    for file in manifest.files {
        let normalized = normalize_source_path(&file.path)
            .map_err(|message| failure("SOURCE_MANIFEST_PATH_INVALID", message))?;
        validate_manifest_entry(&normalized, &file)?;
        if expected.insert(normalized.clone(), file).is_some() {
            return Err(failure(
                "SOURCE_MANIFEST_DUPLICATE_PATH",
                format!("Duplicate logical manifest path: {normalized}"),
            ));
        }
    }
    validate_manifest_symlinks(&expected)?;

    let logical_static_bytes = u64::try_from(summary.logical_static_bytes).unwrap_or(u64::MAX);
    let artifact_size_bytes = u64::try_from(summary.artifact_size_bytes).unwrap_or(u64::MAX);
    Ok(PreparedVerification {
        summary,
        max_expanded_bytes: logical_static_bytes
            .saturating_add(artifact_size_bytes)
            .saturating_add(tar_structural_overhead_bytes(&expected))
            .saturating_add(TAR_BLOCK_SIZE as u64)
            .saturating_add(logical_manifest_body_bytes)
            .saturating_add(tar_padding_bytes(logical_manifest_body_bytes))
            .saturating_add(MAX_METADATA_OVERHEAD_BYTES),
        expected,
        logical_manifest,
    })
}

fn validate_manifest_entry(
    path: &str,
    file: &SourceLogicalManifestFile,
) -> Result<(), SourceBundleVerificationFailure> {
    match file.entry_type {
        SourceLogicalManifestEntryType::File => {
            if file.link_target.is_some() {
                return Err(failure(
                    "SOURCE_MANIFEST_LINK_INVALID",
                    format!("Regular file declares linkTarget: {path}"),
                ));
            }
        }
        SourceLogicalManifestEntryType::Symlink => {
            let link_target = file.link_target.as_deref().ok_or_else(|| {
                failure(
                    "SOURCE_MANIFEST_LINK_INVALID",
                    format!("Symlink manifest entry missing linkTarget: {path}"),
                )
            })?;
            if file.size != 0 {
                return Err(failure(
                    "SOURCE_MANIFEST_LINK_INVALID",
                    format!("Symlink manifest entry must have size 0: {path}"),
                ));
            }
            validate_symlink_target(path, link_target)
                .map_err(|message| failure("SOURCE_MANIFEST_LINK_TARGET_UNSAFE", message))?;
            if sha256_hex(link_target.as_bytes()) != file.sha256 {
                return Err(failure(
                    "SOURCE_MANIFEST_LINK_SHA_MISMATCH",
                    format!("Symlink target SHA-256 mismatch: {path}"),
                ));
            }
        }
    }
    Ok(())
}

fn tar_structural_overhead_bytes(expected: &HashMap<String, SourceLogicalManifestFile>) -> u64 {
    let mut overhead = (TAR_BLOCK_SIZE as u64).saturating_mul(TAR_END_BLOCK_COUNT);
    for (path, file) in expected {
        overhead = overhead
            .saturating_add(TAR_BLOCK_SIZE as u64)
            .saturating_add(tar_padding_bytes(file.size));
        let needs_pax_path = path.len() > TAR_NAME_LENGTH;
        let needs_pax_link_path = file.entry_type == SourceLogicalManifestEntryType::Symlink
            && file
                .link_target
                .as_ref()
                .is_some_and(|target| target.len() > TAR_LINKNAME_LENGTH);
        if needs_pax_path || needs_pax_link_path {
            let mut pax_body_bytes = 0_u64;
            if needs_pax_path {
                pax_body_bytes = pax_body_bytes.saturating_add(pax_record_byte_len("path", path));
            }
            if needs_pax_link_path && let Some(link_target) = file.link_target.as_deref() {
                pax_body_bytes =
                    pax_body_bytes.saturating_add(pax_record_byte_len("linkpath", link_target));
            }
            overhead = overhead
                .saturating_add(TAR_BLOCK_SIZE as u64)
                .saturating_add(pax_body_bytes)
                .saturating_add(tar_padding_bytes(pax_body_bytes));
        }
    }
    overhead
}

fn tar_padding_bytes(size: u64) -> u64 {
    let block_size = TAR_BLOCK_SIZE as u64;
    let remainder = size % block_size;
    if remainder == 0 {
        0
    } else {
        block_size - remainder
    }
}

fn pax_record_byte_len(key: &str, value: &str) -> u64 {
    let body = format!("{key}={value}\n");
    let mut length = format!("0 {body}").len();
    loop {
        let record = format!("{length} {body}");
        let byte_length = record.len();
        if byte_length == length {
            return u64::try_from(byte_length).unwrap_or(u64::MAX);
        }
        length = byte_length;
    }
}

fn validate_manifest_symlinks(
    expected: &HashMap<String, SourceLogicalManifestFile>,
) -> Result<(), SourceBundleVerificationFailure> {
    for (path, file) in expected {
        if file.entry_type != SourceLogicalManifestEntryType::Symlink {
            continue;
        }
        let link_target = file.link_target.as_deref().ok_or_else(|| {
            failure(
                "SOURCE_MANIFEST_LINK_INVALID",
                format!("Symlink manifest entry missing linkTarget: {path}"),
            )
        })?;
        let resolved = resolve_symlink_target(path, link_target)
            .map_err(|message| failure("SOURCE_MANIFEST_LINK_TARGET_UNSAFE", message))?;
        validate_raw_symlink_target_semantics(expected, path, link_target)?;
        if !manifest_resolves_to_file(expected, &resolved, &mut HashSet::from([path.clone()])) {
            return Err(failure(
                "SOURCE_MANIFEST_LINK_TARGET_MISSING",
                format!("Symlink target is not present in manifest: {path}"),
            ));
        }
        let nested_prefix = format!("{path}/");
        if expected
            .keys()
            .any(|candidate| candidate.starts_with(&nested_prefix))
        {
            return Err(failure(
                "SOURCE_MANIFEST_LINK_NESTED_ENTRY",
                format!("Symlink path contains nested logical entry: {path}"),
            ));
        }
    }
    Ok(())
}

fn manifest_resolves_to_file(
    expected: &HashMap<String, SourceLogicalManifestFile>,
    path: &str,
    seen_symlinks: &mut HashSet<String>,
) -> bool {
    if let Some(file) = expected.get(path) {
        return match file.entry_type {
            SourceLogicalManifestEntryType::File => true,
            SourceLogicalManifestEntryType::Symlink => {
                let Some(link_target) = file.link_target.as_deref() else {
                    return false;
                };
                if !seen_symlinks.insert(path.to_string()) {
                    return false;
                }
                let Ok(resolved) = resolve_symlink_target(path, link_target) else {
                    return false;
                };
                manifest_resolves_to_file(expected, &resolved, seen_symlinks)
            }
        };
    }

    for prefix in path_prefixes_from_deepest(path) {
        let Some(file) = expected.get(&prefix) else {
            continue;
        };
        if file.entry_type != SourceLogicalManifestEntryType::Symlink {
            continue;
        }
        let Some(link_target) = file.link_target.as_deref() else {
            return false;
        };
        if !seen_symlinks.insert(prefix.clone()) {
            return false;
        }
        let Ok(resolved) = resolve_symlink_target(&prefix, link_target) else {
            return false;
        };
        let suffix = &path[(prefix.len() + 1)..];
        return manifest_resolves_to_file(expected, &format!("{resolved}/{suffix}"), seen_symlinks);
    }

    expected.keys().any(|candidate| {
        candidate
            .strip_prefix(path)
            .is_some_and(|suffix| suffix.starts_with('/'))
            && manifest_resolves_to_file(expected, candidate, &mut seen_symlinks.clone())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManifestPathKind {
    File,
    Directory,
    Missing,
}

fn validate_raw_symlink_target_semantics(
    expected: &HashMap<String, SourceLogicalManifestFile>,
    path: &str,
    link_target: &str,
) -> Result<(), SourceBundleVerificationFailure> {
    let mut parts: Vec<&str> = path.rsplit_once('/').map_or(Vec::new(), |(parent, _)| {
        parent
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    });

    for segment in link_target.split('/') {
        let current_path = parts.join("/");
        if !current_path.is_empty()
            && manifest_path_kind(
                expected,
                &current_path,
                &mut HashSet::from([path.to_string()]),
            ) == ManifestPathKind::File
        {
            return Err(failure(
                "SOURCE_MANIFEST_LINK_TARGET_UNSAFE",
                format!("Symlink target traverses through a regular file: {path} -> {link_target}"),
            ));
        }
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(failure(
                        "SOURCE_MANIFEST_LINK_TARGET_UNSAFE",
                        format!("Symlink target escapes archive root: {path} -> {link_target}"),
                    ));
                }
            }
            normal => parts.push(normal),
        }
    }
    Ok(())
}

fn manifest_path_kind(
    expected: &HashMap<String, SourceLogicalManifestFile>,
    path: &str,
    seen_symlinks: &mut HashSet<String>,
) -> ManifestPathKind {
    if let Some(file) = expected.get(path) {
        return match file.entry_type {
            SourceLogicalManifestEntryType::File => ManifestPathKind::File,
            SourceLogicalManifestEntryType::Symlink => {
                let Some(link_target) = file.link_target.as_deref() else {
                    return ManifestPathKind::Missing;
                };
                if !seen_symlinks.insert(path.to_string()) {
                    return ManifestPathKind::Missing;
                }
                let Ok(resolved) = resolve_symlink_target(path, link_target) else {
                    return ManifestPathKind::Missing;
                };
                manifest_path_kind(expected, &resolved, seen_symlinks)
            }
        };
    }

    if expected.keys().any(|candidate| {
        candidate
            .strip_prefix(path)
            .is_some_and(|suffix| suffix.starts_with('/'))
            && manifest_resolves_to_file(expected, candidate, &mut seen_symlinks.clone())
    }) {
        ManifestPathKind::Directory
    } else {
        ManifestPathKind::Missing
    }
}

fn path_prefixes_from_deepest(path: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut end = path.len();
    while let Some(index) = path[..end].rfind('/') {
        if index == 0 {
            break;
        }
        prefixes.push(path[..index].to_string());
        end = index;
    }
    prefixes
}

fn validate_symlink_target(path: &str, link_target: &str) -> Result<(), String> {
    resolve_symlink_target(path, link_target).map(|_| ())
}

fn resolve_symlink_target(path: &str, link_target: &str) -> Result<String, String> {
    if link_target.is_empty() || link_target.contains('\\') || link_target.contains('\0') {
        return Err(format!("Invalid symlink target for {path}: {link_target}"));
    }
    let target = Path::new(link_target);
    if target.is_absolute() {
        return Err(format!("Invalid symlink target for {path}: {link_target}"));
    }

    let mut resolved = PathBuf::new();
    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
    {
        resolved.push(parent);
    }
    for component in target.components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(format!(
                        "Symlink target escapes archive root: {path} -> {link_target}"
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Invalid symlink target for {path}: {link_target}"));
            }
        }
    }

    let resolved = resolved.to_string_lossy().replace('\\', "/");
    normalize_source_path(&resolved)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TarState {
    Header,
    Body,
    Padding,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Pax,
    File,
    LogicalManifest,
    Metadata,
}

struct StreamingEntry {
    kind: EntryKind,
    path: String,
    remaining: usize,
    padding_remaining: usize,
    pax_body: Vec<u8>,
    hasher: Option<Sha256>,
    expected_sha256: Option<String>,
}

struct StreamingTarVerifier {
    input: SourceBundleVerificationInput,
    state: TarState,
    buffer: BytesMut,
    entry: Option<StreamingEntry>,
    pending_pax_path: Option<String>,
    pending_pax_link_path: Option<String>,
    decompressed_bytes: u64,
    seen: HashSet<String>,
    prepared: Option<PreparedVerification>,
    manifest_seen: bool,
}

impl StreamingTarVerifier {
    fn new(input: SourceBundleVerificationInput) -> Self {
        Self {
            input,
            state: TarState::Header,
            buffer: BytesMut::new(),
            entry: None,
            pending_pax_path: None,
            pending_pax_link_path: None,
            decompressed_bytes: 0,
            seen: HashSet::new(),
            prepared: None,
            manifest_seen: false,
        }
    }

    fn push(&mut self, chunk: &[u8]) -> Result<(), SourceBundleVerificationFailure> {
        self.decompressed_bytes = self
            .decompressed_bytes
            .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        let max_expanded_bytes = self
            .prepared
            .as_ref()
            .map(|prepared| prepared.max_expanded_bytes)
            .unwrap_or_else(|| {
                MAX_LOGICAL_MANIFEST_BYTES.saturating_add((TAR_BLOCK_SIZE as u64) * 2)
            });
        if self.decompressed_bytes > max_expanded_bytes {
            return Err(failure(
                "SOURCE_EXPANSION_LIMIT",
                "SOURCE_BUNDLE_V1 archive expands beyond logical manifest budget",
            ));
        }
        self.buffer.extend_from_slice(chunk);
        self.process()
    }

    fn finish(
        &mut self,
    ) -> Result<SourceBundleVerificationResult, SourceBundleVerificationFailure> {
        self.process()?;
        if self.state != TarState::Done {
            return Err(failure(
                "SOURCE_TAR_TRUNCATED",
                "SOURCE_BUNDLE_V1 tar archive ended before a zero block",
            ));
        }
        let prepared = self.prepared.as_ref().ok_or_else(|| {
            failure(
                "SOURCE_LOGICAL_MANIFEST_MISSING",
                "SOURCE_BUNDLE_V1 archive is missing embedded logical manifest metadata",
            )
        })?;
        for path in prepared.expected.keys() {
            if !self.seen.contains(path) {
                return Err(failure(
                    "SOURCE_ARCHIVE_MISSING_ENTRY",
                    format!("Archive is missing logical manifest entry: {path}"),
                ));
            }
        }
        Ok(SourceBundleVerificationResult {
            summary: SourceBundleVerificationSummary {
                file_count: prepared.summary.file_count,
                logical_static_bytes: prepared.summary.logical_static_bytes,
                artifact_size_bytes: prepared.summary.artifact_size_bytes,
            },
            logical_manifest: prepared.logical_manifest.clone(),
        })
    }

    fn process(&mut self) -> Result<(), SourceBundleVerificationFailure> {
        loop {
            match self.state {
                TarState::Done => {
                    if contains_non_zero(&self.buffer) {
                        return Err(failure(
                            "SOURCE_TAR_TRAILING_DATA",
                            "Tar archive contains non-zero bytes after end marker",
                        ));
                    }
                    self.buffer.clear();
                    return Ok(());
                }
                TarState::Header => {
                    if self.buffer.len() < TAR_BLOCK_SIZE {
                        return Ok(());
                    }
                    let header = self.buffer.split_to(TAR_BLOCK_SIZE).freeze();
                    if is_zero_block(&header) {
                        self.state = TarState::Done;
                        continue;
                    }
                    self.start_entry(&header)?;
                }
                TarState::Body => {
                    self.consume_body()?;
                    if self.state == TarState::Body {
                        return Ok(());
                    }
                }
                TarState::Padding => {
                    let padding_remaining = self
                        .entry
                        .as_ref()
                        .ok_or_else(|| {
                            failure(
                                "SOURCE_TAR_HEADER_INVALID",
                                "Tar parser lost current entry state",
                            )
                        })?
                        .padding_remaining;
                    if self.buffer.len() < padding_remaining {
                        return Ok(());
                    }
                    let _ = self.buffer.split_to(padding_remaining);
                    self.entry = None;
                    self.state = TarState::Header;
                }
            }
        }
    }

    fn start_entry(&mut self, header: &[u8]) -> Result<(), SourceBundleVerificationFailure> {
        let type_flag = header.get(TAR_TYPEFLAG_OFFSET).copied().unwrap_or(0);
        let size = parse_octal(header, TAR_SIZE_OFFSET, TAR_SIZE_LENGTH).ok_or_else(|| {
            failure(
                "SOURCE_TAR_HEADER_INVALID",
                "Tar entry has invalid size header",
            )
        })?;
        let mode = parse_octal(header, TAR_MODE_OFFSET, TAR_MODE_LENGTH).ok_or_else(|| {
            failure(
                "SOURCE_TAR_HEADER_INVALID",
                "Tar entry has invalid mode header",
            )
        })?;
        let padding_remaining = padding_for(size);

        if type_flag == b'x' {
            require_tar_mode(mode, TAR_FILE_MODE, "PAX metadata")?;
            if !self.manifest_seen {
                return Err(failure(
                    "SOURCE_LOGICAL_MANIFEST_ORDER_INVALID",
                    "SOURCE_BUNDLE_V1 logical manifest metadata must be the first archive entry",
                ));
            }
            self.entry = Some(StreamingEntry {
                kind: EntryKind::Pax,
                path: String::new(),
                remaining: size,
                padding_remaining,
                pax_body: Vec::with_capacity(size.min(4096)),
                hasher: None,
                expected_sha256: None,
            });
            if size == 0 {
                self.finish_entry()?;
            } else {
                self.state = TarState::Body;
            }
            return Ok(());
        }

        if type_flag != b'0' && type_flag != 0 && type_flag != b'2' {
            let label = if type_flag == 0 {
                "NUL".to_string()
            } else {
                char::from(type_flag).to_string()
            };
            return Err(failure(
                "SOURCE_TAR_ENTRY_UNSUPPORTED",
                format!("Unsupported tar entry type: {label}"),
            ));
        }

        let raw_path = self
            .pending_pax_path
            .take()
            .unwrap_or_else(|| read_tar_path(header));
        let raw_link_target = self
            .pending_pax_link_path
            .take()
            .unwrap_or_else(|| read_tar_link_name(header));
        let normalized = normalize_source_path(&raw_path)
            .map_err(|message| failure("SOURCE_TAR_PATH_INVALID", message))?;
        if normalized == SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH {
            require_tar_mode(mode, TAR_FILE_MODE, "logical manifest")?;
            if self.manifest_seen {
                return Err(failure(
                    "SOURCE_LOGICAL_MANIFEST_DUPLICATE",
                    "SOURCE_BUNDLE_V1 archive contains duplicate logical manifest metadata",
                ));
            }
            if size as u64 > MAX_LOGICAL_MANIFEST_BYTES {
                return Err(failure(
                    "SOURCE_LOGICAL_MANIFEST_TOO_LARGE",
                    "SOURCE_BUNDLE_V1 embedded logical manifest exceeds the maximum metadata size",
                ));
            }
            self.manifest_seen = true;
            self.entry = Some(StreamingEntry {
                kind: EntryKind::LogicalManifest,
                path: normalized,
                remaining: size,
                padding_remaining,
                pax_body: Vec::with_capacity(size.min(4096)),
                hasher: None,
                expected_sha256: None,
            });
            if size == 0 {
                self.finish_entry()?;
            } else {
                self.state = TarState::Body;
            }
            return Ok(());
        }
        if !self.manifest_seen {
            return Err(failure(
                "SOURCE_LOGICAL_MANIFEST_ORDER_INVALID",
                "SOURCE_BUNDLE_V1 logical manifest metadata must be the first archive entry",
            ));
        }
        if normalized.starts_with(METADATA_PREFIX) {
            require_tar_mode(mode, TAR_FILE_MODE, "metadata")?;
            self.entry = Some(StreamingEntry {
                kind: EntryKind::Metadata,
                path: normalized,
                remaining: size,
                padding_remaining,
                pax_body: Vec::new(),
                hasher: None,
                expected_sha256: None,
            });
            if size == 0 {
                self.finish_entry()?;
            } else {
                self.state = TarState::Body;
            }
            return Ok(());
        }

        let prepared = self.prepared.as_ref().ok_or_else(|| {
            failure(
                "SOURCE_LOGICAL_MANIFEST_MISSING",
                "SOURCE_BUNDLE_V1 archive file entries appeared before embedded logical manifest",
            )
        })?;
        let file = prepared.expected.get(&normalized).ok_or_else(|| {
            failure(
                "SOURCE_ARCHIVE_UNDECLARED_ENTRY",
                format!("Archive contains undeclared entry: {normalized}"),
            )
        })?;
        if self.seen.contains(&normalized) {
            return Err(failure(
                "SOURCE_ARCHIVE_DUPLICATE_PATH",
                format!("Archive contains duplicate entry: {normalized}"),
            ));
        }
        if type_flag == b'2' {
            require_tar_mode(mode, TAR_SYMLINK_MODE, &normalized)?;
            if file.entry_type != SourceLogicalManifestEntryType::Symlink {
                return Err(failure(
                    "SOURCE_ARCHIVE_ENTRY_TYPE_MISMATCH",
                    format!("Archive entry type mismatch: {normalized}"),
                ));
            }
            if size != 0 || file.size != 0 {
                return Err(failure(
                    "SOURCE_TAR_SYMLINK_INVALID",
                    format!("Symlink entry must have size 0: {normalized}"),
                ));
            }
            validate_symlink_target(&normalized, &raw_link_target)
                .map_err(|message| failure("SOURCE_TAR_LINK_TARGET_UNSAFE", message))?;
            if Some(raw_link_target.as_str()) != file.link_target.as_deref() {
                return Err(failure(
                    "SOURCE_ARCHIVE_LINK_TARGET_MISMATCH",
                    format!("Archive symlink target mismatch: {normalized}"),
                ));
            }
            if sha256_hex(raw_link_target.as_bytes()) != file.sha256 {
                return Err(failure(
                    "SOURCE_ARCHIVE_LINK_SHA_MISMATCH",
                    format!("Archive symlink target SHA-256 mismatch: {normalized}"),
                ));
            }
            self.seen.insert(normalized);
            return Ok(());
        }
        if file.entry_type != SourceLogicalManifestEntryType::File {
            return Err(failure(
                "SOURCE_ARCHIVE_ENTRY_TYPE_MISMATCH",
                format!("Archive entry type mismatch: {normalized}"),
            ));
        }
        require_tar_mode(
            mode,
            if file.executable {
                TAR_EXECUTABLE_FILE_MODE
            } else {
                TAR_FILE_MODE
            },
            &normalized,
        )?;
        if size != usize::try_from(file.size).unwrap_or(usize::MAX) {
            return Err(failure(
                "SOURCE_ARCHIVE_SIZE_MISMATCH",
                format!("Archive entry size mismatch: {normalized}"),
            ));
        }

        self.entry = Some(StreamingEntry {
            kind: EntryKind::File,
            path: normalized,
            remaining: size,
            padding_remaining,
            pax_body: Vec::new(),
            hasher: Some(Sha256::new()),
            expected_sha256: Some(file.sha256.clone()),
        });
        if size == 0 {
            self.finish_entry()?;
        } else {
            self.state = TarState::Body;
        }
        Ok(())
    }

    fn consume_body(&mut self) -> Result<(), SourceBundleVerificationFailure> {
        let entry = self.entry.as_mut().ok_or_else(|| {
            failure(
                "SOURCE_TAR_HEADER_INVALID",
                "Tar parser lost current entry state",
            )
        })?;
        if self.buffer.is_empty() {
            return Ok(());
        }

        let take = entry.remaining.min(self.buffer.len());
        let body = self.buffer.split_to(take);
        entry.remaining -= take;
        match entry.kind {
            EntryKind::File => {
                if let Some(hasher) = &mut entry.hasher {
                    hasher.update(&body);
                }
            }
            EntryKind::Pax | EntryKind::LogicalManifest => entry.pax_body.extend_from_slice(&body),
            EntryKind::Metadata => {}
        }
        if entry.remaining == 0 {
            self.finish_entry()?;
        }
        Ok(())
    }

    fn finish_entry(&mut self) -> Result<(), SourceBundleVerificationFailure> {
        let entry = self.entry.as_mut().ok_or_else(|| {
            failure(
                "SOURCE_TAR_HEADER_INVALID",
                "Tar parser lost current entry state",
            )
        })?;
        match entry.kind {
            EntryKind::Pax => {
                let pax = parse_pax(&entry.pax_body)?;
                self.pending_pax_path = pax.path;
                self.pending_pax_link_path = pax.link_path;
            }
            EntryKind::LogicalManifest => {
                let logical_manifest: Value =
                    serde_json::from_slice(&entry.pax_body).map_err(|error| {
                        failure(
                            "SOURCE_LOGICAL_MANIFEST_INVALID",
                            format!("Embedded logical manifest is invalid JSON: {error}"),
                        )
                    })?;
                let canonical_manifest = canonical_source_logical_manifest_json(&logical_manifest);
                if canonical_manifest.as_bytes() != entry.pax_body.as_slice() {
                    return Err(failure(
                        "SOURCE_LOGICAL_MANIFEST_NOT_CANONICAL",
                        "Embedded logical manifest must use canonical SOURCE_BUNDLE_V1 JSON encoding",
                    ));
                }
                let prepared = prepare_verification(
                    &self.input,
                    logical_manifest,
                    u64::try_from(entry.pax_body.len()).unwrap_or(u64::MAX),
                )?;
                self.prepared = Some(prepared);
            }
            EntryKind::File => {
                let actual_file_sha = entry
                    .hasher
                    .take()
                    .map(|hasher| hex::encode(hasher.finalize()))
                    .unwrap_or_default();
                if Some(actual_file_sha.as_str()) != entry.expected_sha256.as_deref() {
                    return Err(failure(
                        "SOURCE_ARCHIVE_FILE_SHA_MISMATCH",
                        format!("Archive entry SHA-256 mismatch: {}", entry.path),
                    ));
                }
                self.seen.insert(entry.path.clone());
            }
            EntryKind::Metadata => {}
        }

        if entry.padding_remaining > 0 {
            self.state = TarState::Padding;
        } else {
            self.entry = None;
            self.state = TarState::Header;
        }
        Ok(())
    }
}

fn require_tar_mode(
    actual: usize,
    expected: usize,
    entry: &str,
) -> Result<(), SourceBundleVerificationFailure> {
    if actual != expected {
        return Err(failure(
            "SOURCE_ARCHIVE_MODE_MISMATCH",
            format!("Archive entry mode mismatch for {entry}"),
        ));
    }
    Ok(())
}

struct PaxHeaders {
    path: Option<String>,
    link_path: Option<String>,
}

fn parse_pax(body: &[u8]) -> Result<PaxHeaders, SourceBundleVerificationFailure> {
    let mut cursor = 0;
    let mut path = None;
    let mut link_path = None;

    while cursor < body.len() {
        let relative_space = body[cursor..].iter().position(|byte| *byte == b' ');
        let space = relative_space
            .map(|offset| cursor + offset)
            .ok_or_else(|| failure("SOURCE_TAR_PAX_INVALID", "Invalid PAX record length"))?;
        if space <= cursor {
            return Err(failure(
                "SOURCE_TAR_PAX_INVALID",
                "Invalid PAX record length",
            ));
        }
        let length_text = std::str::from_utf8(&body[cursor..space])
            .ok()
            .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| failure("SOURCE_TAR_PAX_INVALID", "Invalid PAX record length"))?;
        let length = length_text
            .parse::<usize>()
            .map_err(|_| failure("SOURCE_TAR_PAX_INVALID", "Invalid PAX record length"))?;
        let end = cursor.saturating_add(length);
        if length == 0 || end > body.len() || end <= space + 1 {
            return Err(failure(
                "SOURCE_TAR_PAX_INVALID",
                "Invalid PAX record length",
            ));
        }
        if body[end - 1] != b'\n' {
            return Err(failure(
                "SOURCE_TAR_PAX_INVALID",
                "PAX record missing newline",
            ));
        }

        let record = &body[(space + 1)..(end - 1)];
        let equals = record
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or_else(|| failure("SOURCE_TAR_PAX_INVALID", "Invalid PAX key/value record"))?;
        if equals == 0 {
            return Err(failure(
                "SOURCE_TAR_PAX_INVALID",
                "Invalid PAX key/value record",
            ));
        }
        let key = std::str::from_utf8(&record[..equals]).map_err(|_| {
            failure(
                "SOURCE_TAR_PAX_INVALID_UTF8",
                "PAX header is not valid UTF-8",
            )
        })?;
        let value = std::str::from_utf8(&record[(equals + 1)..]).map_err(|_| {
            failure(
                "SOURCE_TAR_PAX_INVALID_UTF8",
                "PAX header is not valid UTF-8",
            )
        })?;
        match key {
            "path" => path = Some(value.to_string()),
            "linkpath" => link_path = Some(value.to_string()),
            _ => {
                return Err(failure(
                    "SOURCE_TAR_PAX_UNSUPPORTED",
                    format!("Unsupported PAX key: {key}"),
                ));
            }
        }
        cursor = end;
    }

    Ok(PaxHeaders { path, link_path })
}

fn read_tar_path(header: &[u8]) -> String {
    let name = read_null_terminated(header, 0, TAR_NAME_LENGTH);
    let prefix = read_null_terminated(header, TAR_PREFIX_OFFSET, TAR_PREFIX_LENGTH);
    if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    }
}

fn read_tar_link_name(header: &[u8]) -> String {
    read_null_terminated(header, TAR_LINKNAME_OFFSET, TAR_LINKNAME_LENGTH)
}

fn read_null_terminated(bytes: &[u8], offset: usize, length: usize) -> String {
    let end = (offset + length).min(bytes.len());
    let slice = &bytes[offset..end];
    let nul = slice
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(slice.len());
    String::from_utf8_lossy(&slice[..nul]).to_string()
}

fn parse_octal(bytes: &[u8], offset: usize, length: usize) -> Option<usize> {
    let raw = read_null_terminated(bytes, offset, length);
    let trimmed = raw.trim();
    if !trimmed.bytes().all(|byte| (b'0'..=b'7').contains(&byte)) {
        return None;
    }
    if trimmed.is_empty() {
        Some(0)
    } else {
        usize::from_str_radix(trimmed, 8).ok()
    }
}

fn is_zero_block(block: &[u8]) -> bool {
    block.iter().all(|byte| *byte == 0)
}

fn padding_for(size: usize) -> usize {
    let rem = size % TAR_BLOCK_SIZE;
    if rem == 0 { 0 } else { TAR_BLOCK_SIZE - rem }
}

fn contains_non_zero(bytes: &[u8]) -> bool {
    bytes.iter().any(|byte| *byte != 0)
}

fn failure(
    error_code: impl Into<String>,
    message: impl Into<String>,
) -> SourceBundleVerificationFailure {
    SourceBundleVerificationFailure {
        error_code: error_code.into(),
        message: message.into(),
        details: None,
    }
}
