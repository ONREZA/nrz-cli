//! Private application package construction, without publication or READY authority.
//!
//! The caller admits aggregate disk capacity, worker CPU/memory/time, and concurrency
//! before calling. The supplied staging parent and pinned tools must stay in trusted
//! custody. Executors must terminate all workers and close writable handles before
//! returning. Local limits count logical staging bytes/nodes and verification work;
//! filesystem overhead, the already prepared source, allocator arenas and worker
//! address spaces belong to the caller's reservation. Cleanup is best effort.

use std::{
    fs::{self, File, OpenOptions, Permissions},
    io::{self, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::Arc,
};

use nrz_contract::{
    application_package_v1::{BlobReference, PackageDescriptor, Role, SchemaVersion},
    source_bundle_v2::{BlobDigest, DigestAlgorithm, FileEntry, FileRole},
};
use nrz_runtime_artifact::application_package_v1::{
    APPLICATION_PACKAGE_MEDIA_TYPE, ApplicationPackagePolicy, VerifiedApplicationPackageDescriptor,
    application_package_layout, verify_application_package_descriptor,
};
use nrz_source_bundle::source_bundle_v2::{
    VerifiedSourceBundleV2Manifest, admission::AdmittedSourceBundleV2Manifest,
};
use tempfile::TempDir;

use crate::{
    application_erofs_metadata::{MetadataLimits, verify_erofs_metadata},
    application_erofs_readback::verify_extracted_tree,
    application_erofs_tools::{
        ApplicationErofsToolExecutor, ApplicationErofsTools, ApplicationToolLimits, ToolError,
    },
    application_verity::append_application_verity,
    source_v2_preparation::{
        PreparationError, PreparedSourceV2, common_prefix_bytes, create_directory, entry_path,
    },
};

/// Candidate profile; production enablement needs host/toolchain qualification.
/// Fixed by this version: zstd level 3, 4 KiB blocks/clusters, one worker,
/// epoch timestamps, uid/gid zero, no xattrs/hardlinks, files/dirs 0775,
/// symlinks 0777, and UUID/salt derived from the role tree and exact tools.
pub use nrz_contract::application_package_v1::APPLICATION_EROFS_PROFILE_V1;
const RECIPE_CONTEXT: &str = "ONREZA application package role recipe v1";
const WORKING_HEADROOM: u64 = 256 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct ApplicationMaterializationLimits {
    /// Upper bound for each complete EROFS + verity package.
    pub max_package_bytes: u64,
    /// Additional logical regular-file bytes, excluding the prepared snapshot.
    pub max_staging_file_bytes: u64,
    /// Additional directories, files and symlinks, including private containers.
    pub max_staging_nodes: u64,
    /// Per-path and raw symlink target bound.
    pub max_path_bytes: usize,
    /// Conservative working allocation budget, excluding the admitted manifest.
    pub max_verification_memory_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplicationMaterializationError {
    #[error("application materialization exceeds {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Preparation(#[from] PreparationError),
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error("application materialization I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("invalid application materialization: {0}")]
    Invalid(String),
}

struct Component {
    descriptor: VerifiedApplicationPackageDescriptor,
    path: PathBuf,
}

/// Owns immutable package contents and the original admitted inventory together.
/// This proves local package content; native compatibility, durable publication,
/// authentication and mounted-generation acceptance remain separate obligations.
pub struct MaterializedApplicationPackages {
    components: Vec<Component>,
    _staging: TempDir,
    source: AdmittedSourceBundleV2Manifest,
    // Release package paths before the executor's physical storage reservation.
    _execution_owner: Arc<dyn ApplicationErofsToolExecutor>,
}

impl MaterializedApplicationPackages {
    pub fn manifest(&self) -> &VerifiedSourceBundleV2Manifest {
        self.source.verified()
    }

    pub fn descriptors(
        &self,
    ) -> impl ExactSizeIterator<Item = &VerifiedApplicationPackageDescriptor> {
        self.components
            .iter()
            .map(|component| &component.descriptor)
    }

    /// Readers cannot detach package bytes from their custody/reservation owner.
    /// ```compile_fail,E0505
    /// use nrz_dependency_materializer::application_package::MaterializedApplicationPackages;
    /// use nrz_contract::application_package_v1::Role;
    /// use std::io::Read;
    /// fn detach(packages: MaterializedApplicationPackages) {
    ///     let mut reader = packages.open_package(Role::Static).unwrap();
    ///     drop(packages);
    ///     reader.read_exact(&mut [0]).unwrap();
    /// }
    /// ```
    pub fn open_package(&self, role: Role) -> io::Result<ApplicationPackageReader<'_>> {
        let component = self
            .components
            .iter()
            .find(|component| component.descriptor.wire().role == role)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "empty application role"))?;
        Ok(ApplicationPackageReader {
            file: File::open(&component.path)?,
            owner: PhantomData,
        })
    }
}

