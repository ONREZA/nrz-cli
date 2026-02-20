use std::collections::HashSet;
use std::fs;
use std::io::Read;

use tempfile::tempdir;

use super::bundle;

#[test]
fn produces_valid_tar_zst() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.js"), "console.log('ok')").unwrap();

    let (bytes, _sha) = bundle::create_bundle(dir.path()).unwrap();

    // Decompress and read tar entries
    let decoder = zstd::Decoder::new(bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let mut found = HashSet::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        found.insert(path);
    }

    assert!(found.contains("index.html"));
    assert!(found.contains("assets/app.js"));
    assert_eq!(found.len(), 2);
}

#[test]
fn sha256_is_64_hex_chars() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "content").unwrap();

    let (_bytes, sha) = bundle::create_bundle(dir.path()).unwrap();

    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    // Must be lowercase
    assert_eq!(sha, sha.to_lowercase());
}

#[test]
fn sha256_deterministic() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    fs::write(dir.path().join("b.txt"), "bbb").unwrap();

    let (_bytes1, sha1) = bundle::create_bundle(dir.path()).unwrap();
    let (_bytes2, sha2) = bundle::create_bundle(dir.path()).unwrap();

    assert_eq!(sha1, sha2);
}

#[test]
fn relative_paths_no_leading_slash() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    fs::write(dir.path().join("server/entry.mjs"), "export default {}").unwrap();

    let (bytes, _sha) = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        assert!(
            !path.starts_with('/'),
            "path should not start with /: {path}"
        );
    }
}

#[test]
fn preserves_file_content() {
    let dir = tempdir().unwrap();
    let content = "hello world 12345";
    fs::write(dir.path().join("data.txt"), content).unwrap();

    let (bytes, _sha) = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let mut entry = archive.entries().unwrap().next().unwrap().unwrap();
    let mut buf = String::new();
    entry.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, content);
}

#[test]
fn empty_dir_returns_non_empty_archive() {
    // An empty dir produces a valid tar.zst (with just the tar end-of-archive marker)
    let dir = tempdir().unwrap();
    let result = bundle::create_bundle(dir.path());
    // Empty archive is still valid — tar end-of-archive blocks exist
    assert!(result.is_ok());
}

#[test]
fn nonexistent_dir_returns_error() {
    let result = bundle::create_bundle(std::path::Path::new("/tmp/nrz-test-nonexistent-dir-12345"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("failed to read directory"),
        "unexpected error: {msg}"
    );
}

#[test]
fn deeply_nested_directory() {
    let dir = tempdir().unwrap();
    let deep = dir.path().join("a/b/c/d/e");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("deep.js"), "export default 1").unwrap();
    fs::write(dir.path().join("root.txt"), "root").unwrap();

    let (bytes, _sha) = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    let paths: Vec<String> = archive
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
        .collect();

    assert!(paths.contains(&"a/b/c/d/e/deep.js".to_string()));
    assert!(paths.contains(&"root.txt".to_string()));
    assert_eq!(paths.len(), 2);
}

#[test]
fn entries_sorted_for_determinism() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let (bytes, _sha) = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    let paths: Vec<String> = archive
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
        .collect();

    assert_eq!(paths, vec!["a.txt", "m.txt", "z.txt"]);
}

#[cfg(unix)]
#[test]
fn skips_symlinks() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();

    let (bytes, _sha) = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let entries: Vec<_> = archive
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
        .collect();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], "real.txt");
}
