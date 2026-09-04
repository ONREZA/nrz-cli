//! Filesystem abstraction for framework detection.
//!
//! `LocalFs` delegates to `std::fs`, `VirtualFs` operates on an in-memory
//! tree built from a JSON manifest (used by `nrz detect --stdin`).

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const MAX_DETECTION_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
const MAX_DETECTION_TREE_ENTRIES: usize = 100_000;
const MAX_DETECTION_CONTENT_FILES: usize = 256;
const MAX_DETECTION_PATH_BYTES: usize = 1024;
pub(super) const MAX_DETECTION_PATH_DEPTH: usize = 64;
pub(super) const MAX_DETECTION_FILE_CONTENT_BYTES: usize = 512 * 1024;
const MAX_DETECTION_TOTAL_CONTENT_BYTES: usize = 2 * 1024 * 1024;

/// Abstract filesystem for detection logic.
pub trait Fs {
    fn exists(&self, path: &str) -> bool;
    fn is_dir(&self, path: &str) -> bool;
    fn read_file(&self, path: &str) -> Option<String>;
    fn read_file_prefix(&self, path: &str, max_bytes: usize) -> Option<String> {
        let content = self.read_file(path)?;
        if content.len() <= max_bytes {
            return Some(content);
        }
        let end = (0..=max_bytes)
            .rev()
            .find(|offset| content.is_char_boundary(*offset))?;
        Some(content[..end].to_string())
    }
    fn list_dir(&self, path: &str) -> Vec<String>;
}

/// Local filesystem rooted at `root`.
pub struct LocalFs {
    root: PathBuf,
    canonical_root: PathBuf,
}

impl LocalFs {
    pub fn new(root: &Path) -> Self {
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        Self {
            root: root.to_path_buf(),
            canonical_root,
        }
    }

    fn resolve_existing(&self, path: &str) -> Option<PathBuf> {
        let normalized = path.replace('\\', "/");
        let relative = Path::new(&normalized);
        let bytes = normalized.as_bytes();
        if bytes.get(1) == Some(&b':')
            || normalized.starts_with("//")
            || relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            return None;
        }
        let canonical = self.root.join(relative).canonicalize().ok()?;
        canonical
            .starts_with(&self.canonical_root)
            .then_some(canonical)
    }
}

impl Fs for LocalFs {
    fn exists(&self, path: &str) -> bool {
        self.resolve_existing(path).is_some()
    }

    fn is_dir(&self, path: &str) -> bool {
        self.resolve_existing(path)
            .is_some_and(|path| path.is_dir())
    }

    fn read_file(&self, path: &str) -> Option<String> {
        let full = self.resolve_existing(path)?;
        let metadata = std::fs::metadata(&full).ok()?;
        if !metadata.is_file() || metadata.len() > MAX_DETECTION_FILE_CONTENT_BYTES as u64 {
            return None;
        }
        let mut content = String::new();
        let mut file = std::fs::File::open(&full)
            .ok()?
            .take(MAX_DETECTION_FILE_CONTENT_BYTES as u64 + 1);
        file.read_to_string(&mut content).ok()?;
        (content.len() <= MAX_DETECTION_FILE_CONTENT_BYTES).then_some(content)
    }

    fn read_file_prefix(&self, path: &str, max_bytes: usize) -> Option<String> {
        let full = self.resolve_existing(path)?;
        if !std::fs::metadata(&full).ok()?.is_file() {
            return None;
        }
        let mut bytes = Vec::with_capacity(max_bytes);
        std::fs::File::open(&full)
            .ok()?
            .take(max_bytes as u64)
            .read_to_end(&mut bytes)
            .ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn list_dir(&self, path: &str) -> Vec<String> {
        let Some(dir) = self.resolve_existing(path) else {
            return Vec::new();
        };
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(err) => {
                tracing::warn!(
                    path = %dir.display(),
                    error = %err,
                    "failed to list detection directory"
                );
                return Vec::new();
            }
        };
        let mut result: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .collect();
        result.sort();
        result
    }
}

/// In-memory filesystem from a JSON manifest.
#[derive(Debug)]
pub struct VirtualFs {
    tree: HashSet<String>,
    dirs: HashSet<String>,
    files: HashMap<String, String>,
}

