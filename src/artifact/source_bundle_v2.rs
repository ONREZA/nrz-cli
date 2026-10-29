use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, ensure};
use nrz_contract::source_bundle_v2 as wire;
use nrz_runtime_artifact::ArtifactRoot;
use nrz_source_bundle::source_bundle_v2::{
    SourceBundleV2ManifestBudget, VerifiedSourceBundleV2Manifest,
    encode_verified_source_bundle_v2_manifest,
};
use sha2::{Digest, Sha256};

use super::source_bundle_v1::{
    RuntimeDependencyPackaging, SourceBundleEntryKind, SourceFileVerification,
    SourceLogicalManifestFileRole, SourceRuntimeMetadata, prepare_source_bundle_inventory,
};
use super::{FileEntry, RuntimeArtifactScan};
use crate::build::manifest::Manifest;

/// Freeze only the planner's rooted inventory. This producer establishes no
/// trusted custody: the publication owner must independently verify its output.
pub(crate) fn freeze_source_bundle(
    root: &ArtifactRoot,
    manifest: &Manifest,
    files: &[FileEntry],
    scan: &RuntimeArtifactScan,
    dependency_packaging: RuntimeDependencyPackaging,
    destination: &Path,
    metadata: SourceRuntimeMetadata<'_>,
) -> anyhow::Result<(Vec<u8>, VerifiedSourceBundleV2Manifest)> {
    let (entries, mut logical) = prepare_source_bundle_inventory(
        root,
        manifest,
        files,
        scan,
        dependency_packaging,
        metadata,
        SourceFileVerification::DuringCopy,
    )?;
    for layer in &mut logical.layers {
        let application =
            nrz_source_bundle::layer_application_runtime(layer.runtime_config.as_ref())
                .map_err(anyhow::Error::msg)?;
        if layer.target == super::source_bundle_v1::SourceLogicalManifestLayerTarget::Compute
            && application.is_none()
        {
            let runtime = layer
                .runtime_config
                .as_ref()
                .context("Source V2 requires a frozen runtime configuration")?;
            let target = runtime
                .get("buildRuntimeVersion")
                .and_then(serde_json::Value::as_str)
                .context("Source V2 requires the retained or admitted runtime target")?;
            let launch =
                nrz_runtime_artifact::source_layer_launch_for_target(Some(runtime), Some(target))?;
            let family = match launch.profile {
                nrz_runtime_artifact::RuntimeProfile::Bun => "BUN",
                nrz_runtime_artifact::RuntimeProfile::Node22
                | nrz_runtime_artifact::RuntimeProfile::Node24
                | nrz_runtime_artifact::RuntimeProfile::Node26 => "NODE",
                _ => anyhow::bail!(
                    "Source V2 requires an explicit application runtime for this launcher"
                ),
            };
            layer
                .runtime_config
                .as_mut()
                .and_then(serde_json::Value::as_object_mut)
                .context("Source V2 runtime configuration must be an object")?
                .insert(
                    "applicationRuntime".into(),
                    serde_json::json!({"family":family,"args":launch.args}),
                );
        }
        if let Some(serde_json::Value::Object(runtime)) = &mut layer.runtime_config
            && runtime.contains_key("applicationRuntime")
        {
            // V2 carries the explicit launcher instead of duplicate V1 labels.
            runtime.remove("runtimeFamily");
            runtime.remove("isBinaryEntry");
        }
    }
    let mut output_files = Vec::with_capacity(entries.len());
    // Own only a newly created tree so any freeze error removes partial output.
    let mut builder = tempfile::Builder::new();
    builder
        .prefix(
            destination
                .file_name()
                .context("frozen Source V2 tree has no name")?,
        )
        .rand_bytes(0);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        builder.permissions(fs::Permissions::from_mode(0o700));
    }
    let frozen = builder
        .tempdir_in(
            destination
                .parent()
                .context("frozen Source V2 tree has no parent")?,
        )
        .context("failed to create frozen Source V2 tree")?;
    for (entry, logical_file) in entries.iter().zip(&logical.files) {
        ensure!(
            entry.path == logical_file.path,
            "Source V2 inventory order changed"
        );
        let output = destination.join(&entry.path);
        fs::create_dir_all(output.parent().context("source file has no parent")?)?;
        let role = match logical_file.role {
            SourceLogicalManifestFileRole::Static => wire::FileRole::Static,
            SourceLogicalManifestFileRole::Prerender => wire::FileRole::Prerender,
            SourceLogicalManifestFileRole::Compute => wire::FileRole::Compute,
            SourceLogicalManifestFileRole::Dependency => wire::FileRole::Dependency,
        };
        let path = wire::NonEmptyString(entry.path.clone());
        let layer_name = logical_file.layer_name.clone().map(wire::LayerName);
        let content_type = logical_file.content_type.clone();
        match &entry.kind {
            SourceBundleEntryKind::File { source_path, root } => {
                let digest = copy_file(
                    root,
                    source_path,
                    &output,
                    entry.size,
                    &entry.sha256,
                    entry.executable,
                )?;
                output_files.push(wire::FileEntry::File {
                    path,
                    role,
                    layer_name,
                    content_type,
                    blob_digest: wire::BlobDigest {
                        algorithm: wire::DigestAlgorithm::Blake3,
                        value: digest,
                    },
                    size: entry.size,
                    executable: entry.executable,
                });
            }
            SourceBundleEntryKind::Symlink { link_target, .. } => {
                output_files.push(wire::FileEntry::Symlink {
                    path,
                    role,
                    layer_name,
                    content_type,
                    link_target: wire::NonEmptyString(link_target.clone()),
                    size: 0,
                });
            }
        }
    }
    // Create links last so no output write can traverse a produced symlink.
    for entry in &entries {
        if let SourceBundleEntryKind::Symlink { link_target, .. } = &entry.kind {
            create_symlink(link_target, &destination.join(&entry.path))?;
        }
    }
    let wire = wire::OnrezaSourceBundleV2 {
        schema_version: wire::SchemaVersion::V2,
        capabilities: vec![wire::Capability::ComputeNetworkV1],
        files: output_files,
        layers: serde_json::from_value(serde_json::to_value(logical.layers)?)?,
        routes: serde_json::from_value(serde_json::to_value(logical.routes)?)?,
        entrypoints: logical
            .entrypoints
            .into_iter()
            .map(wire::NonEmptyString)
            .collect(),
    };
    let budget = SourceBundleV2ManifestBudget {
        max_file_count: entries.len() as u64,
        max_logical_bytes: 9_007_199_254_740_991,
        max_static_file_bytes: 9_007_199_254_740_991,
    };
    let (bytes, verified) =
        encode_verified_source_bundle_v2_manifest(serde_json::to_value(wire)?, budget)?;
    ensure!(
        verified
            .wire()
            .files
            .iter()
            .any(|file| matches!(file, wire::FileEntry::File { .. })),
        "SOURCE_BUNDLE_V2 output must contain a regular file"
    );
    let _ = frozen.keep();
    Ok((bytes, verified))
}

