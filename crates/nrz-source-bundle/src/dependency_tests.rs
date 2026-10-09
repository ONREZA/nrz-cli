use std::io::Cursor;

use tempfile::tempdir;

use super::*;
use crate::{SOURCE_BUNDLE_V1_SCHEMA_VERSION, SourceLogicalManifestLayer, sha256_hex};

#[cfg(unix)]
#[test]
fn rejects_symlink_ancestors_before_creating_directories_outside_the_tree() {
    let directory = tempdir().unwrap();
    let bundle_path = directory.path().join("source.tar.zst");
    let destination = directory.path().join("trees");
    let outside = directory.path().join("node_modules/pkg");
    fs::create_dir(&destination).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let files = vec![
        dependency_file(
            "node_modules/pkg/index.js",
            b"index",
            false,
            SourceLogicalManifestEntryType::File,
            None,
        ),
        dependency_file(
            ".next/node_modules/link",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("../../node_modules/pkg"),
        ),
        dependency_file(
            ".next/node_modules/link/created/file.js",
            b"file",
            false,
            SourceLogicalManifestEntryType::File,
            None,
        ),
    ];
    let encoder =
        zstd::stream::write::Encoder::new(File::create(&bundle_path).unwrap(), 1).unwrap();
    let mut archive = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_mode(0o777);
    header.set_size(0);
    archive
        .append_link(
            &mut header,
            &files[1].path,
            files[1].link_target.as_deref().unwrap(),
        )
        .unwrap();
    append_file(&mut archive, &files[2].path, b"file", 0o644);
    archive.into_inner().unwrap().finish().unwrap();

    assert!(matches!(
        extract_dependency_source_trees(&bundle_path, &manifest(files), &destination),
        Err(DependencySourceTreeError::Archive(_))
    ));
    assert!(!outside.join("created").exists());
}

#[test]
fn rejects_archive_type_substitution_even_when_size_and_link_name_match() {
    for (expected_type, actual_type, link_target, error_message) in [
        (
            SourceLogicalManifestEntryType::File,
            tar::EntryType::Symlink,
            None,
            "entry type or size mismatch",
        ),
        (
            SourceLogicalManifestEntryType::Symlink,
            tar::EntryType::Regular,
            Some("target"),
            "symlink type mismatch",
        ),
    ] {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let destination = directory.path().join("dependencies");
        fs::create_dir(&destination).unwrap();
        let expected = dependency_file(
            "node_modules/pkg/link",
            b"",
            false,
            expected_type,
            link_target,
        );
        let encoder =
            zstd::stream::write::Encoder::new(File::create(&bundle_path).unwrap(), 1).unwrap();
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(actual_type);
        header.set_mode(0o644);
        header.set_size(0);
        header.set_link_name("target").unwrap();
        header.set_cksum();
        archive
            .append_data(&mut header, &expected.path, std::io::empty())
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap();

        let error =
            extract_dependency_source_trees(&bundle_path, &manifest(vec![expected]), &destination)
                .expect_err(
                    "matching size and link name must not permit an archive type substitution",
                );
        assert!(error.to_string().contains(error_message), "{error}");
    }
}

#[test]
fn rejects_a_nonempty_destination_without_changing_existing_files() {
    let directory = tempdir().unwrap();
    let bundle_path = directory.path().join("source.tar.zst");
    let destination = directory.path().join("dependencies");
    fs::create_dir(&destination).unwrap();
    let sentinel = destination.join("existing");
    fs::write(&sentinel, b"keep me").unwrap();
    let files = vec![dependency_file(
        "node_modules/pkg/index.js",
        b"module",
        false,
        SourceLogicalManifestEntryType::File,
        None,
    )];
    write_bundle(
        &bundle_path,
        &files,
        &[(files[0].path.as_str(), b"module", 0o644)],
    );

    let error = extract_dependency_source_trees(&bundle_path, &manifest(files), &destination)
        .expect_err("extraction must require an empty destination");
    assert!(
        error.to_string().contains("destination is not empty"),
        "{error}"
    );
    assert_eq!(fs::read(&sentinel).unwrap(), b"keep me");
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
}

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
            "server/node_modules/pkg/index.js",
            b"module",
            false,
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
        &[
            (files[0].path.as_str(), script, 0o755),
            (files[1].path.as_str(), b"module", 0o644),
        ],
    );
    let manifest = manifest(files);

    let trees = extract_dependency_source_trees(&bundle_path, &manifest, &destination).unwrap();

    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0].source_root, "server/node_modules");
    assert_eq!(trees[0].mount_point, "/output/server/node_modules");
    let extracted = trees[0].path.join("pkg/bin.js");
    assert_eq!(fs::read(&extracted).unwrap(), script);
    let sibling = trees[0].path.join("pkg/index.js");
    assert_eq!(fs::read(&sibling).unwrap(), b"module");
    assert_eq!(
        fs::metadata(&sibling).unwrap().permissions().mode() & 0o777,
        0o644
    );
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

    let error =
        extract_dependency_source_trees(&bundle_path, &manifest(files), &destination).unwrap_err();

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
    // Reconstruct the declared mount layout; staging hashes are not runtime paths.
    let output = directory.path().join("output");
    for tree in &trees {
        let mounted = output.join(&tree.source_root);
        fs::create_dir_all(mounted.parent().unwrap()).unwrap();
        fs::rename(&tree.path, &mounted).unwrap();
    }
    assert_eq!(
        fs::read(output.join(".next/node_modules/@prisma/client-generated/index.js")).unwrap(),
        client
    );
}

