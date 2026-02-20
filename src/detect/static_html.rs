//! Static HTML site detection (fallback when no package.json).

use std::path::Path;

/// Check if the directory looks like a static HTML site.
/// Returns true when index.html exists and is non-empty, and no package.json is present.
pub fn is_static_html_site(project_dir: &Path) -> bool {
    // Must NOT have package.json
    if project_dir.join("package.json").exists() {
        return false;
    }

    // Must have a non-empty index.html
    let index_path = project_dir.join("index.html");
    match std::fs::metadata(&index_path) {
        Ok(meta) => meta.is_file() && meta.len() > 0,
        Err(_) => false,
    }
}

/// Scan for HTML files in the root directory (not recursive).
pub fn find_html_files(project_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(project_dir) {
        Ok(e) => e,
        Err(_) => return files,
    };
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str()
            && (name.ends_with(".html") || name.ends_with(".htm"))
        {
            files.push(name.to_string());
        }
    }
    files.sort();
    files
}
