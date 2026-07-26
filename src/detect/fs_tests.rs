use super::fs::*;

#[test]
fn virtual_fs_exists() {
    let json = r#"{"tree":["package.json","src/","src/app/"],"files":{"package.json":"{}"}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert!(vfs.exists("package.json"));
    assert!(vfs.exists("src"));
    assert!(vfs.exists("src/app"));
    assert!(!vfs.exists("nonexistent"));
}

#[test]
fn virtual_fs_is_dir() {
    let json = r#"{"tree":["package.json","src/","src/app/"],"files":{"package.json":"{}"}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert!(vfs.is_dir("src"));
    assert!(vfs.is_dir("src/app"));
    assert!(!vfs.is_dir("package.json"));
    assert!(!vfs.is_dir("nonexistent"));
}

#[test]
fn virtual_fs_read_file() {
    let json = r#"{"tree":["package.json"],"files":{"package.json":"{\"name\":\"test\"}"}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert_eq!(
        vfs.read_file("package.json"),
        Some("{\"name\":\"test\"}".to_string())
    );
    assert!(vfs.read_file("nonexistent").is_none());
}

#[test]
fn local_fs_skips_oversized_detection_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        vec![b'a'; MAX_DETECTION_FILE_CONTENT_BYTES + 1],
    )
    .unwrap();
    let fs = LocalFs::new(dir.path());

    assert!(fs.read_file("package.json").is_none());
}

#[test]
fn virtual_fs_list_dir() {
    let json =
        r#"{"tree":["package.json","src/","src/app/","src/index.ts","README.md"],"files":{}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    let root_entries = vfs.list_dir("");
    assert!(root_entries.contains(&"package.json".to_string()));
    assert!(root_entries.contains(&"src".to_string()));
    assert!(root_entries.contains(&"README.md".to_string()));

    let src_entries = vfs.list_dir("src");
    assert!(src_entries.contains(&"app".to_string()));
    assert!(src_entries.contains(&"index.ts".to_string()));
}

#[test]
fn virtual_fs_list_dir_empty() {
    let json = r#"{"tree":[],"files":{}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert!(vfs.list_dir("").is_empty());
}

#[test]
fn virtual_fs_from_json_minimal() {
    let json = r#"{"tree":[],"files":{}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert!(!vfs.exists("anything"));
}

#[test]
fn virtual_fs_implicit_parent_dirs() {
    let json = r#"{"tree":["app/api/route.ts"],"files":{}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert!(vfs.is_dir("app"));
    assert!(vfs.is_dir("app/api"));
    assert!(vfs.exists("app/api/route.ts"));
    assert!(!vfs.is_dir("app/api/route.ts"));
}

#[test]
fn virtual_fs_files_create_tree_entries() {
    let json =
        r#"{"tree":[],"files":{"next.config.js":"module.exports = { output: 'standalone' }"}}"#;
    let vfs = VirtualFs::from_json(json).unwrap();
    assert!(vfs.exists("next.config.js"));
    assert_eq!(
        vfs.read_file("next.config.js"),
        Some("module.exports = { output: 'standalone' }".to_string())
    );
}

#[test]
fn virtual_fs_bounds_file_content() {
    let oversized = "x".repeat(MAX_DETECTION_FILE_CONTENT_BYTES + 1);
    let json = serde_json::json!({
        "tree": [],
        "files": { "package.json": oversized }
    })
    .to_string();

    let error = VirtualFs::from_json(&json).unwrap_err();

    assert!(
        error.to_string().contains("file content exceeds"),
        "{error}"
    );
}

#[test]
fn virtual_fs_bounds_path_depth() {
    let path = (0..=MAX_DETECTION_PATH_DEPTH)
        .map(|_| "nested")
        .collect::<Vec<_>>()
        .join("/");
    let json = serde_json::json!({ "tree": [path], "files": {} }).to_string();

    let error = VirtualFs::from_json(&json).unwrap_err();

    assert!(
        error.to_string().contains("path exceeds") && error.to_string().contains("components"),
        "{error}"
    );
}

#[test]
fn virtual_fs_rejects_parent_paths() {
    let json = r#"{"tree":["../package.json"],"files":{}}"#;

    let error = VirtualFs::from_json(json).unwrap_err();

    assert!(error.to_string().contains("must be relative"), "{error}");
}

#[test]
fn local_fs_basic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("test.txt"), "hello").unwrap();
    std::fs::create_dir_all(dir.path().join("subdir")).unwrap();

    let fs = LocalFs::new(dir.path());
    assert!(fs.exists("test.txt"));
    assert!(fs.exists("subdir"));
    assert!(fs.is_dir("subdir"));
    assert!(!fs.is_dir("test.txt"));
    assert_eq!(fs.read_file("test.txt"), Some("hello".to_string()));
    assert!(fs.read_file("nonexistent").is_none());

    let entries = fs.list_dir("");
    assert!(entries.contains(&"test.txt".to_string()));
    assert!(entries.contains(&"subdir".to_string()));
}

#[test]
fn local_fs_does_not_resolve_parent_paths() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(parent.path().join("secret.txt"), "secret").unwrap();

    let fs = LocalFs::new(&root);
    assert!(!fs.exists("../secret.txt"));
    assert!(fs.read_file("../secret.txt").is_none());
}

#[cfg(unix)]
#[test]
fn local_fs_does_not_follow_symlinks_outside_root() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();

    let fs = LocalFs::new(root.path());
    assert!(!fs.exists("linked/secret.txt"));
    assert!(fs.read_file("linked/secret.txt").is_none());
    assert!(fs.list_dir("linked").is_empty());
}
