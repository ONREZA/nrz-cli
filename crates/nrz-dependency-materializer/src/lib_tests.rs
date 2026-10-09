use super::*;
use std::os::unix::fs::symlink;
use std::time::{Duration, SystemTime};

const LIMITS: DependencyTreeLimits = DependencyTreeLimits {
    max_files: 100,
    max_expanded_bytes: 1024 * 1024,
    max_path_bytes: 512,
    max_symlinks: 10,
};

#[test]
fn logical_tree_identity_ignores_mtime_and_non_executable_mode_noise() {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("node_modules/example");
    fs::create_dir_all(&package).unwrap();
    let file = package.join("index.js");
    fs::write(&file, "export const value = 1;\n").unwrap();
    symlink("example/index.js", root.path().join("node_modules/link.js")).unwrap();

    assert_eq!(
        fs::read(root.path().join("node_modules/link.js")).unwrap(),
        b"export const value = 1;\n"
    );
    normalize_tree(root.path(), LIMITS).unwrap();
    let first = inspect_dependency_tree(root.path(), LIMITS).unwrap();
    fs::set_permissions(&file, Permissions::from_mode(0o600)).unwrap();
    let opened = File::options().write(true).open(&file).unwrap();
    opened
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(9_999)),
        )
        .unwrap();
    let second = inspect_dependency_tree(root.path(), LIMITS).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.expanded_file_count, 2);
    assert_eq!(first.regular_file_count, 1);
    assert_eq!(first.symlink_count, 1);
}

#[test]
fn logical_tree_identity_includes_executable_semantics() {
    let root = tempfile::tempdir().unwrap();
    let helper = root.path().join("helper");
    fs::write(&helper, "#!/bin/sh\n").unwrap();
    let regular = inspect_dependency_tree(root.path(), LIMITS).unwrap();

    fs::set_permissions(&helper, Permissions::from_mode(0o755)).unwrap();
    let executable = inspect_dependency_tree(root.path(), LIMITS).unwrap();

    assert_ne!(regular.logical_tree_digest, executable.logical_tree_digest);
}

#[test]
fn unsafe_symlink_and_special_entry_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    symlink("../../outside", root.path().join("escape")).unwrap();
    assert!(matches!(
        inspect_dependency_tree(root.path(), LIMITS),
        Err(DependencyMaterializerError::UnsafeSymlink { .. })
    ));

    fs::remove_file(root.path().join("escape")).unwrap();
    let fifo = root.path().join("pipe");
    let fifo_path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: fifo_path is a live NUL-terminated path and mode is passed by value.
    assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
    assert!(matches!(
        inspect_dependency_tree(root.path(), LIMITS),
        Err(DependencyMaterializerError::UnsupportedEntry(_))
    ));
}

#[test]
fn manifest_owned_runtime_mount_scope_allows_a_cross_tree_symlink() {
    let layout = tempfile::tempdir().unwrap();
    let root = layout.path().join(".next/node_modules");
    let sibling = layout.path().join("node_modules");
    fs::create_dir_all(sibling.join("@prisma")).unwrap();
    fs::write(sibling.join("@prisma/client"), b"real").unwrap();
    let prisma = root.join("@prisma");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir(&prisma).unwrap();
    symlink(
        "../../../node_modules/@prisma/client",
        prisma.join("client-generated"),
    )
    .unwrap();
    let trees = [
        dependency_tree(&root, "/output/.next/node_modules", "server"),
        dependency_tree(&sibling, "/output/node_modules", "server"),
    ];
    assert_eq!(fs::read(prisma.join("client-generated")).unwrap(), b"real");

    let tree = inspect_dependency_tree_with_scope(
        &root,
        LIMITS,
        DependencySymlinkScope::RuntimeMounts {
            mount_point: "/output/.next/node_modules",
            trees: &trees,
        },
        false,
    )
    .unwrap();

    assert_eq!(tree.summary.symlink_count, 1);
    assert!(tree.uses_runtime_mount_symlink);
    assert_eq!(
        canonicalization_policy_digest_for(tree.uses_runtime_mount_symlink),
        sha256_prefixed(DEPENDENCY_EROFS_CANONICALIZATION_POLICY_V2.as_bytes())
    );
    assert!(matches!(
        inspect_dependency_tree(&root, LIMITS),
        Err(DependencyMaterializerError::UnsafeSymlink { .. })
    ));
}

#[test]
fn expanded_limits_fail_before_image_generation() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("large"), vec![0u8; 32]).unwrap();
    let limits = DependencyTreeLimits {
        max_expanded_bytes: 16,
        ..LIMITS
    };

    assert!(matches!(
        inspect_dependency_tree(root.path(), limits),
        Err(DependencyMaterializerError::Limit {
            limit_name: "max_expanded_bytes",
            ..
        })
    ));
}

#[test]
fn policy_digest_is_stable_and_prefixed() {
    let digest = canonicalization_policy_digest();
    assert!(digest.starts_with("sha256:"));
    assert_eq!(digest.len(), "sha256:".len() + 64);
    assert_eq!(
        digest,
        canonicalization_policy_digest_for(false),
        "ordinary dependency trees must retain the V1 immutable identity"
    );
    assert_ne!(digest, canonicalization_policy_digest_for(true));
}

#[test]
fn closed_tree_follows_aliases_before_parent_components() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("pkg/deeper/child")).unwrap();
    fs::write(root.path().join("pkg/real"), b"real").unwrap();
    symlink("pkg/deeper/child", root.path().join("alias")).unwrap();
    symlink("alias/../../real", root.path().join("link")).unwrap();
    assert_eq!(fs::read(root.path().join("link")).unwrap(), b"real");
    let summary = inspect_dependency_tree(root.path(), LIMITS).unwrap();
    assert_eq!(summary.symlink_count, 2);
}

#[test]
fn closed_tree_rejects_unresolvable_links_even_when_their_lexical_target_exists() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("real"), b"real").unwrap();
    symlink("missing/../real", root.path().join("link")).unwrap();
    assert!(fs::read(root.path().join("link")).is_err());
    assert!(matches!(
        inspect_dependency_tree(root.path(), LIMITS),
        Err(DependencyMaterializerError::UnsafeSymlink { .. })
    ));
}

#[test]
fn closed_tree_preserves_empty_directories_and_root_directory_aliases() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("empty")).unwrap();
    fs::write(root.path().join("real"), b"real").unwrap();
    symlink("empty/../real", root.path().join("link")).unwrap();
    symlink(".", root.path().join("root")).unwrap();
    fs::write(root.path().join("back\\slash"), b"unix-name").unwrap();
    symlink("back\\slash", root.path().join("unix-link")).unwrap();
    assert_eq!(fs::read(root.path().join("link")).unwrap(), b"real");
    assert_eq!(fs::read(root.path().join("root/real")).unwrap(), b"real");
    assert_eq!(
        fs::read(root.path().join("unix-link")).unwrap(),
        b"unix-name"
    );
    let summary = inspect_dependency_tree(root.path(), LIMITS).unwrap();
    assert_eq!(summary.symlink_count, 3);
}

fn dependency_tree(path: &Path, mount_point: &str, layer_name: &str) -> DependencySourceTree {
    DependencySourceTree {
        source_root: mount_point.trim_start_matches("/output/").into(),
        layer_name: layer_name.into(),
        mount_point: mount_point.into(),
        path: path.to_path_buf(),
        file_count: 0,
        logical_bytes: 0,
    }
}
