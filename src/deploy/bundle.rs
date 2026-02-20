use std::io::Write;
use std::path::Path;

use anyhow::{Context, bail};
use sha2::{Digest, Sha256};

const ZSTD_LEVEL: i32 = 3;

/// Create a tar.zst bundle from the output directory.
///
/// Returns `(compressed_bytes, sha256_hex)` where sha256_hex is lowercase hex, 64 chars.
/// Paths inside the archive are relative to `output_dir` (no leading `/`).
/// Entries are sorted by filename for deterministic output.
pub fn create_bundle(output_dir: &Path) -> anyhow::Result<(Vec<u8>, String)> {
    let buf = Vec::new();
    let encoder = zstd::Encoder::new(buf, ZSTD_LEVEL).context("failed to create zstd encoder")?;
    let mut tar_builder = tar::Builder::new(encoder);

    append_dir_recursive(&mut tar_builder, output_dir, output_dir)?;

    let encoder = tar_builder
        .into_inner()
        .context("failed to finalize tar archive")?;
    let compressed = encoder.finish().context("failed to finalize zstd stream")?;

    if compressed.is_empty() {
        bail!("tar.zst bundle is empty");
    }

    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let sha256_hex = format!("{:x}", hasher.finalize());

    Ok((compressed, sha256_hex))
}

fn append_dir_recursive<W: Write>(
    builder: &mut tar::Builder<W>,
    base: &Path,
    current: &Path,
) -> anyhow::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(current)
        .with_context(|| format!("failed to read directory {}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read entry in {}", current.display()))?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let ft = entry
            .file_type()
            .with_context(|| format!("failed to get file type for {}", path.display()))?;

        if ft.is_symlink() {
            continue;
        }

        let rel = path
            .strip_prefix(base)
            .context("failed to compute relative path")?;

        if ft.is_dir() {
            append_dir_recursive(builder, base, &path)?;
        } else if ft.is_file() {
            builder
                .append_path_with_name(&path, rel)
                .with_context(|| format!("failed to add {} to tar", rel.display()))?;
        }
    }

    Ok(())
}
