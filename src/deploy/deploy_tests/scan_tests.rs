use super::*;

#[test]
fn scan_files_flat_directory() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::write(dir.path().join("style.css"), "body{}").unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "index.html");
    assert_eq!(files[1].path, "style.css");
}

#[test]
fn scan_files_nested_directory() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("assets/images")).unwrap();
    fs::write(dir.path().join("index.html"), "hi").unwrap();
    fs::write(dir.path().join("assets/app.js"), "js").unwrap();
    fs::write(dir.path().join("assets/images/logo.png"), "png").unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 3);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"index.html"));
    assert!(paths.contains(&"assets/app.js"));
    assert!(paths.contains(&"assets/images/logo.png"));
}

#[test]
fn scan_files_skips_vcs_internal_dirs() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".git/objects/pack")).unwrap();
    fs::create_dir_all(dir.path().join("packages/app/.git/objects/pack")).unwrap();
    fs::create_dir_all(dir.path().join(".hg/store")).unwrap();
    fs::create_dir_all(dir.path().join("vendor/pkg/.svn")).unwrap();
    fs::create_dir_all(dir.path().join(".svn")).unwrap();
    fs::write(dir.path().join(".git/objects/pack/pack.dat"), "git").unwrap();
    fs::write(
        dir.path().join("packages/app/.git/objects/pack/pack.dat"),
        "nested-git",
    )
    .unwrap();
    fs::write(dir.path().join(".hg/store/data"), "hg").unwrap();
    fs::write(dir.path().join("vendor/pkg/.svn/entries"), "nested-svn").unwrap();
    fs::write(dir.path().join(".svn/entries"), "svn").unwrap();
    fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
    fs::write(dir.path().join("index.html"), "hi").unwrap();

    let files = scan_dir(dir.path()).unwrap();
    let paths = files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec![".gitignore", "index.html"]);
}

#[test]
fn scan_files_records_correct_sizes() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn lfs_pointer_requires_project_lfs_setting() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/model.glb"), git_lfs_pointer()).unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let err = ensure_no_unresolved_lfs_pointers(dir.path(), &files, false).unwrap_err();
    let coded = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("LFS pointer error must carry a structured code");

    assert_eq!(coded.code, "GIT_LFS_REQUIRED");
    assert!(err.to_string().contains("public/model.glb"), "{err}");
}

#[test]
fn lfs_pointer_still_fails_when_project_lfs_enabled() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("public")).unwrap();
    fs::write(dir.path().join("public/model.glb"), git_lfs_pointer()).unwrap();
    let files = scan_dir(dir.path()).unwrap();

    let err = ensure_no_unresolved_lfs_pointers(dir.path(), &files, true).unwrap_err();
    let coded = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
        .expect("LFS pointer error must carry a structured code");

    assert_eq!(coded.code, "GIT_LFS_UNRESOLVED");
    assert!(err.to_string().contains("git lfs pull"), "{err}");
}

#[test]
fn lfs_qualification_rejects_changed_bytes_before_the_pointer_is_restored() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("asset.txt");
    fs::write(&path, git_lfs_pointer()).unwrap();
    let root = nrz_runtime_artifact::ArtifactRoot::open(dir.path()).unwrap();
    let files = scan_runtime_artifact_rooted(&root, &RuntimeArtifactScan::All).unwrap();
    fs::write(&path, vec![b'x'; git_lfs_pointer().len()]).unwrap();
    let qualification = ensure_no_unresolved_lfs_pointers_rooted(&root, &files, false);
    fs::write(&path, git_lfs_pointer()).unwrap();
    if qualification.is_ok() {
        let plan = source_bundle_v1::build_source_bundle_plan_with_root(
            &root,
            &build_manifest::generate_static_manifest(),
            &files,
            &RuntimeArtifactScan::All,
            source_bundle_v1::RuntimeDependencyPackaging::Embedded,
            None,
        )
        .unwrap();
        let restored = crate::test_support::unpack_source_bundle(&plan);
        assert_eq!(
            fs::read(restored.path().join("asset.txt")).unwrap(),
            git_lfs_pointer().as_bytes()
        );
        panic!("an unresolved pointer was archived after qualification consumed different bytes");
    }
    assert!(
        qualification
            .unwrap_err()
            .to_string()
            .contains("changed after scanning")
    );
}

