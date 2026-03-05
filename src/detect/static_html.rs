//! Static HTML site detection (fallback when no package.json).

use super::fs::Fs;

/// Check if the directory looks like a static HTML site.
/// Returns true when index.html exists and no package.json is present.
pub fn is_static_html_site(fs: &dyn Fs) -> bool {
    // Must NOT have package.json
    if fs.exists("package.json") {
        return false;
    }

    // Must have a non-empty index.html
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
