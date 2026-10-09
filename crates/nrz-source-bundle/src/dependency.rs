use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SourceArchivePathIndex, SourceLogicalManifest,
    SourceLogicalManifestEntryType, SourceLogicalManifestFile, SourcePathGraphError,
    normalize_source_path,
};

const DEPENDENCY_FILE_ROLE: &str = "dependency";
pub const PYTHON_314_SITE_PACKAGES_ROOT: &str = crate::PythonMinor::Python314.site_packages_root();
#[cfg(unix)]
const FILE_MODE: u32 = 0o644;
#[cfg(unix)]
const EXECUTABLE_FILE_MODE: u32 = 0o755;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencySourceTree {
    pub source_root: String,
    pub layer_name: String,
    pub mount_point: String,
    pub path: PathBuf,
    pub file_count: u64,
    pub logical_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencySourceTreeSpec {
    pub source_root: String,
    pub layer_name: String,
    pub mount_point: String,
    pub file_count: u64,
    pub logical_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DependencySourceTreeError {
    #[error("dependency source manifest is invalid: {0}")]
    Manifest(String),
    #[error("dependency source archive is invalid: {0}")]
    Archive(String),
    #[error("dependency source tree I/O failed while attempting to {operation} at {path}", path = .path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

struct DependencyGroup<'a> {
    layer_name: String,
    files: HashMap<String, &'a SourceLogicalManifestFile>,
    file_count: u64,
    logical_bytes: u64,
}

pub fn extract_dependency_source_trees(
    bundle_path: &Path,
    manifest: &SourceLogicalManifest,
    destination: &Path,
) -> Result<Vec<DependencySourceTree>, DependencySourceTreeError> {
    let groups = dependency_groups(manifest)?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let mut owned_paths = HashMap::<&str, HashSet<&str>>::new();
    for group in groups.values() {
        let owned = owned_paths.entry(&group.layer_name).or_default();
        for path in group.files.keys() {
            owned.insert(path);
            for (offset, _) in path.match_indices('/') {
                owned.insert(&path[..offset]);
            }
        }
    }
    let paths = SourceArchivePathIndex::from_entries(manifest.files.iter().map(|file| {
        (
            file.path.as_str(),
            match file.entry_type {
                SourceLogicalManifestEntryType::File => None,
                SourceLogicalManifestEntryType::Symlink => file.link_target.as_deref(),
            },
        )
    }));
    prepare_empty_destination(destination)?;

    let mut trees = BTreeMap::new();
    for (source_root, group) in &groups {
        let tree_path = destination.join(group_directory_name(source_root));
        fs::create_dir(&tree_path)
            .map_err(|source| io_error("create dependency tree", &tree_path, source))?;
        trees.insert(
            source_root.clone(),
            DependencySourceTree {
                source_root: source_root.clone(),
                layer_name: group.layer_name.clone(),
                mount_point: format!("/output/{source_root}"),
                path: tree_path,
                file_count: group.file_count,
                logical_bytes: group.logical_bytes,
            },
        );
    }

    let bundle = File::open(bundle_path)
        .map_err(|source| io_error("open source bundle", bundle_path, source))?;
    let decoder = zstd::stream::read::Decoder::new(bundle)
        .map_err(|error| DependencySourceTreeError::Archive(error.to_string()))?;
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| DependencySourceTreeError::Archive(error.to_string()))?;
    let mut extracted = HashSet::new();

    for entry in entries {
        let mut entry =
            entry.map_err(|error| DependencySourceTreeError::Archive(error.to_string()))?;
        let path = entry
            .path()
            .map_err(|error| DependencySourceTreeError::Archive(error.to_string()))?
            .to_str()
            .ok_or_else(|| {
                DependencySourceTreeError::Archive("archive contains a non-UTF-8 path".to_string())
            })?
            .to_string();
        if path == SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH {
            continue;
        }
        let path = normalize_source_path(&path).map_err(DependencySourceTreeError::Archive)?;
        let Some((source_root, file)) = expected_dependency_file(&groups, &path) else {
            continue;
        };
        if !extracted.insert(path.clone()) {
            return Err(DependencySourceTreeError::Archive(format!(
                "duplicate dependency archive path: {path}"
            )));
        }
        let relative = dependency_relative_path(&source_root, &path)?;
        let tree = trees
            .get(&source_root)
            .expect("tree is created for every dependency group");
        let output = tree.path.join(&relative);
        ensure_parent_directories(&tree.path, &relative)?;

        match file.entry_type {
            SourceLogicalManifestEntryType::File => {
                if !entry.header().entry_type().is_file() || entry.size() != file.size {
                    return Err(DependencySourceTreeError::Archive(format!(
                        "dependency archive entry type or size mismatch: {path}"
                    )));
                }
                let executable = entry
                    .header()
                    .mode()
                    .map_err(|error| DependencySourceTreeError::Archive(error.to_string()))?
                    & 0o111
                    != 0;
                if executable != file.executable {
                    return Err(DependencySourceTreeError::Archive(format!(
                        "dependency archive executable mode mismatch: {path}"
                    )));
                }
                write_verified_file(&mut entry, &output, file, executable)?;
            }
            SourceLogicalManifestEntryType::Symlink => {
                if !entry.header().entry_type().is_symlink() {
                    return Err(DependencySourceTreeError::Archive(format!(
                        "dependency archive symlink type mismatch: {path}"
                    )));
                }
                let link_target = entry
                    .link_name()
                    .map_err(|error| DependencySourceTreeError::Archive(error.to_string()))?
                    .and_then(|target| target.to_str().map(str::to_string))
                    .ok_or_else(|| {
                        DependencySourceTreeError::Archive(format!(
                            "dependency archive symlink target is invalid: {path}"
                        ))
                    })?;
                if file.link_target.as_deref() != Some(link_target.as_str()) {
                    return Err(DependencySourceTreeError::Archive(format!(
                        "dependency archive symlink target mismatch: {path}"
                    )));
                }
                validate_dependency_link_target(&path, &link_target)?;
                let paths = paths.as_ref().map_err(|error| match error {
                    SourcePathGraphError::FileAncestor(_)
                    | SourcePathGraphError::SymlinkAncestor(_) => {
                        DependencySourceTreeError::Archive(error.to_string())
                    }
                    _ => DependencySourceTreeError::Manifest(error.to_string()),
                })?;
                let (required, resolved) = paths.resolve_symlink(&path).map_err(|error| {
                    let reason = match error {
                        SourcePathGraphError::UnsafeTarget(_) => "escapes archive root",
                        _ => "escapes its allowed layer roots",
                    };
                    DependencySourceTreeError::Manifest(format!(
                        "dependency symlink {reason}: {path} -> {link_target} ({error})"
                    ))
                })?;
                let owned = &owned_paths[tree.layer_name.as_str()];
                if !dependency_symlink_target_is_allowed(&groups, &source_root, &resolved)
                    || required.iter().any(|path| !owned.contains(path))
                {
                    return Err(DependencySourceTreeError::Manifest(format!(
                        "dependency symlink escapes its allowed layer roots: {path} -> {link_target}"
                    )));
                }
                create_dependency_symlink(&link_target, &output)
                    .map_err(|source| io_error("create dependency symlink", &output, source))?;
            }
        }
    }

    for group in groups.values() {
        for path in group.files.keys() {
            if !extracted.contains(path) {
                return Err(DependencySourceTreeError::Archive(format!(
                    "dependency manifest entry is missing from archive: {path}"
                )));
            }
        }
    }
    Ok(trees.into_values().collect())
}

pub fn dependency_source_tree_specs(
    manifest: &SourceLogicalManifest,
) -> Result<Vec<DependencySourceTreeSpec>, DependencySourceTreeError> {
    Ok(dependency_groups(manifest)?
        .into_iter()
        .map(|(source_root, group)| DependencySourceTreeSpec {
            mount_point: format!("/output/{source_root}"),
            source_root,
            layer_name: group.layer_name,
            file_count: group.file_count,
            logical_bytes: group.logical_bytes,
        })
        .collect())
}

fn dependency_groups(
    manifest: &SourceLogicalManifest,
) -> Result<BTreeMap<String, DependencyGroup<'_>>, DependencySourceTreeError> {
    let mut groups = BTreeMap::<String, DependencyGroup<'_>>::new();
    for file in manifest
        .files
        .iter()
        .filter(|file| file.role == DEPENDENCY_FILE_ROLE)
    {
        let path =
            normalize_source_path(&file.path).map_err(DependencySourceTreeError::Manifest)?;
        let source_root = dependency_source_root(&path).ok_or_else(|| {
            DependencySourceTreeError::Manifest(format!(
                "dependency file is not inside a supported dependency root: {path}"
            ))
        })?;
        if dependency_relative_path(&source_root, &path)?
            .as_os_str()
            .is_empty()
        {
            return Err(DependencySourceTreeError::Manifest(format!(
                "dependency file cannot own its source root: {path}"
            )));
        }
        let layer_name = file.layer_name.as_deref().ok_or_else(|| {
            DependencySourceTreeError::Manifest(format!("dependency file has no layerName: {path}"))
        })?;
        let group = groups
            .entry(source_root)
            .or_insert_with(|| DependencyGroup {
                layer_name: layer_name.to_string(),
                files: HashMap::new(),
                file_count: 0,
                logical_bytes: 0,
            });
        if group.layer_name != layer_name {
            return Err(DependencySourceTreeError::Manifest(format!(
                "dependency source root is shared by multiple compute layers: {path}"
            )));
        }
        if group.files.insert(path.clone(), file).is_some() {
            return Err(DependencySourceTreeError::Manifest(format!(
                "duplicate dependency manifest path: {path}"
            )));
        }
        group.file_count = group.file_count.saturating_add(1);
        group.logical_bytes = group.logical_bytes.saturating_add(file.size);
    }
    Ok(groups)
}

