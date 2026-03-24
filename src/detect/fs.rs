//! Filesystem abstraction for framework detection.
//!
//! `LocalFs` delegates to `std::fs`, `VirtualFs` operates on an in-memory
//! tree built from a JSON manifest (used by `nrz detect --stdin`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Abstract filesystem for detection logic.
pub trait Fs {
    fn exists(&self, path: &str) -> bool;
    fn is_dir(&self, path: &str) -> bool;
    fn read_file(&self, path: &str) -> Option<String>;
    fn list_dir(&self, path: &str) -> Vec<String>;
}

/// Local filesystem rooted at `root`.
pub struct LocalFs {
    root: PathBuf,
}

impl LocalFs {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
}

impl Fs for LocalFs {
    fn exists(&self, path: &str) -> bool {
        self.root.join(path).exists()
    }

    fn is_dir(&self, path: &str) -> bool {
        self.root.join(path).is_dir()
    }

    fn read_file(&self, path: &str) -> Option<String> {
        let full = self.root.join(path);
        match std::fs::read_to_string(&full) {
            Ok(content) => Some(content),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                tracing::warn!(
                    path = %full.display(),
                    error = %err,
                    "failed to read detection file"
                );
                None
            }
        }
    }

    fn list_dir(&self, path: &str) -> Vec<String> {
        let dir = if path.is_empty() {
            self.root.clone()
        } else {
            self.root.join(path)
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
pub struct VirtualFs {
    tree: HashSet<String>,
    dirs: HashSet<String>,
    files: HashMap<String, String>,
}

impl VirtualFs {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let manifest: VirtualFsManifest = serde_json::from_str(json)?;
        let mut tree = HashSet::new();
        let mut dirs = HashSet::new();

        // Root is always a directory
        dirs.insert(String::new());

        for entry in &manifest.tree {
            let normalized = normalize_path(entry);
            if entry.ends_with('/') {
                dirs.insert(normalized.clone());
            }
            tree.insert(normalized.clone());
            register_parent_dirs(&normalized, &mut dirs);
        }

        let mut files = HashMap::new();
        for (path, content) in &manifest.files {
            let normalized = normalize_path(path);
            files.insert(normalized.clone(), content.clone());
            tree.insert(normalized.clone());
            register_parent_dirs(&normalized, &mut dirs);
        }

        Ok(Self { tree, dirs, files })
    }
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
    "prisma/schema.prisma",
    "prisma.config.ts",
    "prisma.config.js",
    "tsconfig.json",
    "tsconfig.app.json",
    "gatsby-config.js",
    "gatsby-config.ts",
    "angular.json",
    "docusaurus.config.js",
    "docusaurus.config.ts",
    ".vitepress/config.ts",
    ".vitepress/config.js",
];
