use std::io::Write;
use std::path::{Component, Path, PathBuf};

use anyhow::Context;
use sha2::{Digest, Sha256};

use crate::output;

const ZSTD_LEVEL: i32 = 3;

#[derive(Debug)]
pub struct BundleStats {
    pub bytes: Vec<u8>,
    pub sha256_hex: String,
    pub files: usize,
    pub file_entries: Vec<BundleFileEntry>,
    pub symlinks_preserved: usize,
    pub symlinks_skipped: usize,
}

#[derive(Debug, Clone)]
pub struct BundleFileEntry {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Default)]
struct Counters {
    files: usize,
    file_entries: Vec<BundleFileEntry>,
    symlinks_preserved: usize,
    symlinks_skipped: usize,
}

/// Create a tar.zst bundle from the output directory.
///
/// `sha256_hex` is lowercase hex, 64 chars. Paths inside the archive are
/// relative to `output_dir` (no leading `/`). Entries are sorted by filename
/// for deterministic output.
///
/// Relative symlinks that resolve inside `output_dir` are preserved as symlinks
/// in the tar archive. This is required for pnpm-based projects, where
/// `.next/standalone/node_modules/{next,react,...}` are symlinks into `.pnpm/`.
/// Absolute symlinks and broken symlinks are treated as errors, since both
/// produce a bundle that is guaranteed to fail on the compute node. Symlinks
/// that escape the bundle root are skipped with a warning.
pub fn create_bundle(output_dir: &Path) -> anyhow::Result<BundleStats> {
    create_bundle_excluding_dirs(output_dir, &[])
}

/// Create a tar.zst bundle from the output directory, skipping selected
/// layer directories that are shipped through another upload target.
pub fn create_bundle_excluding_dirs(
    output_dir: &Path,
    excluded_dirs: &[String],
) -> anyhow::Result<BundleStats> {
    let canonical_base = std::fs::canonicalize(output_dir)
        .with_context(|| format!("failed to canonicalize {}", output_dir.display()))?;
    let excluded_dirs = normalize_excluded_dirs(excluded_dirs);

    let buf = Vec::new();
    let encoder = zstd::Encoder::new(buf, ZSTD_LEVEL).context("failed to create zstd encoder")?;
    let mut tar_builder = tar::Builder::new(encoder);

    let mut counters = Counters::default();
    append_dir_recursive(
        &mut tar_builder,
        output_dir,
        output_dir,
        &canonical_base,
        &excluded_dirs,
        &mut counters,
    )?;

    let encoder = tar_builder
        .into_inner()
        .context("failed to finalize tar archive")?;
    let compressed = encoder.finish().context("failed to finalize zstd stream")?;

    if compressed.is_empty() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            "tar.zst bundle is empty",
        ));
    }

    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let sha256_hex = format!("{:x}", hasher.finalize());

    Ok(BundleStats {
        bytes: compressed,
        sha256_hex,
        files: counters.files,
        file_entries: counters.file_entries,
        symlinks_preserved: counters.symlinks_preserved,
        symlinks_skipped: counters.symlinks_skipped,
    })
}