#[test]
fn rejects_nonportable_absolute_and_escaping_dependency_symlink_targets() {
    for (target, message) in [
        ("child\\file.js", "target is invalid"),
        ("/absolute/file.js", "target is absolute"),
        ("../../../outside.js", "escapes archive root"),
    ] {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let destination = directory.path().join("dependencies");
        fs::create_dir(&destination).unwrap();
        let files = vec![dependency_file(
            "node_modules/pkg/link",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some(target),
        )];
        write_bundle(&bundle_path, &files, &[]);

        let error = extract_dependency_source_trees(&bundle_path, &manifest(files), &destination)
            .expect_err("an invalid dependency symlink target must be rejected before creation");
        assert!(matches!(error, DependencySourceTreeError::Manifest(_)));
        assert!(error.to_string().contains(message), "{target:?}: {error}");
    }
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

    let error =
        extract_dependency_source_trees(&bundle_path, &manifest(files), &destination).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("escapes its allowed layer roots")
    );
}

#[cfg(unix)]
#[test]
fn extracts_physically_owned_symlink_parent_traversal() {
    let directory = tempdir().unwrap();
    let bundle_path = directory.path().join("source.tar.zst");
    let files = vec![
        dependency_file(
            "node_modules/pkg/real",
            b"real",
            false,
            SourceLogicalManifestEntryType::File,
            None,
        ),
        dependency_file(
            "node_modules/pkg/deeper/child/file",
            b"witness",
            false,
            SourceLogicalManifestEntryType::File,
            None,
        ),
        dependency_file(
            "node_modules/alias",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("pkg/deeper/child"),
        ),
        dependency_file(
            "node_modules/link",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("alias/../../real"),
        ),
    ];
    write_bundle(
        &bundle_path,
        &files,
        &[
            ("node_modules/pkg/real", b"real", 0o644),
            ("node_modules/pkg/deeper/child/file", b"witness", 0o644),
        ],
    );
    let restored = unpack_bundle(&bundle_path, directory.path());
    assert_eq!(
        fs::read(restored.join("node_modules/link")).unwrap(),
        b"real"
    );

    let destination = directory.path().join("trees");
    fs::create_dir(&destination).unwrap();
    let trees =
        extract_dependency_source_trees(&bundle_path, &manifest(files), &destination).unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(fs::read(trees[0].path.join("link")).unwrap(), b"real");
}

#[cfg(unix)]
#[test]
fn rejects_physically_unowned_target_hidden_by_lexical_parent_traversal() {
    let directory = tempdir().unwrap();
    let bundle_path = directory.path().join("source.tar.zst");
    let mut application = dependency_file(
        "admin/secret",
        b"real",
        false,
        SourceLogicalManifestEntryType::File,
        None,
    );
    application.role = "compute".into();
    let files = vec![
        dependency_file(
            "node_modules/pkg/index.js",
            b"real",
            false,
            SourceLogicalManifestEntryType::File,
            None,
        ),
        application,
        dependency_file(
            ".next/node_modules/alias",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("../../node_modules"),
        ),
        dependency_file(
            ".next/node_modules/use",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("alias/../admin/secret"),
        ),
    ];
    write_bundle(
        &bundle_path,
        &files,
        &[
            ("node_modules/pkg/index.js", b"real", 0o644),
            ("admin/secret", b"real", 0o644),
        ],
    );
    let restored = unpack_bundle(&bundle_path, directory.path());
    assert_eq!(
        fs::canonicalize(restored.join(".next/node_modules/use")).unwrap(),
        restored.join("admin/secret")
    );
    assert_eq!(
        fs::read(restored.join(".next/node_modules/use")).unwrap(),
        b"real"
    );

    assert!(matches!(
        extract_fixture_dependencies(directory.path(), &bundle_path, files),
        Err(DependencySourceTreeError::Manifest(_))
    ));
}