fn copy_file(
    root: &ArtifactRoot,
    source: &Path,
    destination: &Path,
    expected_size: u64,
    expected_sha256: &str,
    executable: bool,
) -> anyhow::Result<String> {
    let mut input = root
        .open_file(source)?
        .take(expected_size.saturating_add(1));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(if executable { 0o700 } else { 0o600 });
    }
    let mut output = options.open(destination)?;
    let mut sha256 = Sha256::new();
    let mut blake3 = blake3::Hasher::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 65_536];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        sha256.update(&buffer[..read]);
        blake3.update(&buffer[..read]);
        output.write_all(&buffer[..read])?;
    }
    ensure!(
        size == expected_size
            && crate::deploy::hash::sha256_finalize_hex(sha256) == expected_sha256,
        "Source V2 file changed during freeze: {}",
        source.display()
    );
    Ok(blake3.finalize().to_hex().to_string())
}

#[cfg(unix)]
fn create_symlink(target: &str, destination: &Path) -> anyhow::Result<()> {
    std::os::unix::fs::symlink(target, destination).map_err(Into::into)
}

#[cfg(not(unix))]
fn create_symlink(_target: &str, _destination: &Path) -> anyhow::Result<()> {
    anyhow::bail!("Source V2 symlink producer requires a Unix host")
}

#[cfg(test)]
#[path = "source_bundle_v2_tests.rs"]
mod tests;
