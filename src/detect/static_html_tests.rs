use super::fs::LocalFs;
use super::static_html::*;

#[test]
fn detect_static_html_site() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
    assert!(is_static_html_site(&LocalFs::new(dir.path())));
}

#[test]
fn not_static_if_package_json_present() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
    std::fs::write(dir.path().join("package.json"), "{}").unwrap();
    assert!(!is_static_html_site(&LocalFs::new(dir.path())));
}

#[test]
fn not_static_if_no_index_html() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!is_static_html_site(&LocalFs::new(dir.path())));
}

#[test]
fn not_static_if_index_html_is_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("index.html")).unwrap();
    assert!(!is_static_html_site(&LocalFs::new(dir.path())));
}

#[test]
fn find_html_files_in_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.html"), "<html></html>").unwrap();
    std::fs::write(dir.path().join("about.html"), "<html></html>").unwrap();
    std::fs::write(dir.path().join("style.css"), "body{}").unwrap();
    let files = find_html_files(&LocalFs::new(dir.path()));
    assert_eq!(files, vec!["about.html", "index.html"]);
}

#[test]
fn find_html_files_includes_htm() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("page.htm"), "<html></html>").unwrap();
    let files = find_html_files(&LocalFs::new(dir.path()));
    assert_eq!(files, vec!["page.htm"]);
}

#[test]
fn find_html_files_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let files = find_html_files(&LocalFs::new(dir.path()));
    assert!(files.is_empty());
}
