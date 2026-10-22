use super::*;
use nrz_source_bundle::source_bundle_v2::{
    SOURCE_BUNDLE_V2_MEDIA_TYPE, SOURCE_BUNDLE_V2_SCHEMA_VERSION,
};
use nrz_source_bundle::{
    EDGE_BUILD_HANDOFF_V2_FILE, EDGE_BUILD_HANDOFF_V2_SCHEMA_VERSION,
    EDGE_BUILD_SOURCE_MANIFEST_V2_FILE, EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY, EdgeBuildBindingV2,
    EdgeBuildHandoffV2, EdgeBuildSourceManifestV2, EdgeBuildSourceTreeV2,
};

pub(super) fn binding_from_environment(deployment_id: Uuid) -> anyhow::Result<EdgeBuildBindingV2> {
    let read =
        |name: &str| std::env::var(name).with_context(|| format!("V2 handoff requires {name}"));
    let binding = EdgeBuildBindingV2 {
        deployment_id: deployment_id.to_string(),
        command_id: read("NRZ_EDGE_BUILD_COMMAND_ID")?,
        command_digest: read("NRZ_EDGE_BUILD_COMMAND_DIGEST")?,
        build_attempt_id: read("NRZ_EDGE_BUILD_BUILD_ATTEMPT_ID")?,
        attempt: read("NRZ_EDGE_BUILD_ATTEMPT")?
            .parse()
            .context("invalid V2 handoff attempt")?,
        source_environment_id: read("NRZ_EDGE_BUILD_SOURCE_ENVIRONMENT_ID")?,
        source_writer_generation: read("NRZ_EDGE_BUILD_SOURCE_WRITER_GENERATION")?
            .parse()
            .context("invalid V2 source writer generation")?,
    };
    binding.validate().map_err(anyhow::Error::msg)?;
    Ok(binding)
}

impl EdgeBuildHandoffOutput {
    pub(in crate::deploy) fn is_v2(&self) -> bool {
        self.mode == EDGE_BUILD_HANDOFF_MODE_V2
    }

    pub(in crate::deploy) fn publish_v2(
        &self,
        plan: &crate::deploy::plan::DeployPlan,
        deployment_id: &str,
        environment_id: &str,
        retry_count: u32,
    ) -> anyhow::Result<EdgeBuildHandoffV2> {
        let binding = self.validate_v2_binding(deployment_id, environment_id, retry_count)?;
        self.publish_v2_inventory(
            binding,
            &plan.artifact.root,
            &plan.artifact.runtime.manifest,
            &plan.files,
            &plan.artifact.runtime.scan,
        )
    }

    pub(in crate::deploy) fn validate_v2_binding(
        &self,
        deployment_id: &str,
        environment_id: &str,
        retry_count: u32,
    ) -> anyhow::Result<&EdgeBuildBindingV2> {
        ensure!(self.is_v2(), "Source V2 requires the V2 handoff contract");
        let binding = self
            .binding
            .as_ref()
            .context("V2 handoff command binding is missing")?;
        // Source registration uses retry_count; command bindings use a positive attempt.
        let publication_attempt = retry_count
            .checked_add(1)
            .context("deployment attempt overflow")?;
        ensure!(
            binding.deployment_id == deployment_id
                && binding.attempt == u64::from(publication_attempt),
            "V2 handoff binding disagrees with authenticated deployment attempt"
        );
        ensure!(
            binding.source_environment_id == environment_id,
            "V2 handoff source environment disagrees with authenticated execution context"
        );
        Ok(binding)
    }

    fn publish_v2_inventory(
        &self,
        binding: &EdgeBuildBindingV2,
        source_root: &nrz_runtime_artifact::ArtifactRoot,
        manifest: &crate::build::manifest::Manifest,
        files: &[crate::artifact::FileEntry],
        scan: &crate::artifact::RuntimeArtifactScan,
    ) -> anyhow::Result<EdgeBuildHandoffV2> {
        binding.validate().map_err(anyhow::Error::msg)?;
        fs::create_dir_all(&self.root)?;
        let root = fs::canonicalize(&self.root)?;
        let tree = root.join(EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY);
        let manifest_path = root.join(EDGE_BUILD_SOURCE_MANIFEST_V2_FILE);
        let descriptor_path = root.join(EDGE_BUILD_HANDOFF_V2_FILE);
        for path in [&tree, &manifest_path, &descriptor_path] {
            require_absent(path)?;
        }
        let staging = tempfile::Builder::new()
            .prefix(".source-v2-")
            .tempdir_in(&root)?;
        let staged_tree = staging.path().join(EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY);
        let (bytes, verified) = crate::artifact::source_bundle_v2::freeze_source_bundle(
            source_root,
            manifest,
            files,
            scan,
            crate::artifact::source_bundle_v1::RuntimeDependencyPackaging::TrustedMaterialization,
            &staged_tree,
        )?;
        let staged_manifest = staging.path().join(EDGE_BUILD_SOURCE_MANIFEST_V2_FILE);
        write_new_synced_file(&staged_manifest, &bytes)?;
        let handoff = EdgeBuildHandoffV2 {
            schema_version: EDGE_BUILD_HANDOFF_V2_SCHEMA_VERSION.into(),
            binding: binding.clone(),
            source_manifest: EdgeBuildSourceManifestV2 {
                path: EDGE_BUILD_SOURCE_MANIFEST_V2_FILE.into(),
                media_type: SOURCE_BUNDLE_V2_MEDIA_TYPE.into(),
                schema_version: SOURCE_BUNDLE_V2_SCHEMA_VERSION.into(),
                blake3: verified.manifest_blake3().into(),
                size_bytes: verified.manifest_size_bytes(),
                summary: verified.summary(),
            },
            source_tree: EdgeBuildSourceTreeV2 {
                path: EDGE_BUILD_SOURCE_TREE_V2_DIRECTORY.into(),
            },
        };
        handoff.validate().map_err(anyhow::Error::msg)?;
        let staged_descriptor = staging.path().join(EDGE_BUILD_HANDOFF_V2_FILE);
        let mut descriptor = serde_json::to_vec(&handoff)?;
        descriptor.push(b'\n');
        write_new_synced_file(&staged_descriptor, &descriptor)?;
        publish_new_file(&staged_tree, &tree).context("failed to publish Source V2 tree")?;
        if let Err(error) = publish_new_file(&staged_manifest, &manifest_path) {
            fs::remove_dir_all(&tree).context("failed to release owned Source V2 tree")?;
            sync_directory(&root)?;
            return Err(error).context("failed to publish Source V2 manifest");
        }
        if let Err(error) = publish_new_file(&staged_descriptor, &descriptor_path) {
            fs::remove_file(&manifest_path)
                .context("failed to release owned Source V2 manifest")?;
            fs::remove_dir_all(&tree).context("failed to release owned Source V2 tree")?;
            sync_directory(&root)?;
            return Err(error).context("failed to publish Source V2 descriptor");
        }
        sync_directory(&root)?;
        Ok(handoff)
    }
}

#[cfg(test)]
#[path = "edge_handoff_v2_tests.rs"]
mod tests;