pub struct ApplicationPackageReader<'a> {
    file: File,
    owner: PhantomData<&'a MaterializedApplicationPackages>,
}

impl Read for ApplicationPackageReader<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.read(bytes)
    }
}

impl Seek for ApplicationPackageReader<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}

pub fn materialize_application_packages(
    source: PreparedSourceV2,
    staging_parent: &Path,
    tools: &ApplicationErofsTools,
    limits: ApplicationMaterializationLimits,
) -> Result<MaterializedApplicationPackages, ApplicationMaterializationError> {
    let usage = preflight(source.manifest().wire().files.as_slice(), limits)?;
    let staging = tempfile::Builder::new()
        .prefix(".nrz-application-")
        .tempdir_in(staging_parent)?;
    fs::set_permissions(staging.path(), Permissions::from_mode(0o700))?;
    let policy = ApplicationPackagePolicy {
        canonicalization_profile: APPLICATION_EROFS_PROFILE_V1.into(),
        generator_identity: tools.generator_identity().into(),
    };
    let mut components = Vec::with_capacity(2);
    for (index, role) in [Role::Static, Role::Compute].into_iter().enumerate() {
        if usage[index].entries == 0 {
            continue;
        }
        let mut expected = Vec::new();
        expected
            .try_reserve_exact(usage[index].entries)
            .map_err(|_| ApplicationMaterializationError::Limit("inventory allocation"))?;
        expected.extend(
            source
                .manifest()
                .wire()
                .files
                .iter()
                .filter(|entry| entry_role(entry) == Some(role)),
        );
        let memory = limits
            .max_verification_memory_bytes
            .checked_sub(working_headroom(limits)?)
            .and_then(|bytes| {
                bytes.checked_sub((expected.capacity() * size_of::<&FileEntry>()) as u64)
            })
            .ok_or(ApplicationMaterializationError::Limit(
                "verification memory limit",
            ))?;
        let salt = recipe(&expected, role, tools.generator_identity());
        let mut uuid: [u8; 16] = salt[..16].try_into().expect("fixed salt length");
        uuid[6] = (uuid[6] & 0x0f) | 0x80;
        uuid[8] = (uuid[8] & 0x3f) | 0x80;
        let scratch = tempfile::Builder::new()
            .prefix("work-")
            .tempdir_in(staging.path())?;
        let tree = scratch.path().join("tree");
        copy_role_tree(&source, &expected, &tree)?;
        let path = staging.path().join(format!("{index}.package"));
        let mut image = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        let original = image.metadata()?;
        let tool_limits = ApplicationToolLimits {
            max_output_bytes: limits.max_package_bytes,
            max_output_nodes: 1,
        };
        if let Err(error) = tools.generate(&tree, &path, uuid, tool_limits) {
            drop(expected);
            drop(image);
            error.retain_until_cleanup((scratch, staging, source));
            return Err(error.into());
        }
        let actual = fs::symlink_metadata(&path)?;
        if !actual.is_file()
            || actual.nlink() != 1
            || (actual.dev(), actual.ino()) != (original.dev(), original.ino())
        {
            return invalid("generator replaced the private image inode");
        }
        let size = image.metadata()?.len();
        let layout = application_package_layout(size).map_err(invalid_error)?;
        if layout.package_size > limits.max_package_bytes {
            return Err(ApplicationMaterializationError::Limit("package byte limit"));
        }
        let erofs_size = verify_erofs_metadata(
            &mut image,
            &expected,
            uuid,
            MetadataLimits {
                max_nodes: usage[index].nodes,
                max_path_bytes: limits.max_path_bytes,
                max_memory_bytes: memory,
            },
        )
        .map_err(invalid_error)?;
        let extracted = scratch.path().join("extracted");
        fs::create_dir(&extracted)?;
        if let Err(error) = tools.extract(
            &path,
            &extracted,
            ApplicationToolLimits {
                max_output_bytes: usage[index].file_bytes,
                max_output_nodes: usage[index].nodes,
            },
        ) {
            drop(expected);
            drop(image);
            error.retain_until_cleanup((scratch, staging, source));
            return Err(error.into());
        }
        verify_extracted_tree(
            &extracted,
            &expected,
            usage[index].nodes,
            limits.max_path_bytes,
            memory,
        )
        .map_err(invalid_error)?;
        // No writable worker handles survive execute; content proof and verity share this inode.
        scratch.close()?;
        let computed =
            append_application_verity(&mut image, erofs_size, salt).map_err(invalid_error)?;
        image.sync_all()?;
        drop(image);
        let descriptor = verify_application_package_descriptor(
            PackageDescriptor {
                schema_version: SchemaVersion::ApplicationPackageV1,
                role,
                blob: BlobReference {
                    media_type: APPLICATION_PACKAGE_MEDIA_TYPE.into(),
                    digest: BlobDigest {
                        algorithm: DigestAlgorithm::Blake3,
                        value: computed.blob_digest,
                    },
                    size: computed.blob_size,
                },
                canonicalization_profile: policy.canonicalization_profile.clone(),
                generator_identity: policy.generator_identity.clone(),
                erofs_size,
                verity: computed.verity,
            },
            &policy,
        )
        .map_err(invalid_error)?;
        components.push(Component { descriptor, path });
    }
    Ok(MaterializedApplicationPackages {
        components,
        _staging: staging,
        source: source.release_snapshot()?,
        _execution_owner: tools.execution_owner(),
    })
}