fn expected_dependency_file<'a>(
    groups: &'a BTreeMap<String, DependencyGroup<'a>>,
    path: &str,
) -> Option<(String, &'a SourceLogicalManifestFile)> {
    let source_root = dependency_source_root(path)?;
    let group = groups.get(&source_root)?;
    let file = group.files.get(path)?;
    Some((source_root, *file))
}

fn dependency_source_root(path: &str) -> Option<String> {
    if let Some(minor) = crate::PythonMinor::for_dependency_path(path) {
        return Some(minor.site_packages_root().to_string());
    }
    let mut parts = Vec::new();
    for segment in path.split('/') {
        parts.push(segment);
        if segment == "node_modules" {
            return Some(parts.join("/"));
        }
    }
    None
}

fn dependency_relative_path(
    source_root: &str,
    path: &str,
) -> Result<PathBuf, DependencySourceTreeError> {
    let relative = path
        .strip_prefix(source_root)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .ok_or_else(|| {
            DependencySourceTreeError::Manifest(format!(
                "dependency path is outside its source root: {path}"
            ))
        })?;
    Ok(PathBuf::from(relative))
}

fn prepare_empty_destination(destination: &Path) -> Result<(), DependencySourceTreeError> {
    let metadata = fs::metadata(destination)
        .map_err(|source| io_error("inspect dependency destination", destination, source))?;
    if !metadata.is_dir() {
        return Err(DependencySourceTreeError::Manifest(format!(
            "dependency destination is not a directory: {}",
            destination.display()
        )));
    }
    let mut entries = fs::read_dir(destination)
        .map_err(|source| io_error("read dependency destination", destination, source))?;
    if entries
        .next()
        .transpose()
        .map_err(|source| io_error("read dependency destination entry", destination, source))?
        .is_some()
    {
        return Err(DependencySourceTreeError::Manifest(format!(
            "dependency destination is not empty: {}",
            destination.display()
        )));
    }
    Ok(())
}