#[test]
fn qualification_attests_consumed_bytes_during_in_place_replacement() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("native.data");
    let mut original = b"\xcf\xfa\xed\xfeMACH_O".to_vec();
    original.resize(96, 0);
    fs::write(&path, &original).unwrap();
    let files = scan_dir(dir.path()).unwrap();
    let file = &files[0];
    let result = qualify_scanned_artifact_file(
        fs::File::open(&path).unwrap(),
        &file.path,
        file.size,
        &file.content_hash,
        |reader| {
            fs::write(&path, vec![b'p'; original.len()]).unwrap();
            nrz_runtime_artifact::verify_linux_x86_64_native_platform(reader)?;
            fs::write(&path, &original).unwrap();
            Ok(())
        },
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("changed after scanning"),
        "qualifying different bytes was accepted after the scanned file was restored"
    );
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn lfs_pointer_size_boundary_distinguishes_pointers_from_large_assets() {
    let dir = tempdir().unwrap();
    for size in [1023_usize, 1024, 1025] {
        let mut content = git_lfs_pointer().as_bytes().to_vec();
        content.resize(size, b' ');
        fs::write(dir.path().join("asset.bin"), content).unwrap();
        let files = scan_dir(dir.path()).unwrap();
        for enabled in [false, true] {
            let result = ensure_no_unresolved_lfs_pointers(dir.path(), &files, enabled);
            if size <= 1024 {
                let error = result.expect_err("a pointer at the size limit must still be detected");
                let code = error
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<crate::output::CodedError>())
                    .unwrap();
                assert_eq!(
                    code.code,
                    if enabled {
                        "GIT_LFS_UNRESOLVED"
                    } else {
                        "GIT_LFS_REQUIRED"
                    }
                );
            } else {
                result.expect("a larger asset with a pointer-like prefix is not an LFS pointer");
            }
        }
    }
}

#[test]
fn lfs_detection_requires_the_version_oid_and_size_fields_together() {
    let dir = tempdir().unwrap();
    for (content, pointer) in [
        (git_lfs_pointer(), true),
        (
            "version https://git-lfs.github.com/spec/v1\nsize 10\n",
            false,
        ),
        (
            "version https://git-lfs.github.com/spec/v1\noid sha256:abc\n",
            false,
        ),
        ("oid sha256:abc\nsize 10\n", false),
        ("text\nsize 10\n", false),
    ] {
        let file = dir.path().join("asset.txt");
        fs::write(&file, content).unwrap();
        assert_eq!(
            is_git_lfs_pointer_file(&file).unwrap(),
            pointer,
            "{content:?}"
        );
        let files = scan_dir(dir.path()).unwrap();
        assert_eq!(
            ensure_no_unresolved_lfs_pointers(dir.path(), &files, false).is_err(),
            pointer,
            "{content:?}"
        );
    }
}

#[test]
fn artifact_scan_reports_pruning_and_large_files_on_stderr() {
    const CASE: &str = "NRZ_TEST_ARTIFACT_SCAN_REPORT";
    if let Ok(case) = std::env::var(CASE) {
        let mut files = match case.as_str() {
            "small" => vec![fe("index.html", 10, "aa")],
            "limit" => vec![fe("index.html", 26_214_400, "aa")],
            "large" => vec![fe("index.html", 26_214_401, "aa")],
            "pruned" => vec![
                fe("index.html", 10, "aa"),
                fe(".env", 2, "bb"),
                fe("onreza.toml", 3, "cc"),
            ],
            "ranked" => ["z.bin", "a.bin", "b.bin", "c.bin", "d.bin", "e.bin"]
                .into_iter()
                .map(|path| fe(path, 26_214_401, "aa"))
                .collect(),
            _ => panic!("unknown report case: {case}"),
        };
        if case == "ranked" {
            files.push(fe(".env", u64::MAX, "bb"));
        }
        let collection = prepare_artifact_files(
            &build_manifest::generate_static_manifest(),
            files,
            &make_detection("static-html", None),
            ArtifactRootScope::ProjectRoot,
            &RuntimeArtifactScan::All,
            true,
        );
        assert!(
            collection
                .deployable_entries()
                .iter()
                .all(|file| file.path != ".env")
        );
        return;
    }

    for case in ["small", "limit", "large", "pruned", "ranked"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "deploy::deploy_tests::scan_tests::artifact_scan_reports_pruning_and_large_files_on_stderr",
                "--nocapture",
            ])
            .env(CASE, case)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        let frames = stderr
            .lines()
            .filter_map(|line| line.strip_prefix('\u{1e}'))
            .map(|frame| serde_json::from_str::<serde_json::Value>(frame).unwrap())
            .collect::<Vec<_>>();
        let pruned = frames
            .iter()
            .find(|frame| frame["m"].as_str().unwrap().starts_with("Pruned "));
        assert_eq!(
            pruned.is_some(),
            matches!(case, "pruned" | "ranked"),
            "{case}: {stderr}"
        );
        if case == "pruned" {
            assert!(pruned.unwrap()["m"].as_str().unwrap().contains("2/3"));
        }
        let warning = frames.iter().find(|frame| frame["l"] == "warn");
        assert_eq!(
            warning.is_some(),
            matches!(case, "large" | "ranked"),
            "{case}: {stderr}"
        );
        if let Some(warning) = warning {
            assert_eq!(warning["p"], "deploy");
            assert_eq!(warning["s"], "user");
            let message = warning["m"].as_str().unwrap();
            if case == "ranked" {
                for name in ["a.bin", "b.bin", "c.bin", "d.bin", "e.bin"] {
                    assert!(message.contains(name), "{message}");
                }
                assert!(!message.contains("z.bin"), "{message}");
                assert!(!message.contains(".env"), "{message}");
                assert!(message.find("a.bin").unwrap() < message.find("b.bin").unwrap());
            } else {
                assert!(message.contains("index.html"), "{message}");
            }
        }
    }
}