#[derive(Clone, Copy, Default)]
struct RoleUsage {
    entries: usize,
    nodes: u64,
    file_bytes: u64,
}

fn preflight(
    files: &[FileEntry],
    limits: ApplicationMaterializationLimits,
) -> Result<[RoleUsage; 2], ApplicationMaterializationError> {
    let mut usage = [RoleUsage::default(); 2];
    let mut previous = [""; 2];
    for entry in files {
        let Some(role) = entry_role(entry) else {
            continue;
        };
        let index = usize::from(role == Role::Compute);
        let path = entry_path(entry);
        if path.len() > limits.max_path_bytes {
            return Err(ApplicationMaterializationError::Limit("path byte limit"));
        }
        let common = common_prefix_bytes(previous[index], path);
        let directories = path
            .match_indices('/')
            .filter(|(offset, _)| *offset >= common)
            .count() as u64;
        let item = &mut usage[index];
        if item.entries == 0 {
            item.nodes = 1;
        }
        item.nodes = item
            .nodes
            .checked_add(directories)
            .and_then(|nodes| nodes.checked_add(1))
            .ok_or(ApplicationMaterializationError::Limit(
                "node count overflow",
            ))?;
        item.entries += 1;
        match entry {
            FileEntry::File { size, .. } => {
                item.file_bytes = item
                    .file_bytes
                    .checked_add(*size)
                    .ok_or(ApplicationMaterializationError::Limit("file byte overflow"))?;
            }
            FileEntry::Symlink { link_target, .. }
                if link_target.0.len() > limits.max_path_bytes =>
            {
                return Err(ApplicationMaterializationError::Limit(
                    "symlink target byte limit",
                ));
            }
            _ => {}
        }
        previous[index] = path;
    }
    let mut staging_bytes = 0_u64;
    let mut staging_nodes = 1_u64;
    for item in usage.iter().filter(|item| item.entries != 0) {
        staging_bytes = item
            .file_bytes
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(limits.max_package_bytes))
            .and_then(|bytes| bytes.checked_add(staging_bytes))
            .ok_or(ApplicationMaterializationError::Limit(
                "staging byte overflow",
            ))?;
        staging_nodes = item
            .nodes
            .checked_mul(2)
            .and_then(|nodes| nodes.checked_add(2))
            .and_then(|nodes| nodes.checked_add(staging_nodes))
            .ok_or(ApplicationMaterializationError::Limit(
                "staging node overflow",
            ))?;
        let index_bytes = item
            .entries
            .checked_mul(size_of::<&FileEntry>())
            .and_then(|bytes| (bytes as u64).checked_add(working_headroom(limits).ok()?))
            .ok_or(ApplicationMaterializationError::Limit(
                "verification memory overflow",
            ))?;
        if index_bytes > limits.max_verification_memory_bytes {
            return Err(ApplicationMaterializationError::Limit(
                "verification memory limit",
            ));
        }
    }
    if staging_bytes > limits.max_staging_file_bytes {
        return Err(ApplicationMaterializationError::Limit("staging byte limit"));
    }
    if staging_nodes > limits.max_staging_nodes {
        return Err(ApplicationMaterializationError::Limit("staging node limit"));
    }
    Ok(usage)
}