fn group_directory_name(source_root: &str) -> String {
    let digest = Sha256::digest(source_root.as_bytes());
    format!("dependency-{}", &hex::encode(digest)[..16])
}

fn ensure_parent_directories(
    tree_root: &Path,
    relative: &Path,
) -> Result<(), DependencySourceTreeError> {
    let mut directory = tree_root.to_path_buf();
    let parent = relative.parent().expect("dependency path has a file name");
    for component in parent.components() {
        directory.push(component);
        if let Err(source) = fs::create_dir(&directory) {
            let metadata = fs::symlink_metadata(&directory)
                .map_err(|_| io_error("create dependency directories", &directory, source))?;
            if !metadata.is_dir() {
                return Err(DependencySourceTreeError::Archive(format!(
                    "dependency archive parent is not a directory: {}",
                    directory.display()
                )));
            }
        }
    }
    Ok(())
}

fn write_verified_file<R: Read>(
    reader: &mut R,
    output: &Path,
    expected: &SourceLogicalManifestFile,
    executable: bool,
) -> Result<(), DependencySourceTreeError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(FILE_MODE);
    let mut file = options
        .open(output)
        .map_err(|source| io_error("create dependency file", output, source))?;
    let mut hasher = Sha256::new();
    let mut remaining = expected.size;
    let mut buffer = vec![0_u8; 64 * 1024];
    while remaining > 0 {
        let limit = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = reader
            .read(&mut buffer[..limit])
            .map_err(|source| io_error("read dependency archive entry", output, source))?;
        if read == 0 {
            return Err(DependencySourceTreeError::Archive(format!(
                "dependency archive entry ended early: {}",
                expected.path
            )));
        }
        file.write_all(&buffer[..read])
            .map_err(|source| io_error("write dependency file", output, source))?;
        hasher.update(&buffer[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    if hex::encode(hasher.finalize()) != expected.sha256 {
        return Err(DependencySourceTreeError::Archive(format!(
            "dependency archive entry digest mismatch: {}",
            expected.path
        )));
    }
    set_dependency_file_permissions(output, executable)
        .map_err(|source| io_error("set dependency file permissions", output, source))?;
    Ok(())
}

fn create_dependency_symlink(link_target: &str, output: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(link_target, output)
    }
    #[cfg(not(unix))]
    {
        let _ = (link_target, output);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "dependency symlink extraction requires a Unix platform",
        ))
    }
}

