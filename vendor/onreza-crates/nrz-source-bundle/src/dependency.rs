// @generated vendored copy of platform crates/nrz-source-bundle/src/dependency.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::{
    SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH, SourceLogicalManifest, SourceLogicalManifestEntryType,
    SourceLogicalManifestFile, normalize_source_path,
};

const DEPENDENCY_FILE_ROLE: &str = "dependency";
pub const PYTHON_314_SITE_PACKAGES_ROOT: &str = ".onreza/python/3.14/site-packages";
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
        let output = tree.path.join(relative);
        ensure_parent_directories(&tree.path, &output)?;

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
                let resolved = resolve_symlink_target(&path, &link_target)?;
                if !dependency_symlink_target_is_allowed(&groups, &source_root, &resolved) {
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
    if path
        .strip_prefix(PYTHON_314_SITE_PACKAGES_ROOT)
        .is_some_and(|suffix| suffix.starts_with('/'))
    {
        return Some(PYTHON_314_SITE_PACKAGES_ROOT.to_string());
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
    output: &Path,
) -> Result<(), DependencySourceTreeError> {
    let parent = output.parent().ok_or_else(|| {
        DependencySourceTreeError::Manifest(format!(
            "dependency output has no parent: {}",
            output.display()
        ))
    })?;
    fs::create_dir_all(parent)
        .map_err(|source| io_error("create dependency directories", parent, source))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|source| io_error("canonicalize dependency directory", parent, source))?;
    let canonical_root = fs::canonicalize(tree_root)
        .map_err(|source| io_error("canonicalize dependency tree", tree_root, source))?;
    if !canonical_parent.starts_with(canonical_root) {
        return Err(DependencySourceTreeError::Archive(format!(
            "dependency archive path escaped extraction root: {}",
            output.display()
        )));
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

#[cfg(unix)]
fn create_dependency_symlink(link_target: &str, output: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(link_target, output)
}

#[cfg(not(unix))]
fn create_dependency_symlink(_link_target: &str, _output: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "dependency symlink extraction requires a Unix platform",
    ))
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

fn resolve_symlink_target(
    path: &str,
    link_target: &str,
) -> Result<String, DependencySourceTreeError> {
    if link_target.is_empty() || link_target.contains('\\') || link_target.contains('\0') {
        return Err(DependencySourceTreeError::Manifest(format!(
            "dependency symlink target is invalid: {path} -> {link_target}"
        )));
    }
    let mut resolved = PathBuf::new();
    if let Some(parent) = Path::new(path).parent() {
        resolved.push(parent);
    }
    for component in Path::new(link_target).components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(DependencySourceTreeError::Manifest(format!(
                        "dependency symlink escapes archive root: {path} -> {link_target}"
                    )));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(DependencySourceTreeError::Manifest(format!(
                    "dependency symlink target is absolute: {path} -> {link_target}"
                )));
            }
        }
    }
    let resolved = resolved.to_str().ok_or_else(|| {
        DependencySourceTreeError::Manifest(format!(
            "dependency symlink target is not UTF-8: {path} -> {link_target}"
        ))
    })?;
    normalize_source_path(&resolved.replace('\\', "/")).map_err(DependencySourceTreeError::Manifest)
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
mod tests {
    use std::io::Cursor;

    use tempfile::tempdir;

    use super::*;
    use crate::{SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifestLayer, sha256_hex};