#[cfg(unix)]
#[test]
fn rejects_dependency_links_requiring_an_alias_that_is_not_extracted() {
    let directory = tempdir().unwrap();
    let bundle_path = directory.path().join("source.tar.zst");
    let mut bridge = dependency_file(
        "server/bridge",
        b"",
        false,
        SourceLogicalManifestEntryType::Symlink,
        Some("../node_modules/pkg"),
    );
    bridge.role = "compute".into();
    let files = vec![
        dependency_file(
            "node_modules/pkg/file",
            b"real",
            false,
            SourceLogicalManifestEntryType::File,
            None,
        ),
        dependency_file(
            "node_modules/link",
            b"",
            false,
            SourceLogicalManifestEntryType::Symlink,
            Some("../server/bridge/file"),
        ),
        bridge,
    ];
    write_bundle(
        &bundle_path,
        &files,
        &[("node_modules/pkg/file", b"real", 0o644)],
    );
    let restored = unpack_bundle(&bundle_path, directory.path());
    assert_eq!(
        fs::read(restored.join("node_modules/link")).unwrap(),
        b"real"
    );
    assert!(matches!(
        extract_fixture_dependencies(directory.path(), &bundle_path, files),
        Err(DependencySourceTreeError::Manifest(_))
    ));
}

#[cfg(unix)]
#[test]
fn dependency_endpoint_may_alias_its_root_but_not_a_runtime_parent_directory() {
    for (target, admitted, archive_read_path) in [
        (".", true, ".next/node_modules/link/pkg/file"),
        ("..", false, ".next/node_modules/link/node_modules/pkg/file"),
    ] {
        let directory = tempdir().unwrap();
        let bundle_path = directory.path().join("source.tar.zst");
        let files = vec![
            dependency_file(
                ".next/node_modules/pkg/file",
                b"real",
                false,
                SourceLogicalManifestEntryType::File,
                None,
            ),
            dependency_file(
                ".next/node_modules/link",
                b"",
                false,
                SourceLogicalManifestEntryType::Symlink,
                Some(target),
            ),
        ];
        write_bundle(
            &bundle_path,
            &files,
            &[(".next/node_modules/pkg/file", b"real", 0o644)],
        );
        let restored = unpack_bundle(&bundle_path, directory.path());
        assert_eq!(fs::read(restored.join(archive_read_path)).unwrap(), b"real");
        let extracted = extract_fixture_dependencies(directory.path(), &bundle_path, files);
        if admitted {
            let trees = extracted.unwrap();
            assert_eq!(
                fs::read(trees[0].path.join("link/pkg/file")).unwrap(),
                b"real"
            );
        } else {
            assert!(matches!(
                extracted,
                Err(DependencySourceTreeError::Manifest(_))
            ));
        }
    }
}

fn extract_fixture_dependencies(
    directory: &Path,
    bundle_path: &Path,
    files: Vec<SourceLogicalManifestFile>,
) -> Result<Vec<DependencySourceTree>, DependencySourceTreeError> {
    let destination = directory.join("trees");
    fs::create_dir(&destination).unwrap();
    extract_dependency_source_trees(bundle_path, &manifest(files), &destination)
}

fn unpack_bundle(bundle_path: &Path, directory: &Path) -> PathBuf {
    let restored = directory.join("restored");
    tar::Archive::new(zstd::stream::read::Decoder::new(File::open(bundle_path).unwrap()).unwrap())
        .unpack(&restored)
        .unwrap();
    restored
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

fn append_file<W: Write>(archive: &mut tar::Builder<W>, path: &str, contents: &[u8], mode: u32) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(mode);
    header.set_size(u64::try_from(contents.len()).unwrap());
    header.set_cksum();
    archive
        .append_data(&mut header, path, Cursor::new(contents))
        .unwrap();
}
