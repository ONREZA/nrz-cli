//! Legacy archive fixtures for retained reader and logical preparation tests.
use std::io::{Read, Write};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::*;
use crate::deploy::hash::sha256_finalize_hex;

const SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH: &str = ".__onreza/logical-manifest.json";

const ZSTD_LEVEL: i32 = 9;
const COPY_BUFFER_BYTES: usize = 65_536;
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
const TAR_EXECUTABLE_FILE_MODE: u64 = 0o755;
const TAR_SYMLINK_MODE: u64 = 0o777;
const TAR_SPACE: u8 = 0x20;

#[derive(Debug)]
pub(crate) struct SourceBundlePlan {
    pub(crate) logical_manifest: SourceLogicalManifest,
    pub(crate) logical_manifest_sha256: String,
    pub(crate) source_sha256: String,
    pub(crate) source_size_bytes: u64,
    source_path: PathBuf,
}

impl SourceBundlePlan {
    pub(crate) fn source_path(&self) -> &Path {
        &self.source_path
    }
}

impl Drop for SourceBundlePlan {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.source_path);
    }
}

pub(crate) fn build_source_bundle_plan(
    output_dir: &Path,
    manifest: &Manifest,
    files: &[FileEntry],
) -> anyhow::Result<SourceBundlePlan> {
    build_source_bundle_plan_with_scan(
        output_dir,
        manifest,
        files,
        &RuntimeArtifactScan::All,
        RuntimeDependencyPackaging::Embedded,
        None,
    )
}

pub(crate) fn build_source_bundle_plan_with_scan(
    output_dir: &Path,
    manifest: &Manifest,
    files: &[FileEntry],
    scan: &RuntimeArtifactScan,
    dependency_packaging: RuntimeDependencyPackaging,
    readiness: Option<RuntimeReadinessContract<'_>>,
) -> anyhow::Result<SourceBundlePlan> {
    build_source_bundle_plan_with_root(
        &ArtifactRoot::open(output_dir)?,
        manifest,
        files,
        scan,
        dependency_packaging,
        readiness,
        None,
    )
}

pub(crate) fn build_source_bundle_plan_with_root(
    root: &ArtifactRoot,
    manifest: &Manifest,
    files: &[FileEntry],
    scan: &RuntimeArtifactScan,
    dependency_packaging: RuntimeDependencyPackaging,
    readiness: Option<RuntimeReadinessContract<'_>>,
    runtime_policy_lineage: Option<(&str, &str)>,
) -> anyhow::Result<SourceBundlePlan> {
    let (entries, logical_manifest) = prepare_source_bundle_inventory(
        root,
        manifest,
        files,
        scan,
        dependency_packaging,
        readiness,
        runtime_policy_lineage,
    )?;
    let logical_manifest_json = canonical_logical_manifest_json(&logical_manifest)?;
    let logical_manifest_sha256 = sha256_hex(logical_manifest_json.as_bytes());

    let source_path =
        std::env::temp_dir().join(format!("nrz-source-bundle-{}.tar.zst", Uuid::now_v7()));
    let (source_sha256, source_size_bytes) =
        write_source_bundle(&source_path, logical_manifest_json.as_bytes(), &entries)?;

    Ok(SourceBundlePlan {
        logical_manifest,
        logical_manifest_sha256,
        source_sha256,
        source_size_bytes,
        source_path,
    })
}

pub(super) fn write_source_bundle(
    source_path: &Path,
    logical_manifest_json: &[u8],
    entries: &[SourceBundleEntry],
) -> anyhow::Result<(String, u64)> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(source_path)
        .with_context(|| format!("failed to create {}", source_path.display()))?;
    let result = encode_source_bundle(file, logical_manifest_json, entries);
    if result.is_err() {
        let _ = std::fs::remove_file(source_path);
    }
    result
}

fn encode_source_bundle(
    file: std::fs::File,
    logical_manifest_json: &[u8],
    entries: &[SourceBundleEntry],
) -> anyhow::Result<(String, u64)> {
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
    writer.write_all(&tar_header(path, body.len() as u64, b'0', None, false))?;
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
        let pax_body = nrz_source_bundle::encode_pax_record(&fields).into_bytes();
        writer.write_all(&tar_header(
            ".__onreza/pax",
            pax_body.len() as u64,
            b'x',
            None,
            false,
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
            false,
        ))?;
        return Ok(());
    }

    writer.write_all(&tar_header(
        if needs_pax_path { "file" } else { &entry.path },
        entry.size,
        b'0',
        None,
        entry.executable,
    ))?;
    let SourceBundleEntryKind::File { source_path, root } = &entry.kind else {
        unreachable!("symlink entries return before file copy")
    };
    let mut file = root
        .open_file(source_path)
        .with_context(|| format!("failed to open {}", source_path.display()))?;
    let mut remaining = entry.size;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    while remaining > 0 {
        let limit = remaining.min(COPY_BUFFER_BYTES as u64) as usize;
        let read = file
            .read(&mut buffer[..limit])
            .with_context(|| format!("failed to read {}", source_path.display()))?;
        if read == 0 {
            bail!(
                "SOURCE_BUNDLE_V1 file truncated during packaging: {}",
                entry.path
            );
        }
        writer.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    let mut extra = [0u8; 1];
    if file
        .read(&mut extra)
        .with_context(|| format!("failed to re-check {}", source_path.display()))?
        != 0
    {
        bail!(
            "SOURCE_BUNDLE_V1 file grew during packaging: {}",
            entry.path
        );
    }
    if sha256_finalize_hex(hasher) != entry.sha256 {
        bail!(
            "SOURCE_BUNDLE_V1 file changed during packaging: {}",
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
    executable: bool,
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
    } else if executable {
        TAR_EXECUTABLE_FILE_MODE
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

struct HashingWriter {
    inner: std::fs::File,
    hasher: Sha256,
    bytes_written: u64,
}

impl HashingWriter {
    fn new(inner: std::fs::File) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes_written: 0,
        }
    }

    fn finish(self) -> (String, u64) {
        (sha256_finalize_hex(self.hasher), self.bytes_written)
    }
}

impl Write for HashingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        self.bytes_written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