impl VirtualFs {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        if json.len() > MAX_DETECTION_MANIFEST_BYTES {
            anyhow::bail!(
                "detection manifest exceeds {} bytes",
                MAX_DETECTION_MANIFEST_BYTES
            );
        }
        let manifest: VirtualFsManifest = serde_json::from_str(json)?;
        if manifest.tree.len() > MAX_DETECTION_TREE_ENTRIES {
            anyhow::bail!(
                "detection manifest contains too many tree entries (max {})",
                MAX_DETECTION_TREE_ENTRIES
            );
        }
        if manifest.files.len() > MAX_DETECTION_CONTENT_FILES {
            anyhow::bail!(
                "detection manifest contains too many file contents (max {})",
                MAX_DETECTION_CONTENT_FILES
            );
        }
        let total_content_bytes = manifest.files.values().try_fold(0usize, |total, content| {
            if content.len() > MAX_DETECTION_FILE_CONTENT_BYTES {
                anyhow::bail!(
                    "detection file content exceeds {} bytes",
                    MAX_DETECTION_FILE_CONTENT_BYTES
                );
            }
            total
                .checked_add(content.len())
                .ok_or_else(|| anyhow::anyhow!("detection file content size overflow"))
        })?;
        if total_content_bytes > MAX_DETECTION_TOTAL_CONTENT_BYTES {
            anyhow::bail!(
                "detection file contents exceed {} bytes in total",
                MAX_DETECTION_TOTAL_CONTENT_BYTES
            );
        }
        let mut tree = HashSet::new();
        let mut dirs = HashSet::new();

        // Root is always a directory
        dirs.insert(String::new());

        for entry in manifest.tree {
            validate_virtual_path(&entry)?;
            let normalized = normalize_path(&entry);
            if entry.ends_with('/') {
                dirs.insert(normalized.clone());
            }
            tree.insert(normalized.clone());
            register_parent_dirs(&normalized, &mut dirs);
        }

        let mut files = HashMap::new();
        for (path, content) in manifest.files {
            validate_virtual_path(&path)?;
            let normalized = normalize_path(&path);
            files.insert(normalized.clone(), content);
            tree.insert(normalized.clone());
            register_parent_dirs(&normalized, &mut dirs);
        }

        Ok(Self { tree, dirs, files })
    }
}

fn validate_virtual_path(path: &str) -> anyhow::Result<()> {
    if path.len() > MAX_DETECTION_PATH_BYTES {
        anyhow::bail!(
            "detection manifest path exceeds {} bytes",
            MAX_DETECTION_PATH_BYTES
        );
    }
    let normalized = path.replace('\\', "/");
    let relative = Path::new(&normalized);
    let bytes = normalized.as_bytes();
    if normalized.contains('\0')
        || bytes.get(1) == Some(&b':')
        || normalized.starts_with("//")
        || relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        anyhow::bail!("detection manifest path must be relative and must not contain '..'");
    }
    let depth = normalized
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .count();
    if depth > MAX_DETECTION_PATH_DEPTH {
        anyhow::bail!(
            "detection manifest path exceeds {} components",
            MAX_DETECTION_PATH_DEPTH
        );
    }
    Ok(())
}

impl Fs for VirtualFs {
    fn exists(&self, path: &str) -> bool {
        let normalized = normalize_path(path);
        self.tree.contains(&normalized) || self.dirs.contains(&normalized)
    }

    fn is_dir(&self, path: &str) -> bool {
        let normalized = normalize_path(path);
        self.dirs.contains(&normalized)
    }

    fn read_file(&self, path: &str) -> Option<String> {
        let normalized = normalize_path(path);
        self.files.get(&normalized).cloned()
    }

