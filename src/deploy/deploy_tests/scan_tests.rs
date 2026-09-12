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
    let target = "a".repeat(source_bundle_v1::SOURCE_BUNDLE_LINK_TARGET_MAX_CHARACTERS + 1);
    std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).unwrap();

    let err = scan_dir(dir.path()).unwrap_err();

    assert!(err.to_string().contains("target too long"), "{err}");
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