#[cfg(unix)]
fn set_dependency_file_permissions(output: &Path, executable: bool) -> std::io::Result<()> {
    let mode = if executable {
        EXECUTABLE_FILE_MODE
    } else {
        FILE_MODE
    };
    fs::set_permissions(output, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_dependency_file_permissions(_output: &Path, _executable: bool) -> std::io::Result<()> {
    Ok(())
}

fn validate_dependency_link_target(
    path: &str,
    link_target: &str,
) -> Result<(), DependencySourceTreeError> {
    if link_target.is_empty() || link_target.contains('\\') || link_target.contains('\0') {
        return Err(DependencySourceTreeError::Manifest(format!(
            "dependency symlink target is invalid: {path} -> {link_target}"
        )));
    }
    if Path::new(link_target).is_absolute() {
        return Err(DependencySourceTreeError::Manifest(format!(
            "dependency symlink target is absolute: {path} -> {link_target}"
        )));
    }
    Ok(())
}

fn path_is_within(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}/"))
}

fn dependency_symlink_target_is_allowed(
    groups: &BTreeMap<String, DependencyGroup<'_>>,
    source_root: &str,
    resolved: &str,
) -> bool {
    if path_is_within(resolved, source_root) {
        return true;
    }

    let Some(source_group) = groups.get(source_root) else {
        return false;
    };
    let Some(target_root) = dependency_source_root(resolved) else {
        return false;
    };
    let Some(target_group) = groups.get(&target_root) else {
        return false;
    };
    if source_group.layer_name != target_group.layer_name {
        return false;
    }

    target_group
        .files
        .keys()
        .any(|path| path_is_within(path, resolved))
}

fn io_error(
    operation: &'static str,
    path: &Path,
    source: std::io::Error,
) -> DependencySourceTreeError {
    DependencySourceTreeError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
#[path = "dependency_tests.rs"]
mod tests;