    fn list_dir(&self, path: &str) -> Vec<String> {
        let normalized = normalize_path(path);
        let prefix = if normalized.is_empty() {
            String::new()
        } else {
            format!("{normalized}/")
        };

        let mut entries = Vec::new();
        let mut seen = HashSet::new();

        // Collect direct children from tree entries and dirs
        for entry in self.tree.iter().chain(self.dirs.iter()) {
            if entry.is_empty() {
                continue;
            }
            let child = if prefix.is_empty() {
                entry.as_str()
            } else if let Some(rest) = entry.strip_prefix(&prefix) {
                if rest.is_empty() {
                    continue;
                }
                rest
            } else {
                continue;
            };

            // Only direct children (no further slashes)
            let name = match child.find('/') {
                Some(idx) => &child[..idx],
                None => child,
            };

            if !name.is_empty() && seen.insert(name.to_string()) {
                entries.push(name.to_string());
            }
        }

        entries.sort();
        entries
    }
}

#[derive(Deserialize)]
struct VirtualFsManifest {
    #[serde(default)]
    tree: Vec<String>,
    #[serde(default)]
    files: HashMap<String, String>,
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches("./")
        .trim_end_matches('/')
        .replace('\\', "/")
        .to_string()
}

fn register_parent_dirs(path: &str, dirs: &mut HashSet<String>) {
    let mut current = path.to_string();
    while let Some(idx) = current.rfind('/') {
        current = current[..idx].to_string();
        if !dirs.insert(current.clone()) {
            break; // already registered this and all parents
        }
    }
}

/// Files whose content the server should send in the manifest
/// for accurate remote detection.
pub const DETECTION_CONTENT_FILES: &[&str] = &[
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "main.py",
    "app.py",
    "server.py",
    "src/main.py",
    "src/app.py",
    "src/server.py",
    "package.json",
    "pnpm-workspace.yaml",
    "turbo.json",
    "nx.json",
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "next.config.mts",
    "nuxt.config.ts",
    "nuxt.config.js",
    "svelte.config.js",
    "svelte.config.ts",
    "astro.config.mjs",
    "astro.config.ts",
    "astro.config.js",
    "remix.config.js",
    "react-router.config.ts",
    "react-router.config.js",
    "vite.config.ts",
    "vite.config.mts",
    "vite.config.js",
    "vite.config.mjs",
    "app.config.ts",
    "app.config.js",
    "app.json",
    "tsconfig.json",
    "tsconfig.app.json",
    "gatsby-config.js",
    "gatsby-config.ts",
    "angular.json",
    "docusaurus.config.js",
    "docusaurus.config.ts",
    ".vitepress/config.ts",
    ".vitepress/config.js",
    "keystone.ts",
    "keystone.js",
    "redwood.toml",
    "adonisrc.ts",
    "adonisrc.js",
    "nitro.config.ts",
    "nitro.config.js",
    "config/server.ts",
    "config/server.js",
    "server.js",
    "server.mjs",
    "server.cjs",
    "server.ts",
    "server.mts",
    "server.cts",
    "app.js",
    "app.mjs",
    "app.cjs",
    "app.ts",
    "app.mts",
    "app.cts",
    "index.js",
    "index.mjs",
    "index.cjs",
    "index.ts",
    "index.mts",
    "index.cts",
    "main.js",
    "main.mjs",
    "main.cjs",
    "main.ts",
    "main.mts",
    "main.cts",
    "src/server.js",
    "src/server.mjs",
    "src/server.cjs",
    "src/server.ts",
    "src/server.mts",
    "src/server.cts",
    "src/app.js",
    "src/app.mjs",
    "src/app.cjs",
    "src/app.ts",
    "src/app.mts",
    "src/app.cts",
    "src/index.js",
    "src/index.mjs",
    "src/index.cjs",
    "src/index.ts",
    "src/index.mts",
    "src/index.cts",
    "src/main.js",
    "src/main.mjs",
    "src/main.cjs",
    "src/main.ts",
    "src/main.mts",
    "src/main.cts",
    "dist/server.js",
    "dist/server.mjs",
    "dist/server.cjs",
    "dist/index.js",
    "dist/index.mjs",
    "dist/index.cjs",
    "dist/main.js",
    "dist/main.mjs",
    "dist/main.cjs",
    "dist/src/main.js",
    "dist/src/main.mjs",
    "dist/src/main.cjs",
    "build/server.js",
    "build/server.mjs",
    "build/server.cjs",
    "build/index.js",
    "build/index.mjs",
    "build/index.cjs",
    "build/main.js",
    "build/main.mjs",
    "build/main.cjs",
];
