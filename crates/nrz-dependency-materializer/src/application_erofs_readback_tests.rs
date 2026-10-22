use super::*;
use serde_json::json;
use std::{
    ffi::CString,
    os::unix::fs::{PermissionsExt, symlink},
};

fn regular(path: &str, bytes: &[u8]) -> FileEntry {
    serde_json::from_value(json!({"entryType":"file", "path":path, "role":"static",
        "size":bytes.len(), "blobDigest":{"algorithm":"BLAKE3", "value":blake3::hash(bytes).to_hex().as_str()},
        "executable":false})).unwrap()
}

fn link(path: &str, target: &str) -> FileEntry {
    serde_json::from_value(json!({"entryType":"symlink", "path":path, "role":"static",
        "size":0, "linkTarget":target}))
    .unwrap()
}

fn normalize(path: &Path) {
    let metadata = fs::symlink_metadata(path).unwrap();
    if metadata.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            normalize(&entry.unwrap().path());
        }
    }
    if !metadata.is_symlink() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o775)).unwrap();
    }
    set_time(path, 0, 0);
}

fn set_time(path: &Path, seconds: libc::time_t, nanos: libc::c_long) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let times = [libc::timespec {
        tv_sec: seconds,
        tv_nsec: nanos,
    }; 2];
    // SAFETY: the path and two timestamps stay live; no symlink target is followed.
    assert_eq!(
        unsafe {
            libc::utimensat(
                libc::AT_FDCWD,
                path.as_ptr(),
                times.as_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        },
        0
    );
}

fn verify(root: &Path, expected: &[FileEntry]) -> Result<(), ReadbackError> {
    verify_extracted_tree(
        root,
        &expected.iter().collect::<Vec<_>>(),
        100,
        1024,
        1024 * 1024,
    )
}

#[test]
fn verifies_nested_empty_large_files_and_cross_role_raw_symlink() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("assets/nested")).unwrap();
    let bytes = vec![b'x'; BUFFER_BYTES * 2 + 17];
    fs::write(root.path().join("assets/nested/é"), &bytes).unwrap();
    fs::write(root.path().join("empty"), []).unwrap();
    symlink("../compute/entry", root.path().join("raw-link")).unwrap();
    normalize(root.path());
    verify(
        root.path(),
        &[
            regular("assets/nested/é", &bytes),
            regular("empty", b""),
            link("raw-link", "../compute/entry"),
        ],
    )
    .unwrap();
}

#[test]
fn rejects_missing_extra_and_wrong_types_without_following_symlinks() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file"), b"ok").unwrap();
    normalize(root.path());
    let expected = [regular("file", b"ok")];
    verify(root.path(), &expected).unwrap();
    fs::create_dir(root.path().join("extra")).unwrap();
    normalize(root.path());
    assert!(verify(root.path(), &expected).is_err());
    fs::remove_dir(root.path().join("extra")).unwrap();
    fs::remove_file(root.path().join("file")).unwrap();
    normalize(root.path());
    assert!(verify(root.path(), &expected).is_err());
    symlink("/dev/zero", root.path().join("file")).unwrap();
    normalize(root.path());
    assert!(verify(root.path(), &expected).is_err());
    fs::remove_file(root.path().join("file")).unwrap();
    fs::create_dir(root.path().join("file")).unwrap();
    normalize(root.path());
    assert!(verify(root.path(), &expected).is_err());
}

#[test]
fn rejects_content_link_modes_and_nanosecond_timestamp_changes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("file"), b"ok").unwrap();
    symlink("file", root.path().join("link")).unwrap();
    let expected = [regular("file", b"ok"), link("link", "file")];
    normalize(root.path());
    verify(root.path(), &expected).unwrap();
    fs::write(root.path().join("file"), b"no").unwrap();
    normalize(root.path());
    assert!(verify(root.path(), &expected).is_err());
    fs::write(root.path().join("file"), b"ok").unwrap();
    normalize(root.path());
    fs::set_permissions(root.path().join("file"), fs::Permissions::from_mode(0o4775)).unwrap();
    assert!(verify(root.path(), &expected).is_err());
    normalize(root.path());
    set_time(&root.path().join("file"), 0, 1);
    assert!(verify(root.path(), &expected).is_err());
    normalize(root.path());
    set_time(root.path(), 1, 0);
    assert!(verify(root.path(), &expected).is_err());
    fs::remove_file(root.path().join("link")).unwrap();
    symlink("else", root.path().join("link")).unwrap();
    normalize(root.path());
    assert!(verify(root.path(), &expected).is_err());
}

#[test]
fn enforces_nodes_path_and_working_memory_before_traversal() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("a/b")).unwrap();
    fs::write(root.path().join("a/b/file"), b"ok").unwrap();
    normalize(root.path());
    let file = regular("a/b/file", b"ok");
    let expected = [&file];
    assert!(matches!(
        verify_extracted_tree(root.path(), &expected, 3, 1024, 1024 * 1024),
        Err(ReadbackError::Limit("nodes"))
    ));
    assert!(matches!(
        verify_extracted_tree(root.path(), &expected, 100, 3, 1024 * 1024),
        Err(ReadbackError::Limit("path bytes"))
    ));
    assert!(matches!(
        verify_extracted_tree(root.path(), &expected, 100, 1024, 1),
        Err(ReadbackError::Limit("working memory"))
    ));
    verify_extracted_tree(root.path(), &expected, 4, 1024, 1024 * 1024).unwrap();
}
