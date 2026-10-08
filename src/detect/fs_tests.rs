use super::fs::*;

#[test]
fn virtual_fs_normalized_paths_match_local_detection_inputs() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join("src")).unwrap();
    std::fs::write(project.path().join("package.json"), "{}").unwrap();
    std::fs::write(project.path().join("src/server.js"), "server").unwrap();
    let local = LocalFs::new(project.path());

    for (package_path, server_path, directory_path) in [
        ("package.json", "src/server.js", "src/"),
        (".\\package.json", ".\\src\\server.js", "src\\"),
        ("././package.json", "src/./server.js", "./src/"),
        ("package.json", "src//server.js", "src//"),
    ] {
        let input = serde_json::json!({
            "tree": [package_path, directory_path, server_path],
            "files": {package_path: "{}", server_path: "server"}
        });
        let virtual_fs = VirtualFs::from_json(&input.to_string()).unwrap();
        assert_eq!(
            virtual_fs.needed_content_files().unwrap(),
            ["package.json", "src/server.js"],
            "{input}"
        );
        for path in [
            "package.json",
            package_path,
            "src/server.js",
            server_path,
            "src",
            directory_path,
        ] {
            assert_eq!(
                virtual_fs.exists(path),
                local.exists(path),
                "{path}: {input}"
            );
            assert_eq!(
                virtual_fs.is_dir(path),
                local.is_dir(path),
                "{path}: {input}"
            );
            assert_eq!(
                virtual_fs.is_file(path),
                local.is_file(path),
                "{path}: {input}"
            );
            assert_eq!(
                virtual_fs.read_file(path),
                local.read_file(path),
                "{path}: {input}"
            );
            assert_eq!(
                virtual_fs.list_dir(path),
                local.list_dir(path),
                "{path}: {input}"
            );
        }
    }
}

#[test]
fn virtual_fs_directory_queries_match_local_filesystem() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("package.json"), "{}").unwrap();
    let local = LocalFs::new(project.path());
    let virtual_fs = VirtualFs::from_json(r#"{"files":{"package.json":"{}"}}"#).unwrap();
    for path in [
        "package.json/",
        "package.json\\",
        "package.json/.",
        "package.json\\.",
        ".",
        "./",
    ] {
        assert_eq!(virtual_fs.exists(path), local.exists(path), "{path}");
        assert_eq!(virtual_fs.is_dir(path), local.is_dir(path), "{path}");
        assert_eq!(virtual_fs.read_file(path), local.read_file(path), "{path}");
    }
}

#[test]
fn virtual_fs_rejects_colliding_file_content_aliases() {
    for alias in ["./package.json", ".\\package.json", "././package.json"] {
        let input = serde_json::json!({"files": {"package.json": "first", alias: "second"}});
        assert!(VirtualFs::from_json(&input.to_string()).is_err(), "{input}");
    }
    let input =
        serde_json::json!({"files": {"src/server.js": "first", "src//server.js": "second"}});
    assert!(VirtualFs::from_json(&input.to_string()).is_err(), "{input}");
}

#[test]
fn virtual_fs_rejects_file_contents_at_directory_paths() {
    for input in [
        serde_json::json!({"files": {"": "root"}}),
        serde_json::json!({"files": {".": "root"}}),
        serde_json::json!({"files": {"src/": "directory"}}),
        serde_json::json!({"files": {"src\\": "directory"}}),
        serde_json::json!({"files": {"src/.": "directory"}}),
        serde_json::json!({"files": {"src\\.": "directory"}}),
        serde_json::json!({"tree": ["src/"], "files": {"src": "file"}}),
        serde_json::json!({"tree": ["src/."], "files": {"src": "file"}}),
        serde_json::json!({"tree": ["src\\"], "files": {"src": "file"}}),
        serde_json::json!({"tree": ["src/child.js"], "files": {"src": "file"}}),
        serde_json::json!({"files": {"src": "file", "src/child.js": "child"}}),
    ] {
        assert!(VirtualFs::from_json(&input.to_string()).is_err(), "{input}");
    }
}

#[test]
fn detection_filesystems_keep_nonrelative_paths_inaccessible() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("package.json"), "{}").unwrap();
    let local = LocalFs::new(project.path());
    let virtual_fs = VirtualFs::from_json(r#"{"files":{"package.json":"{}"}}"#).unwrap();
    for path in [
        "/",
        "//",
        "../package.json",
        "src/../package.json",
        "/package.json",
        "//package.json",
        "C:/package.json",
        "C:\\package.json",
        "./C:/package.json",
        ".\\C:\\package.json",
        "././/C:/package.json",
        "\\\\server\\package.json",
    ] {
        let input = serde_json::json!({"tree": [path]});
        assert!(VirtualFs::from_json(&input.to_string()).is_err(), "{path}");
        for fs in [&local as &dyn Fs, &virtual_fs as &dyn Fs] {
            assert!(!fs.exists(path), "{path}");
            assert!(!fs.is_dir(path), "{path}");
            assert!(fs.read_file(path).is_none(), "{path}");
            assert!(fs.list_dir(path).is_empty(), "{path}");
        }
    }
}

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
fn needed_contents_follow_qualified_go_package_discovery() {
    let tree = [
        "cmd/server/main.go",
        "server.go",
        "main_linux_amd64.go",
        "go.mod",
        "cmd/.tool/main.go",
        "cmd/server/part_linux.go",
        "package.json",
        "main_windows.go",
        "cmd/server/main_arm64.go",
        "main_test.go",
        "_ignored.go",
        ".ignored.go",
        "lib/main.go",
        "cmd/server/deep/main.go",
        "cmd/server/directory.go/",
        "README.md",
    ];
    let fs =
        VirtualFs::from_json(&serde_json::json!({"tree":tree,"files":{}}).to_string()).unwrap();
    assert_eq!(
        fs.needed_content_files().unwrap(),
        [
            "cmd/.tool/main.go",
            "cmd/server/main.go",
            "cmd/server/part_linux.go",
            "go.mod",
            "main_linux_amd64.go",
            "package.json",
            "server.go",
        ]
    );
}

#[test]
fn needed_contents_reject_candidate_overflow_without_truncation() {
    let mut tree = (0..MAX_DETECTION_CONTENT_FILES)
        .map(|index| format!("cmd/server_{index}/main.go"))
        .collect::<Vec<_>>();
    let fs =
        VirtualFs::from_json(&serde_json::json!({"tree":tree,"files":{}}).to_string()).unwrap();
    assert_eq!(
        fs.needed_content_files().unwrap().len(),
        MAX_DETECTION_CONTENT_FILES
    );
    tree.push("go.mod".into());
    let fs =
        VirtualFs::from_json(&serde_json::json!({"tree":tree,"files":{}}).to_string()).unwrap();
    assert!(
        fs.needed_content_files()
            .unwrap_err()
            .to_string()
            .contains("max 256")
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
