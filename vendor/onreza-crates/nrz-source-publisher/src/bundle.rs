// @generated vendored copy of platform crates/nrz-source-publisher/src/bundle.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures::StreamExt as _;
use nrz_source_bundle::{
    SourceBundleVerificationInput, SourceLogicalManifest, compute_source_artifact_id,
    verify_source_bundle_stream,
};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::SourcePublicationError;

pub(crate) const SOURCE_BUNDLE_FORMAT: &str = "tar.zst";
pub(crate) const CLI_PROTOCOL_VERSION: &str = "source-bundle-v1-embedded-manifest";
const MULTIPART_THRESHOLD_BYTES: u64 = 256 * 1024 * 1024;
const MULTIPART_CHUNK_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SourceBundleInput {
    pub path: PathBuf,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    pub logical_manifest_sha256: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MultipartPart {
    pub part_number: u32,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MultipartDescriptor {
    pub part_size_bytes: u64,
    pub parts: Vec<MultipartPart>,
}

#[derive(Debug)]
pub struct PreparedSourceBundle {
    input: SourceBundleInput,
    source_artifact_id: String,
    manifest: SourceLogicalManifest,
    max_static_file_size_bytes: u64,
    multipart: Option<MultipartDescriptor>,
}

impl PreparedSourceBundle {
    pub async fn verify(
        workspace_id: Uuid,
        input: SourceBundleInput,
    ) -> Result<Self, SourcePublicationError> {
        let actual_size = tokio::fs::metadata(&input.path)
            .await
            .map_err(|source| SourcePublicationError::Io {
                operation: "stat the source bundle",
                source,
            })?
            .len();
        if actual_size != input.source_size_bytes {
            return Err(SourcePublicationError::InvalidSourceBundle(format!(
                "source size mismatch: expected {}, got {actual_size}",
                input.source_size_bytes
            )));
        }
        let source_artifact_id = compute_source_artifact_id(
            &workspace_id.to_string(),
            &input.logical_manifest_sha256,
            &input.source_sha256,
            None,
        );
        let source = open_source_bundle(&input.path).await?;
        let verification = verify_source_bundle_stream(
            SourceBundleVerificationInput {
                owner_workspace_id: workspace_id.to_string(),
                source_artifact_id: source_artifact_id.clone(),
                source_sha256: input.source_sha256.clone(),
                logical_manifest_sha256: input.logical_manifest_sha256.clone(),
            },
            ReaderStream::new(source).map(|item| item.map_err(|error| error.to_string())),
        )
        .await
        .map_err(|error| SourcePublicationError::InvalidSourceBundle(error.to_string()))?;
        let manifest: SourceLogicalManifest = serde_json::from_value(verification.logical_manifest)
            .map_err(|error| SourcePublicationError::InvalidSourceBundle(error.to_string()))?;
        let max_static_file_size_bytes = manifest
            .files
            .iter()
            .filter(|file| file.role == "static")
            .map(|file| file.size)
            .max()
            .unwrap_or(0);
        let multipart = if source_uses_multipart(input.source_size_bytes) {
            Some(describe_multipart(&input.path, input.source_size_bytes).await?)
        } else {
            None
        };
        Ok(Self {
            input,
            source_artifact_id,
            manifest,
            max_static_file_size_bytes,
            multipart,
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.input.path
    }

    #[must_use]
    pub fn source_sha256(&self) -> &str {
        &self.input.source_sha256
    }

    #[must_use]
    pub const fn source_size_bytes(&self) -> u64 {
        self.input.source_size_bytes
    }

    #[must_use]
    pub fn logical_manifest_sha256(&self) -> &str {
        &self.input.logical_manifest_sha256
    }

    #[must_use]
    pub fn source_artifact_id(&self) -> &str {
        &self.source_artifact_id
    }

    #[must_use]
    pub const fn manifest(&self) -> &SourceLogicalManifest {
        &self.manifest
    }

    #[must_use]
    pub const fn max_static_file_size_bytes(&self) -> u64 {
        self.max_static_file_size_bytes
    }

    #[must_use]
    pub(crate) const fn multipart(&self) -> Option<&MultipartDescriptor> {
        self.multipart.as_ref()
    }

    pub(crate) async fn read_all(&self) -> Result<Bytes, SourcePublicationError> {
        tokio::fs::read(&self.input.path)
            .await
            .map(Bytes::from)
            .map_err(|source| SourcePublicationError::Io {
                operation: "read the source bundle",
                source,
            })
    }

    pub(crate) async fn read_chunk(
        &self,
        offset: u64,
        size: u64,
    ) -> Result<Bytes, SourcePublicationError> {
        read_file_slice(&self.input.path, offset, size).await
    }
}

#[must_use]
pub const fn source_uses_multipart(source_size_bytes: u64) -> bool {
    source_size_bytes >= MULTIPART_THRESHOLD_BYTES
}

async fn open_source_bundle(path: &Path) -> Result<tokio::fs::File, SourcePublicationError> {
    let mut options = tokio::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(path)
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "open the source bundle",
            source,
        })?;
    let metadata = file
        .metadata()
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "inspect the source bundle",
            source,
        })?;
    if !metadata.is_file() {
        return Err(SourcePublicationError::InvalidSourceBundle(
            "source bundle is not a regular file".to_string(),
        ));
    }
    Ok(file)
}

async fn describe_multipart(
    path: &Path,
    source_size_bytes: u64,
) -> Result<MultipartDescriptor, SourcePublicationError> {
    let mut parts = Vec::new();
    let mut remaining = source_size_bytes;
    let mut offset = 0_u64;
    let mut part_number = 1_u32;
    while remaining > 0 {
        let size = remaining.min(MULTIPART_CHUNK_BYTES);
        let bytes = read_file_slice(path, offset, size).await?;
        parts.push(MultipartPart {
            part_number,
            size_bytes: size,
            sha256: hex::encode(Sha256::digest(&bytes)),
        });
        remaining -= size;
        offset += size;
        part_number = part_number.saturating_add(1);
    }
    Ok(MultipartDescriptor {
        part_size_bytes: MULTIPART_CHUNK_BYTES,
        parts,
    })
}

async fn read_file_slice(
    path: &Path,
    offset: u64,
    size: u64,
) -> Result<Bytes, SourcePublicationError> {
    let size = usize::try_from(size).map_err(|_| {
        SourcePublicationError::InvalidSourceBundle("multipart chunk exceeds usize".to_string())
    })?;
    let mut file =
        tokio::fs::File::open(path)
            .await
            .map_err(|source| SourcePublicationError::Io {
                operation: "open the source bundle chunk",
                source,
            })?;
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "seek the source bundle chunk",
            source,
        })?;
    let mut bytes = vec![0_u8; size];
    file.read_exact(&mut bytes)
        .await
        .map_err(|source| SourcePublicationError::Io {
            operation: "read the source bundle chunk",
            source,
        })?;
    Ok(Bytes::from(bytes))
}