#[test]
fn scan_files_computes_sha256_from_original_content() {
    let dir = tempdir().unwrap();
    let content = "hello world";
    fs::write(dir.path().join("file.txt"), content).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    let hash = files[0].content_hash.as_str();
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Known SHA-256 of "hello world" — guards against accidental hashing of
    // anything other than SOURCE_BUNDLE_V1 identity bytes.
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn scan_files_handles_file_larger_than_chunk() {
    // Stream-hash path covers a file that needs multiple read() calls — guards
    // against an off-by-one that would only hash the first chunk.
    let dir = tempdir().unwrap();
    let content = vec![0xABu8; 200 * 1024]; // 200 KiB > 64 KiB SCAN_HASH_CHUNK_BYTES
    fs::write(dir.path().join("big.bin"), &content).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
    let expected = sha256_hex(&content);
    assert_eq!(files[0].content_hash, expected);
}

#[test]
fn scan_files_sha256_deterministic_across_calls() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "same content").unwrap();

    let files1 = scan_dir(dir.path()).unwrap();
    let files2 = scan_dir(dir.path()).unwrap();

    assert_eq!(files1[0].content_hash, files2[0].content_hash);
}

#[test]
fn scan_files_sha256_differs_for_different_content() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "content A").unwrap();
    fs::write(dir.path().join("b.txt"), "content B").unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_ne!(files[0].content_hash, files[1].content_hash);
}

#[test]
fn scan_files_empty_directory() {
    let dir = tempdir().unwrap();
    let files = scan_dir(dir.path()).unwrap();
    assert!(files.is_empty());
}

#[test]
fn selected_scan_skips_missing_roots_and_propagates_invalid_ancestors() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/server.js"), "SERVER").unwrap();
    fs::write(dir.path().join("block"), "REGULAR_FILE").unwrap();
    let owner = nrz_runtime_artifact::ArtifactRoot::open(dir.path()).unwrap();
    for (optional, rejected) in [("missing", false), ("block/child", true)] {
        let scan = RuntimeArtifactScan::Selected {
            roots: ["dist", optional]
                .into_iter()
                .map(|path| crate::artifact::RuntimeArtifactScanRoot {
                    path: path.to_owned(),
                    kind: crate::artifact::RuntimeArtifactScanRootKind::BuildOutput,
                })
                .collect(),
            symlink_roots: vec![],
        };
        let result = scan_runtime_artifact_rooted(&owner, &scan);
        if rejected {
            assert!(result.is_err(), "invalid ancestor was silently skipped");
        } else {
            assert_eq!(
                result
                    .unwrap()
                    .iter()
                    .map(|file| file.path.as_str())
                    .collect::<Vec<_>>(),
                ["dist/server.js"]
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn scan_rejects_symlinks_resolving_to_the_artifact_root() {
    for target in [".", "child/.."] {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("child")).unwrap();
        std::os::unix::fs::symlink(target, dir.path().join("root-alias")).unwrap();
        let error = scan_dir(dir.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unsafe SOURCE_BUNDLE_V1 symlink target"),
            "{error}"
        );
    }
}

#[test]
fn scan_files_sorted_alphabetically() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let files = scan_dir(dir.path()).unwrap();

    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "m.txt");
    assert_eq!(files[2].path, "z.txt");
}

#[test]
fn scan_files_allows_double_dots_in_filename() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file..backup.js"), "x").unwrap();

    let files = scan_dir(dir.path()).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "file..backup.js");
}

