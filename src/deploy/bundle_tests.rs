use std::collections::HashSet;
use std::fs;
use std::io::Read;
#[cfg(unix)]
use std::path::PathBuf;

use tempfile::tempdir;

use super::bundle;

#[test]
fn produces_valid_tar_zst() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("index.html"), "<h1>hi</h1>").unwrap();
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.js"), "console.log('ok')").unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();

    // Decompress and read tar entries
    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let mut found = HashSet::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        found.insert(path);
    }

    assert!(found.contains("index.html"));
    assert!(found.contains("assets/app.js"));
    assert!(
        found.contains("assets"),
        "directory entry should be emitted"
    );
    assert_eq!(found.len(), 3);
}

#[test]
fn sha256_is_64_hex_chars() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file.txt"), "content").unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();

    assert_eq!(stats.sha256_hex.len(), 64);
    assert!(stats.sha256_hex.chars().all(|c| c.is_ascii_hexdigit()));
    // Must be lowercase
    assert_eq!(stats.sha256_hex, stats.sha256_hex.to_lowercase());
}

#[test]
fn sha256_deterministic() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    fs::write(dir.path().join("b.txt"), "bbb").unwrap();

    let stats1 = bundle::create_bundle(dir.path()).unwrap();
    let stats2 = bundle::create_bundle(dir.path()).unwrap();

    assert_eq!(stats1.sha256_hex, stats2.sha256_hex);
}

#[test]
fn relative_paths_no_leading_slash() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("server")).unwrap();
    fs::write(dir.path().join("server/entry.mjs"), "export default {}").unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
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

    let stats = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
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
        msg.contains("failed to canonicalize"),
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

    let stats = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    let paths: Vec<String> = archive
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
        .collect();

    assert!(paths.contains(&"a/b/c/d/e/deep.js".to_string()));
    assert!(paths.contains(&"root.txt".to_string()));
    // Intermediate directory entries are emitted so empty dirs survive extraction.
    assert!(paths.contains(&"a".to_string()));
    assert!(paths.contains(&"a/b/c/d/e".to_string()));
    assert_eq!(paths.len(), 7);
}

#[test]
fn entries_sorted_for_determinism() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("z.txt"), "z").unwrap();
    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("m.txt"), "m").unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();

    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
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
fn preserves_relative_symlinks_inside_bundle() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    // Relative target pointing inside the bundle — mimics pnpm layout.
    std::os::unix::fs::symlink("real.txt", dir.path().join("link.txt")).unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();
    assert_eq!(stats.symlinks_preserved, 1);
    assert_eq!(stats.files, 1);

    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let mut found_link_target: Option<String> = None;
    let mut found_regular: bool = false;
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        match entry.header().entry_type() {
            tar::EntryType::Symlink => {
                assert_eq!(path, "link.txt");
                found_link_target = Some(
                    entry
                        .link_name()
                        .unwrap()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            tar::EntryType::Regular => {
                assert_eq!(path, "real.txt");
                found_regular = true;
            }
            other => panic!("unexpected entry type: {other:?}"),
        }
    }

    assert!(found_regular, "regular file should be in archive");
    assert_eq!(found_link_target.as_deref(), Some("real.txt"));
}

#[cfg(unix)]
#[test]
fn symlink_round_trip_resolves_after_extraction() {
    // Mirrors pnpm's .next/standalone layout: `node_modules/next` is a symlink
    // into `.pnpm/next-.../node_modules/next/`.
    let src = tempdir().unwrap();
    let pnpm_next = src
        .path()
        .join("node_modules/.pnpm/next@1.0.0/node_modules/next");
    fs::create_dir_all(&pnpm_next).unwrap();
    fs::write(pnpm_next.join("index.js"), "module.exports = 'ok'").unwrap();

    let nm = src.path().join("node_modules");
    std::os::unix::fs::symlink(".pnpm/next@1.0.0/node_modules/next", nm.join("next")).unwrap();

    let stats = bundle::create_bundle(src.path()).unwrap();

    // Extract into a fresh directory and verify the symlink points to real content.
    let out = tempdir().unwrap();
    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(out.path()).unwrap();

    let extracted_link = out.path().join("node_modules/next");
    assert!(
        extracted_link.is_symlink(),
        "tar extraction should preserve symlink, not materialize it as a directory"
    );

    let resolved = fs::read_to_string(extracted_link.join("index.js")).unwrap();
    assert_eq!(resolved, "module.exports = 'ok'");
}

#[cfg(unix)]
#[test]
fn rejects_absolute_symlink() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "real").unwrap();
    // Absolute target cannot survive extraction on the compute node — must fail loudly.
    std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("abs.link")).unwrap();

    let err = bundle::create_bundle(dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("absolute target"),
        "unexpected error message: {msg}"
    );
}

#[cfg(unix)]
#[test]
fn skips_symlink_escaping_bundle() {
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "host state").unwrap();

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("ok.txt"), "ok").unwrap();
    // Relative target that escapes the bundle root via `../`.
    let escape_target = PathBuf::from("..")
        .join(outside.path().file_name().unwrap())
        .join("secret.txt");
    std::os::unix::fs::symlink(escape_target, dir.path().join("escape.link")).unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();
    assert_eq!(stats.symlinks_skipped, 1);

    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);

    let paths: Vec<String> = archive
        .entries()
        .unwrap()
        .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
        .collect();

    assert_eq!(paths, vec!["ok.txt"]);
}

#[cfg(unix)]
#[test]
fn rejects_broken_symlink() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("present.txt"), "hi").unwrap();
    // Broken symlink in build output usually means a corrupt dependency install — must fail loudly.
    std::os::unix::fs::symlink("missing-target.txt", dir.path().join("broken.link")).unwrap();

    let err = bundle::create_bundle(dir.path()).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("broken symlink"),
        "unexpected error message: {msg}"
    );
}

#[cfg(unix)]
#[test]
fn sha256_deterministic_with_symlink() {
    // Guards against future changes that introduce nondeterminism through symlink handling
    // (e.g., using an absolute path from canonicalize() in the emitted target).
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("real.txt"), "content").unwrap();
    std::os::unix::fs::symlink("real.txt", dir.path().join("alias.txt")).unwrap();

    let stats1 = bundle::create_bundle(dir.path()).unwrap();
    let stats2 = bundle::create_bundle(dir.path()).unwrap();

    assert_eq!(stats1.sha256_hex, stats2.sha256_hex);
}

#[test]
fn preserves_empty_directories() {
    // Runtime code often expects dirs like `logs/`, `tmp/`, `data/` to exist
    // before writing to them; if the bundle drops empty dirs, the first write
    // on the compute node hits ENOENT with no obvious cause.
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("logs")).unwrap();
    fs::create_dir(dir.path().join("data")).unwrap();
    fs::write(dir.path().join("app.js"), "console.log('ok')").unwrap();

    let stats = bundle::create_bundle(dir.path()).unwrap();

    let out = tempdir().unwrap();
    let decoder = zstd::Decoder::new(stats.bytes.as_slice()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(out.path()).unwrap();

    assert!(
        out.path().join("logs").is_dir(),
        "empty logs/ should exist after extraction"
    );
    assert!(
        out.path().join("data").is_dir(),
        "empty data/ should exist after extraction"
    );
    assert!(out.path().join("app.js").is_file());
}
