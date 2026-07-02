//! Static HTML site detection.

use super::fs::Fs;

/// Check if the directory looks like a static HTML site.
/// Returns true when a non-empty root index.html exists.
pub fn is_static_html_site(fs: &dyn Fs) -> bool {
    match fs.read_file("index.html") {
        Some(content) => !content.is_empty(),
        None => false,
    }
}

/// Scan for HTML files in the root directory (not recursive).
pub fn find_html_files(fs: &dyn Fs) -> Vec<String> {
    let mut files: Vec<String> = fs
        .list_dir("")
        .into_iter()
        .filter(|name| name.ends_with(".html") || name.ends_with(".htm"))
        .collect();
    files.sort();
    files
}
