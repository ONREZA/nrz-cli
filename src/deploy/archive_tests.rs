use std::io::Read;

use flate2::read::GzDecoder;

use super::archive::create_tar_gz;

#[test]
fn creates_valid_tar_gz() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("hello.txt"), "world").unwrap();
    std::fs::create_dir(tmp.path().join("sub")).unwrap();
    std::fs::write(tmp.path().join("sub/nested.txt"), "nested content").unwrap();

    let bytes = create_tar_gz(tmp.path()).unwrap();
    assert!(!bytes.is_empty());

    // Verify it's valid gzip + tar
    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);
    let entries: Vec<String> = archive
        .entries()
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path().unwrap().to_string_lossy().into_owned())
        .collect();

    assert!(
        entries.iter().any(|e| e.contains("hello.txt")),
        "should contain hello.txt, got: {entries:?}"
    );
    assert!(
        entries.iter().any(|e| e.contains("nested.txt")),
        "should contain nested.txt, got: {entries:?}"
    );
}

#[test]
fn preserves_file_content() {
    let tmp = tempfile::tempdir().unwrap();
    let content = "test content 12345";
    std::fs::write(tmp.path().join("file.txt"), content).unwrap();

    let bytes = create_tar_gz(tmp.path()).unwrap();

    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().into_owned();
        if path.contains("file.txt") {
            let mut buf = String::new();
            entry.read_to_string(&mut buf).unwrap();
            assert_eq!(buf, content);
            return;
        }
    }
    panic!("file.txt not found in archive");
}

#[test]
fn empty_directory_produces_archive() {
    let tmp = tempfile::tempdir().unwrap();
    let bytes = create_tar_gz(tmp.path()).unwrap();
    assert!(!bytes.is_empty());
}
