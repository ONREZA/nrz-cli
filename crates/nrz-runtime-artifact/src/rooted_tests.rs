use std::{fs, path::Path};

use super::ArtifactRoot;

#[test]
fn root_relative_reads_preserve_bytes_and_reject_nonregular_or_outside_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("root");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("data"), b"owned bytes").unwrap();
    fs::write(temp.path().join("outside"), b"outside bytes").unwrap();
    let root = ArtifactRoot::open(&path).unwrap();
    assert_eq!(root.read(Path::new("data")).unwrap(), b"owned bytes");
    assert!(root.file_type(Path::new("data"), false).unwrap().is_file());
    assert!(root.file_type(Path::new(""), true).unwrap().is_dir());
    assert_eq!(root.canonicalize(Path::new("")).unwrap(), Path::new(""));
    assert_eq!(root.read_dir(Path::new("")).unwrap(), [Path::new("data")]);
    assert!(root.read(Path::new("")).is_err());
    assert!(root.read(Path::new("../outside")).is_err());
    assert!(root.read(&path.join("data")).is_err());
}

#[cfg(unix)]
#[test]
fn acquired_root_survives_parent_path_rebinding() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let parent = temp.path().join("parent");
    let foreign = temp.path().join("foreign");
    fs::create_dir_all(parent.join("root")).unwrap();
    fs::create_dir_all(foreign.join("root")).unwrap();
    fs::write(parent.join("root/data"), b"original").unwrap();
    fs::write(foreign.join("root/data"), b"replacement").unwrap();
    let root = ArtifactRoot::open(&parent.join("root")).unwrap();
    let shared = root.clone();
    fs::rename(&parent, temp.path().join("retained")).unwrap();
    symlink(&foreign, &parent).unwrap();
    assert_eq!(fs::read(parent.join("root/data")).unwrap(), b"replacement");
    assert_eq!(shared.read(Path::new("data")).unwrap(), b"original");
    assert!(shared.file_type(Path::new("data"), true).unwrap().is_file());
    assert_eq!(
        shared.canonicalize(Path::new("data")).unwrap(),
        Path::new("data")
    );
    assert_eq!(shared.read_dir(Path::new("")).unwrap(), [Path::new("data")]);
}

#[cfg(unix)]
#[test]
fn physical_parent_traversal_and_raw_links_preserve_containment() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("root");
    fs::create_dir_all(path.join("deep/child")).unwrap();
    fs::write(path.join("data"), b"lexical").unwrap();
    fs::write(path.join("deep/data"), b"physical").unwrap();
    symlink("deep/child", path.join("alias")).unwrap();
    symlink("data", path.join("final-link")).unwrap();
    symlink(temp.path(), path.join("outside")).unwrap();
    fs::create_dir(path.join("parent")).unwrap();
    symlink("../data", path.join("parent/raw-link")).unwrap();
    symlink("foreign target", temp.path().join("raw-link")).unwrap();
    let root = ArtifactRoot::open(&path).unwrap();
    assert_eq!(
        root.read_link(Path::new("parent/raw-link")).unwrap(),
        Path::new("../data")
    );
    fs::rename(path.join("parent"), path.join("retained-parent")).unwrap();
    symlink(temp.path(), path.join("parent")).unwrap();
    assert_eq!(
        fs::read_link(path.join("parent/raw-link")).unwrap(),
        Path::new("foreign target")
    );
    assert!(root.read_link(Path::new("parent/raw-link")).is_err());
    assert_eq!(root.read(Path::new("alias/../data")).unwrap(), b"physical");
    assert_eq!(
        root.canonicalize(Path::new("alias/../data")).unwrap(),
        Path::new("deep/data")
    );
    assert!(
        root.file_type(Path::new("alias"), false)
            .unwrap()
            .is_symlink()
    );
    assert!(root.file_type(Path::new("alias"), true).unwrap().is_dir());
    assert_eq!(root.read_link(Path::new("outside")).unwrap(), temp.path());
    assert!(root.read(Path::new("outside/root/data")).is_err());
    assert!(
        root.canonicalize(Path::new("alias/../../../root/data"))
            .is_err()
    );
    assert!(root.open_file(Path::new("final-link")).is_err());
    assert!(root.read(Path::new("deep")).is_err());
}

#[cfg(unix)]
#[test]
fn special_file_inputs_fail_without_waiting_for_a_writer() {
    use std::os::unix::ffi::OsStrExt as _;
    let temp = tempfile::tempdir().unwrap();
    let fifo = std::ffi::CString::new(temp.path().join("fifo").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let root = ArtifactRoot::open(temp.path()).unwrap();
    assert_eq!(
        root.open_file(Path::new("fifo")).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
}