    #[cfg(unix)]
    #[test]
    fn extracts_a_closed_dependency_tree_with_exact_modes_and_links() {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let destination = directory.path().join("dependencies");
        fs::create_dir(&destination).unwrap();
        let script = b"#!/usr/bin/env node\n";
        let files = vec![
            dependency_file(
                "server/node_modules/pkg/bin.js",
                script,
                true,
                SourceLogicalManifestEntryType::File,
                None,
            ),
            dependency_file(
                "server/node_modules/.bin/pkg",
                b"",
                false,
                SourceLogicalManifestEntryType::Symlink,
                Some("../pkg/bin.js"),
            ),
        ];
        write_bundle(
            &bundle_path,
            &files,
            &[(files[0].path.as_str(), script, 0o755)],
        );
        let manifest = manifest(files);

        let trees = extract_dependency_source_trees(&bundle_path, &manifest, &destination).unwrap();

        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].source_root, "server/node_modules");
        assert_eq!(trees[0].mount_point, "/output/server/node_modules");
        let extracted = trees[0].path.join("pkg/bin.js");
        assert_eq!(fs::read(&extracted).unwrap(), script);
        assert_eq!(
            fs::metadata(&extracted).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert_eq!(
            fs::read_link(trees[0].path.join(".bin/pkg")).unwrap(),
            PathBuf::from("../pkg/bin.js")
        );
    }

    #[test]
    fn derives_canonical_dependency_source_tree_specs() {
        let files = vec![
            dependency_file(
                "server/node_modules/pkg/index.js",
                b"module",
                false,
                SourceLogicalManifestEntryType::File,
                None,
            ),
            dependency_file(
                "server/node_modules/pkg/bin.js",
                b"bin",
                true,
                SourceLogicalManifestEntryType::File,
                None,
            ),
        ];

        let specs = dependency_source_tree_specs(&manifest(files)).unwrap();

        assert_eq!(
            specs,
            vec![DependencySourceTreeSpec {
                source_root: "server/node_modules".to_string(),
                layer_name: "server".to_string(),
                mount_point: "/output/server/node_modules".to_string(),
                file_count: 2,
                logical_bytes: 9,
            }]
        );
    }

    #[test]
    fn rejects_a_dependency_archive_mode_that_disagrees_with_the_manifest() {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let destination = directory.path().join("dependencies");
        fs::create_dir(&destination).unwrap();
        let files = vec![dependency_file(
            "node_modules/pkg/bin.js",
            b"bin",
            true,
            SourceLogicalManifestEntryType::File,
            None,
        )];
        write_bundle(
            &bundle_path,
            &files,
            &[(files[0].path.as_str(), b"bin", 0o644)],
        );

        let error = extract_dependency_source_trees(&bundle_path, &manifest(files), &destination)
            .unwrap_err();

        assert!(error.to_string().contains("executable mode mismatch"));
    }

    #[cfg(unix)]
    #[test]
    fn extracts_a_dependency_symlink_into_another_tree_of_the_same_layer() {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let destination = directory.path().join("dependencies");
        fs::create_dir(&destination).unwrap();
        let client = b"module.exports = {}\n";
        let files = vec![
            dependency_file(
                "node_modules/@prisma/client/index.js",
                client,
                false,
                SourceLogicalManifestEntryType::File,
                None,
            ),
            dependency_file(
                ".next/node_modules/@prisma/client-generated",
                b"",
                false,
                SourceLogicalManifestEntryType::Symlink,
                Some("../../../node_modules/@prisma/client"),
            ),
        ];
        write_bundle(
            &bundle_path,
            &files,
            &[(files[0].path.as_str(), client, 0o644)],
        );

        let trees =
            extract_dependency_source_trees(&bundle_path, &manifest(files), &destination).unwrap();

        let next_tree = trees
            .iter()
            .find(|tree| tree.source_root == ".next/node_modules")
            .unwrap();
        assert_eq!(
            fs::read_link(next_tree.path.join("@prisma/client-generated")).unwrap(),
            PathBuf::from("../../../node_modules/@prisma/client")
        );
    }

    #[test]
    fn rejects_a_dependency_symlink_to_an_unowned_layer_path() {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let destination = directory.path().join("dependencies");
        fs::create_dir(&destination).unwrap();
        let files = vec![dependency_file(
            ".next/node_modules/@prisma/client-generated",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("../../../node_modules/@prisma/client"),
        )];
        write_bundle(&bundle_path, &files, &[]);

        let error = extract_dependency_source_trees(&bundle_path, &manifest(files), &destination)
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("escapes its allowed layer roots")
        );
    }

    fn dependency_file(
        path: &str,
        contents: &[u8],
        executable: bool,
        entry_type: SourceLogicalManifestEntryType,
        link_target: Option<&str>,
    ) -> SourceLogicalManifestFile {
        SourceLogicalManifestFile {
            path: path.to_string(),
            sha256: link_target
                .map(|target| sha256_hex(target.as_bytes()))
                .unwrap_or_else(|| sha256_hex(contents)),
            size: u64::try_from(contents.len()).unwrap(),
            content_type: None,
            role: DEPENDENCY_FILE_ROLE.to_string(),
            layer_name: Some("server".to_string()),
            entry_type,
            link_target: link_target.map(str::to_string),
            executable,
        }
    }

    fn manifest(files: Vec<SourceLogicalManifestFile>) -> SourceLogicalManifest {
        SourceLogicalManifest {
            schema_version: SOURCE_BUNDLE_V1_SCHEMA_VERSION.to_string(),
            capabilities: Vec::new(),
            files,
            layers: vec![SourceLogicalManifestLayer {
                name: "server".to_string(),
                target: "COMPUTE".to_string(),
                root_path: Some("server".to_string()),
                entrypoint: None,
                runtime_config: None,
            }],
            routes: Vec::new(),
            entrypoints: Vec::new(),
        }
    }

    fn write_bundle(
        path: &Path,
        manifest_files: &[SourceLogicalManifestFile],
        regular_files: &[(&str, &[u8], u32)],
    ) {
        let encoder = zstd::stream::write::Encoder::new(File::create(path).unwrap(), 1).unwrap();
        let mut archive = tar::Builder::new(encoder);
        let manifest = serde_json::to_vec(&manifest(manifest_files.to_vec())).unwrap();
        append_file(
            &mut archive,
            SOURCE_BUNDLE_LOGICAL_MANIFEST_PATH,
            &manifest,
            0o644,
        );
        for (entry_path, contents, mode) in regular_files {
            append_file(&mut archive, entry_path, contents, *mode);
        }
        for file in manifest_files
            .iter()
            .filter(|file| file.entry_type == SourceLogicalManifestEntryType::Symlink)
        {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_mode(0o777);
            header.set_size(0);
            header.set_cksum();
            archive
                .append_link(
                    &mut header,
                    &file.path,
                    file.link_target.as_deref().unwrap(),
                )
                .unwrap();
        }
        let encoder = archive.into_inner().unwrap();
        encoder.finish().unwrap();
    }

    fn append_file<W: Write>(
        archive: &mut tar::Builder<W>,
        path: &str,
        contents: &[u8],
        mode: u32,
    ) {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(mode);
        header.set_size(u64::try_from(contents.len()).unwrap());
        header.set_cksum();
        archive
            .append_data(&mut header, path, Cursor::new(contents))
            .unwrap();
    }
}