fn working_headroom(
    limits: ApplicationMaterializationLimits,
) -> Result<u64, ApplicationMaterializationError> {
    (limits.max_path_bytes as u64)
        .checked_mul(8)
        .and_then(|bytes| bytes.checked_add(WORKING_HEADROOM))
        .ok_or(ApplicationMaterializationError::Limit(
            "verification memory overflow",
        ))
}

fn entry_role(entry: &FileEntry) -> Option<Role> {
    let (FileEntry::File { role, .. } | FileEntry::Symlink { role, .. }) = entry;
    match role {
        FileRole::Compute => Some(Role::Compute),
        FileRole::Static | FileRole::Prerender | FileRole::Config => Some(Role::Static),
        FileRole::Dependency => None,
    }
}

fn recipe(expected: &[&FileEntry], role: Role, generator: &str) -> [u8; 32] {
    fn field(hash: &mut blake3::Hasher, value: &[u8]) {
        hash.update(&(value.len() as u64).to_le_bytes());
        hash.update(value);
    }
    let mut hash = blake3::Hasher::new_derive_key(RECIPE_CONTEXT);
    field(&mut hash, APPLICATION_EROFS_PROFILE_V1.as_bytes());
    field(&mut hash, generator.as_bytes());
    field(
        &mut hash,
        match role {
            Role::Static => b"STATIC",
            Role::Compute => b"COMPUTE",
        },
    );
    hash.update(&(expected.len() as u64).to_le_bytes());
    for entry in expected {
        field(&mut hash, entry_path(entry).as_bytes());
        match entry {
            FileEntry::File {
                size, blob_digest, ..
            } => {
                hash.update(b"f");
                hash.update(&size.to_le_bytes());
                field(&mut hash, blob_digest.value.as_bytes());
            }
            FileEntry::Symlink { link_target, .. } => {
                hash.update(b"l");
                field(&mut hash, link_target.0.as_bytes());
            }
        }
    }
    *hash.finalize().as_bytes()
}

fn copy_role_tree(
    source: &PreparedSourceV2,
    expected: &[&FileEntry],
    tree: &Path,
) -> Result<(), ApplicationMaterializationError> {
    create_directory(tree)?;
    let mut previous = "";
    let mut buffer = [0_u8; 64 * 1024];
    for entry in expected {
        let path = entry_path(entry);
        let common = common_prefix_bytes(previous, path);
        for (slash, _) in path
            .match_indices('/')
            .filter(|(slash, _)| *slash >= common)
        {
            create_directory(&tree.join(&path[..slash]))?;
        }
        let target = tree.join(path);
        match entry {
            FileEntry::File { size, .. } => {
                let mut reader = source.open_file(path)?;
                let mut writer = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target)?;
                let mut remaining = *size;
                while remaining != 0 {
                    let wanted = remaining.min(buffer.len() as u64) as usize;
                    reader.read_exact(&mut buffer[..wanted])?;
                    writer.write_all(&buffer[..wanted])?;
                    remaining -= wanted as u64;
                }
                if reader.read(&mut buffer[..1])? != 0 {
                    return invalid("prepared file length changed");
                }
                writer.set_permissions(Permissions::from_mode(0o775))?;
            }
            FileEntry::Symlink { link_target, .. } => symlink(&link_target.0, target)?,
        }
        previous = path;
    }
    Ok(())
}

fn invalid<T>(message: &str) -> Result<T, ApplicationMaterializationError> {
    Err(ApplicationMaterializationError::Invalid(message.into()))
}

fn invalid_error(error: impl std::fmt::Display) -> ApplicationMaterializationError {
    ApplicationMaterializationError::Invalid(error.to_string())
}

#[cfg(test)]
#[path = "application_package_tests.rs"]
mod tests;
