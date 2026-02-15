use std::path::Path;

use anyhow::Context;
use flate2::Compression;
use flate2::write::GzEncoder;

/// Create a tar.gz archive of a directory, returning the bytes in memory.
pub fn create_tar_gz(dir: &Path) -> anyhow::Result<Vec<u8>> {
    let buf = Vec::new();
    let encoder = GzEncoder::new(buf, Compression::default());
    let mut archive = tar::Builder::new(encoder);

    archive
        .append_dir_all(".", dir)
        .with_context(|| format!("failed to archive {}", dir.display()))?;

    let encoder = archive.into_inner().context("failed to finalize tar")?;
    encoder.finish().context("failed to finalize gzip")
}