#[cfg(unix)]
#[test]
fn scan_files_preserves_safe_relative_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink("real.txt", dir.path().join("link.txt")).unwrap();

    let files = scan_dir(dir.path()).unwrap();

    let link = files.iter().find(|file| file.path == "link.txt").unwrap();
    assert_eq!(link.size, 0);
    assert_eq!(link.content_hash, sha256_hex(b"real.txt"));
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_absolute_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();
    assert!(err.to_string().contains("absolute target"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_overlong_symlink_targets() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    let at_limit = format!("{}real.txt", "./".repeat(252));
    assert_eq!(at_limit.len(), 512);
    std::os::unix::fs::symlink(&at_limit, dir.path().join("valid.txt")).unwrap();
    let files = scan_dir(dir.path()).expect("a symlink target at the character limit is valid");
    let link = files.iter().find(|file| file.path == "valid.txt").unwrap();
    assert_eq!(link.content_hash, sha256_hex(at_limit.as_bytes()));
    assert_eq!(link.symlink_resolved_path.as_deref(), Some("real.txt"));
    let target = "a".repeat(source_bundle_v1::SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS + 1);
    std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("target too long"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_backslash_only_symlink_targets_before_normalizing_paths() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real\\target"), "real").unwrap();
    std::os::unix::fs::symlink("real\\target", dir.path().join("alias")).unwrap();
    let error = scan_dir(dir.path())
        .expect_err("a physical Unix target is not necessarily a portable archive target");
    assert!(
        error
            .to_string()
            .contains("unsafe SOURCE_BUNDLE_V1 symlink target"),
        "{error}"
    );
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_broken_symlinks() {
    let dir = tempdir().unwrap();
    std::os::unix::fs::symlink("missing.txt", dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();
    assert!(err.to_string().contains("broken symlink"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_symlinks_that_escape_output() {
    let root = tempdir().unwrap();
    let dir = root.path().join("app");
    let outside = root.path().join("outside");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink("../outside/real.txt", dir.join("link.txt")).unwrap();

    let err = scan_dir(&dir).unwrap_err();
    assert!(err.to_string().contains("escapes build output"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_symlinks_that_traverse_through_regular_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink("real.txt/", dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("broken symlink"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_files_rejects_symlink_parent_traversal_after_regular_file() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("dir")).unwrap();
    fs::write(dir.path().join("dir/file"), "file").unwrap();
    fs::write(dir.path().join("dir/other"), "other").unwrap();
    std::os::unix::fs::symlink("dir/file/../other", dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("broken symlink"), "{err}");
}

#[cfg(unix)]
#[test]
fn scan_and_bundle_preserve_physical_parent_paths_through_deep_directory_aliases() {
    let dir = tempdir().unwrap();
    for (path, body) in [
        ("physical/deeper/child/witness.txt", "directory witness"),
        ("physical/real.txt", "actual target"),
    ] {
        let file = dir.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, body).unwrap();
    }
    let raw_target = "directory-alias/../../real.txt";
    for (name, target) in [
        ("directory-alias", "physical/deeper/child"),
        ("alias", raw_target),
    ] {
        std::os::unix::fs::symlink(target, dir.path().join(name)).unwrap();
    }
    let files = scan_dir(dir.path()).unwrap();
    let alias = files.iter().find(|file| file.path == "alias").unwrap();
    assert_eq!(
        alias.symlink_resolved_path.as_deref(),
        Some("physical/real.txt")
    );
    assert_eq!(alias.content_hash, hash::sha256_hex(raw_target.as_bytes()));
    let manifest = serde_json::from_value(serde_json::json!({"version":1,"layers":[{"name":"static","target":"STATIC","directory":"."}],"routes":[]})).unwrap();
    let plan =
        crate::artifact::source_bundle_v1::build_source_bundle_plan(dir.path(), &manifest, &files)
            .unwrap();
    let unpacked = crate::test_support::unpack_source_bundle(&plan);
    assert_eq!(
        fs::read(unpacked.path().join("alias")).unwrap(),
        b"actual target"
    );
}

#[cfg(unix)]
#[test]
fn runtime_scan_and_bundle_keep_the_owned_root_after_path_replacement() {
    let directory = tempdir().unwrap();
    let root_path = directory.path().join("output");
    fs::create_dir(&root_path).unwrap();
    fs::write(root_path.join("file.txt"), b"original").unwrap();
    let root = nrz_runtime_artifact::ArtifactRoot::open(&root_path).unwrap();
    fs::rename(&root_path, directory.path().join("original-output")).unwrap();
    fs::create_dir(&root_path).unwrap();
    fs::write(root_path.join("file.txt"), b"foreign").unwrap();
    let manifest = build_manifest::generate_static_manifest();
    let files = prepare_artifact_files(
        &manifest,
        scan_runtime_artifact_rooted(&root, &RuntimeArtifactScan::All).unwrap(),
        &make_detection("static-html", None),
        ArtifactRootScope::BuildOutput,
        &RuntimeArtifactScan::All,
        true,
    )
    .deployable_entries();
    assert_eq!(files[0].content_hash, sha256_hex(b"original"));
    let bundle = source_bundle_v1::build_source_bundle_plan_with_root(
        &root,
        &manifest,
        &files,
        &RuntimeArtifactScan::All,
        source_bundle_v1::RuntimeDependencyPackaging::Embedded,
        None,
    )
    .unwrap();
    let restored = crate::test_support::unpack_source_bundle(&bundle);
    assert_eq!(
        fs::read(restored.path().join("file.txt")).unwrap(),
        b"original"
    );
}