fn normalize_excluded_dirs(dirs: &[String]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = dirs
        .iter()
        .filter_map(|dir| {
            let trimmed = dir.trim_matches('/');
            if trimmed.is_empty() || trimmed == "." {
                return None;
            }
            let path = Path::new(trimmed);
            if path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            }) {
                return None;
            }
            Some(path.to_path_buf())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn is_excluded(rel: &Path, excluded_dirs: &[PathBuf]) -> bool {
    excluded_dirs
        .iter()
        .any(|dir| rel == dir || rel.starts_with(dir))
}

fn append_dir_recursive<W: Write>(
    builder: &mut tar::Builder<W>,
    base: &Path,
    current: &Path,
    canonical_base: &Path,
    excluded_dirs: &[PathBuf],
    counters: &mut Counters,
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

        let rel = path
            .strip_prefix(base)
            .context("failed to compute relative path")?;
        if is_excluded(rel, excluded_dirs) {
            continue;
        }

        if ft.is_symlink() {
            append_symlink(builder, &path, rel, canonical_base, counters)?;
        } else if ft.is_dir() {
            // Emit a directory entry before recursing so empty dirs survive extraction
            // (runtime code that expects `logs/`, `tmp/`, etc. to exist would otherwise
            // get an ENOENT on first access — a silent skip class we already fixed for symlinks).
            append_dir_entry(builder, rel)?;
            append_dir_recursive(
                builder,
                base,
                &path,
                canonical_base,
                excluded_dirs,
                counters,
            )?;
        } else if ft.is_file() {
            let size = entry
                .metadata()
                .with_context(|| format!("failed to stat {}", path.display()))?
                .len();
            builder
                .append_path_with_name(&path, rel)
                .with_context(|| format!("failed to add {} to tar", rel.display()))?;
            counters.file_entries.push(BundleFileEntry {
                path: rel.to_string_lossy().replace('\\', "/"),
                size,
            });
            counters.files += 1;
        }
    }

    Ok(())
}

/// Append a symlink to the tar archive, preserving its relative target.
///
/// Stores the symlink entry (not the target's contents) so bundle size stays
/// small for pnpm layouts and Node's module resolver sees the expected
/// `node_modules/<pkg>` symlink structure at runtime.
///
/// Invariants: a symlink is only emitted when its target is relative AND the
/// resolved path stays inside the bundle root. Absolute targets and broken
/// symlinks are errors (both produce bundles that fail on the compute node);
/// symlinks that escape the bundle are skipped with a warning.
fn append_symlink<W: Write>(
    builder: &mut tar::Builder<W>,
    path: &Path,
    rel: &Path,
    canonical_base: &Path,
    counters: &mut Counters,
) -> anyhow::Result<()> {
    let target: PathBuf = std::fs::read_link(path)
        .with_context(|| format!("failed to read symlink {}", path.display()))?;

    if target.is_absolute() {
        return Err(output::coded_error(
            "INVALID_BUILD_OUTPUT",
            format!(
                "symlink {} has absolute target {} — absolute symlinks cannot survive extraction on the compute node. Check your build output for an upstream bug.",
                path.display(),
                target.display()
            ),
        ));
    }

    // canonicalize() follows the link and collapses `..`, so comparing against
    // the canonical base guards against escapes even when the target contains `../`.
    match std::fs::canonicalize(path) {
        Ok(canonical) if canonical.starts_with(canonical_base) => {}
        Ok(canonical) => {
            tracing::warn!(
                path = %path.display(),
                target = %target.display(),
                resolved = %canonical.display(),
                "skipping symlink that escapes bundle root"
            );
            counters.symlinks_skipped += 1;
            return Ok(());
        }
        Err(e) => {
            return Err(output::coded_error(
                "INVALID_BUILD_OUTPUT",
                format!(
                    "broken symlink {} -> {} ({}). If this is inside node_modules, try reinstalling dependencies; otherwise, check where the symlink target should live.",
                    path.display(),
                    target.display(),
                    e
                ),
            ));
        }
    }

    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_mtime(0);

    builder
        .append_link(&mut header, rel, &target)
        .with_context(|| format!("failed to add symlink {} to tar", rel.display()))?;
    counters.symlinks_preserved += 1;

    Ok(())
}

/// Emit a deterministic directory entry so empty dirs survive extraction.
///
/// Uses fixed mode 0o755 and mtime 0 to keep bundle sha256 reproducible;
/// without this, an empty `logs/` or `data/` dir in the build output would
/// silently vanish from the archive.
fn append_dir_entry<W: Write>(builder: &mut tar::Builder<W>, rel: &Path) -> anyhow::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_mtime(0);

    builder
        .append_data(&mut header, rel, std::io::empty())
        .with_context(|| format!("failed to add directory {} to tar", rel.display()))?;

    Ok(())
}
