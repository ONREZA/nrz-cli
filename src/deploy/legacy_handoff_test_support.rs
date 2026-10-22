use super::*;
use crate::artifact::source_bundle_v1::SourceBundlePlan;
use crate::deploy::hash::sha256_finalize_hex;
use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V1_FILE, EDGE_BUILD_HANDOFF_V1_SCHEMA_VERSION,
    EDGE_BUILD_SOURCE_BUNDLE_V1_FILE, EdgeBuildHandoffV1, EdgeBuildSourceBundleV1,
    SOURCE_BUNDLE_V1_MEDIA_TYPE, SOURCE_BUNDLE_V1_SCHEMA_VERSION,
};
use sha2::{Digest, Sha256};
use std::io::Read;
const COPY_BUFFER_BYTES: usize = 65_536;

impl EdgeBuildHandoffOutput {
    pub(in crate::deploy) fn publish(
        &self,
        source_bundle: &SourceBundlePlan,
    ) -> anyhow::Result<EdgeBuildHandoffV1> {
        let manifest: nrz_source_bundle::SourceLogicalManifest =
            serde_json::from_value(serde_json::to_value(&source_bundle.logical_manifest)?)
                .context("failed to decode Edge build source graph")?;
        nrz_runtime_artifact::validate_source_bundle_application_graph(
            &source_bundle.logical_manifest_sha256,
            &source_bundle.source_sha256,
            source_bundle.source_size_bytes,
            &manifest,
        )
        .map_err(|error| {
            crate::output::coded_error(
                "INVALID_RUNTIME_ARTIFACT_GRAPH",
                format!("Edge build source graph is invalid: {error}"),
            )
        })?;
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
        let _ = fs::remove_file(&archive_temp);
        let _ = fs::remove_file(&descriptor_temp);
        result
    }

    pub(in crate::deploy) fn publish_inner(
        &self,
        source_bundle: &SourceBundlePlan,
        root: &Path,
        archive_temp: &Path,
        final_archive: &Path,
        descriptor_temp: &Path,
        final_descriptor: &Path,
    ) -> anyhow::Result<EdgeBuildHandoffV1> {
        let (copied_size, copied_sha256) = copy_and_hash(
            source_bundle.source_path(),
            archive_temp,
            source_bundle.source_size_bytes,
        )?;
        ensure!(
            copied_size == source_bundle.source_size_bytes,
            "Edge build handoff source bundle size changed during publication"
        );
        ensure!(
            copied_sha256 == source_bundle.source_sha256,
            "Edge build handoff source bundle digest changed during publication"
        );
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
        publish_new_file(archive_temp, final_archive)
            .context("failed to publish Edge build source bundle")?;
        if let Err(error) = publish_new_file(descriptor_temp, final_descriptor) {
            fs::remove_file(final_archive).with_context(|| {
                format!(
                    "failed to release owned archive after descriptor publication failed: {error}"
                )
            })?;
            sync_directory(root).context("failed to sync rolled-back Edge build handoff")?;
            return Err(error).context("failed to publish Edge build handoff descriptor");
        }
        sync_directory(root)?;
        Ok(handoff)
    }
}

fn copy_and_hash(
    source: &Path,
    destination: &Path,
    expected_size: u64,
) -> anyhow::Result<(u64, String)> {
    let mut input = File::open(source)
        .with_context(|| format!("failed to open source bundle {}", source.display()))?
        .take(expected_size.saturating_add(1));
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
