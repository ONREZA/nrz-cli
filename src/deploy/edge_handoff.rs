use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V1_FILE, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION,
    EDGE_BUILD_SOURCE_BUNDLE_V1_FILE, EdgeBuildHandoffV1, EdgeBuildSourceBundleV1,
    SOURCE_BUNDLE_V1_MEDIA_TYPE, SOURCE_BUNDLE_V1_SCHEMA_VERSION,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::artifact::source_bundle_v1::SourceBundlePlan;
use crate::deploy::hash::sha256_finalize_hex;

pub(super) const EDGE_BUILD_HANDOFF_MODE_ENV: &str = "NRZ_EDGE_BUILD_HANDOFF";
pub(super) const EDGE_BUILD_HANDOFF_MODE_V1: &str = "V1";
const OUTPUT_DIRECTORY_ENV: &str = "ONREZA_OUTPUT_DIR";
const PLATFORM_RUNNER_ENV: &str = "NRZ_RUNNER";
const PLATFORM_RUNNER_VALUE: &str = "PLATFORM";
const COPY_BUFFER_BYTES: usize = 64 * 1024;

pub(super) struct EdgeBuildHandoffOutput {
    root: PathBuf,
}

impl EdgeBuildHandoffOutput {
    pub(super) fn from_process_environment(
        resume_deployment_id: Option<Uuid>,
    ) -> anyhow::Result<Option<Self>> {
        let mode = std::env::var(EDGE_BUILD_HANDOFF_MODE_ENV).ok();
        let output = std::env::var_os(OUTPUT_DIRECTORY_ENV).map(PathBuf::from);
        let platform_runner =
            std::env::var(PLATFORM_RUNNER_ENV).is_ok_and(|value| value == PLATFORM_RUNNER_VALUE);
        Self::from_values(
            mode.as_deref(),
            output.as_deref(),
            platform_runner,
            resume_deployment_id,
        )
    }

    pub(super) fn publish(
        &self,
        source_bundle: &SourceBundlePlan,
    ) -> anyhow::Result<EdgeBuildHandoffV1> {
        fs::create_dir_all(&self.root).with_context(|| {
            format!(
                "failed to create Edge build handoff directory {}",
                self.root.display()
            )
        })?;
        let root = fs::canonicalize(&self.root).with_context(|| {
            format!(
                "failed to canonicalize Edge build handoff directory {}",
                self.root.display()
            )
        })?;
        ensure!(
            root.is_dir(),
            "Edge build handoff output must be a directory"
        );

        let final_archive = root.join(EDGE_BUILD_SOURCE_BUNDLE_V1_FILE);
        let final_descriptor = root.join(EDGE_BUILD_HANDOFF_V1_FILE);
        require_absent(&final_archive)?;
        require_absent(&final_descriptor)?;

        let archive_temp = root.join(format!(".source-bundle-{}.tmp", Uuid::now_v7()));
        let descriptor_temp = root.join(format!(".edge-build-handoff-{}.tmp", Uuid::now_v7()));
        let result = self.publish_inner(
            source_bundle,
            &root,
            &archive_temp,
            &final_archive,
            &descriptor_temp,
            &final_descriptor,
        );
        if result.is_err() {
            let _ = fs::remove_file(&archive_temp);
            let _ = fs::remove_file(&descriptor_temp);
        }
        result
    }

    pub(super) fn from_values(
        mode: Option<&str>,
        output: Option<&Path>,
        platform_runner: bool,
        resume_deployment_id: Option<Uuid>,
    ) -> anyhow::Result<Option<Self>> {
        let Some(mode) = mode else {
            return Ok(None);
        };
        if mode != EDGE_BUILD_HANDOFF_MODE_V1 {
            bail!("unsupported {EDGE_BUILD_HANDOFF_MODE_ENV} mode");
        }
        ensure!(
            platform_runner,
            "{EDGE_BUILD_HANDOFF_MODE_ENV} requires {PLATFORM_RUNNER_ENV}={PLATFORM_RUNNER_VALUE}"
        );
        ensure!(
            resume_deployment_id.is_some(),
            "{EDGE_BUILD_HANDOFF_MODE_ENV} requires --resume-deployment"
        );
        let output = output.context("Edge build handoff requires ONREZA_OUTPUT_DIR")?;
        ensure!(
            output.is_absolute(),
            "Edge build handoff output must be an absolute path"
        );
        Ok(Some(Self {
            root: output.to_path_buf(),
        }))
    }

    fn publish_inner(
        &self,
        source_bundle: &SourceBundlePlan,
        root: &Path,
        archive_temp: &Path,
        final_archive: &Path,
        descriptor_temp: &Path,
        final_descriptor: &Path,
    ) -> anyhow::Result<EdgeBuildHandoffV1> {
        let (copied_size, copied_sha256) =
            copy_and_hash(source_bundle.source_path(), archive_temp)?;
        ensure!(
            copied_size == source_bundle.source_size_bytes,
            "Edge build handoff source bundle size changed during publication"
        );
        ensure!(
            copied_sha256 == source_bundle.source_sha256,
            "Edge build handoff source bundle digest changed during publication"
        );
        fs::rename(archive_temp, final_archive)
            .context("failed to publish Edge build source bundle")?;
        sync_directory(root)?;

        let handoff = EdgeBuildHandoffV1 {
            schema_version: EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION.to_string(),
            source_bundle: EdgeBuildSourceBundleV1 {
                path: EDGE_BUILD_SOURCE_BUNDLE_V1_FILE.to_string(),
                media_type: SOURCE_BUNDLE_V1_MEDIA_TYPE.to_string(),
                schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
                sha256: copied_sha256,
                size_bytes: copied_size,
                logical_manifest_sha256: source_bundle.logical_manifest_sha256.clone(),
            },
        };
        handoff.validate().map_err(anyhow::Error::msg)?;
        let mut descriptor = serde_json::to_vec(&handoff)
            .context("failed to serialize Edge build handoff descriptor")?;
        descriptor.push(b'\n');
        write_new_synced_file(descriptor_temp, &descriptor)?;
        fs::rename(descriptor_temp, final_descriptor)
            .context("failed to publish Edge build handoff descriptor")?;
        sync_directory(root)?;
        Ok(handoff)
    }
}

fn copy_and_hash(source: &Path, destination: &Path) -> anyhow::Result<(u64, String)> {
    let mut input = File::open(source)
        .with_context(|| format!("failed to open source bundle {}", source.display()))?;
    let mut output = new_file(destination)?;
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let read = input
            .read(&mut buffer)
            .context("failed to read Edge build source bundle")?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .context("failed to write Edge build source bundle")?;
        hasher.update(&buffer[..read]);
        size = size
            .checked_add(read as u64)
            .context("Edge build source bundle size overflow")?;
    }
    output
        .sync_all()
        .context("failed to sync Edge build source bundle")?;
    Ok((size, sha256_finalize_hex(hasher)))
}

fn write_new_synced_file(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let mut file = new_file(path)?;
    file.write_all(contents)
        .context("failed to write Edge build handoff descriptor")?;
    file.sync_all()
        .context("failed to sync Edge build handoff descriptor")
}

fn new_file(path: &Path) -> anyhow::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))
}

fn require_absent(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => bail!(
            "Edge build handoff output already exists: {}",
            path.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect handoff output {}", path.display())),
    }
}

fn sync_directory(path: &Path) -> anyhow::Result<()> {
    File::open(path)
        .with_context(|| format!("failed to open handoff directory {}", path.display()))?
        .sync_all()
        .with_context(|| format!("failed to sync handoff directory {}", path.display()))
}
