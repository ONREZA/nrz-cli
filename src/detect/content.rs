//! Bounded content admission shared by local and remote detection.

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;

use super::fs::{
    MAX_DETECTION_FILE_CONTENT_BYTES, MAX_DETECTION_MANIFEST_BYTES,
    MAX_DETECTION_TOTAL_CONTENT_BYTES, SERVER_ENTRY_FILES,
};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentMode {
    Full,
    Optional,
    Prefix,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRule {
    pub mode: ContentMode,
    pub max_bytes: usize,
}

pub fn content_rule(path: &str) -> ContentRule {
    if path == "yarn.lock" {
        ContentRule {
            mode: ContentMode::Prefix,
            max_bytes: super::package_manager::YARN_LOCK_HEADER_BYTES,
        }
    } else {
        ContentRule {
            mode: if SERVER_ENTRY_FILES.contains(&path)
                || super::native::is_go_detection_source_path(path)
                || super::python::PYTHON_ENTRY_CANDIDATES.contains(&path)
                || path == "index.html"
            {
                ContentMode::Optional
            } else {
                ContentMode::Full
            },
            max_bytes: MAX_DETECTION_FILE_CONTENT_BYTES,
        }
    }
}

#[derive(Debug)]
pub struct EvidenceIncomplete {
    pub file: String,
}

impl fmt::Display for EvidenceIncomplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Detection evidence is incomplete for {}; choose a smaller project root or declare the framework and deploy runtime in onreza.toml",
            self.file
        )
    }
}

impl std::error::Error for EvidenceIncomplete {}

#[derive(Debug)]
pub enum InputLimit {
    TreeEntries(usize),
    ContentFiles(usize),
    PathBytes(usize),
    PathDepth(usize),
}

impl fmt::Display for InputLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TreeEntries(limit) => write!(
                f,
                "detection manifest contains too many tree entries (max {limit})"
            ),
            Self::ContentFiles(limit) => write!(
                f,
                "detection manifest requires too many file contents (max {limit})"
            ),
            Self::PathBytes(limit) => write!(f, "detection manifest path exceeds {limit} bytes"),
            Self::PathDepth(limit) => {
                write!(f, "detection manifest path exceeds {limit} components")
            }
        }
    }
}

impl std::error::Error for InputLimit {}

pub(super) fn require_runtime_evidence(
    fs: &dyn super::fs::Fs,
    detection: &super::types::DetectionResult,
    explicit_intent: bool,
) -> anyhow::Result<()> {
    // An omitted Go package may own the application; absence is not static evidence.
    if !explicit_intent
        && matches!(detection.framework.as_str(), "other" | "static-html")
        && !super::package_json::PackageJson::load_from_fs(fs)
            .is_some_and(|package| super::has_javascript_application_signal(fs, &package))
        && fs.is_file("go.mod")
        && let Some(file) = super::native::unreadable_go_source(fs)
    {
        return Err(EvidenceIncomplete { file }.into());
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentRequest {
    pub files: Vec<String>,
    pub content_rules: BTreeMap<String, ContentRule>,
    pub limits: ContentLimits,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentLimits {
    pub manifest_bytes: usize,
    pub total_content_bytes: usize,
}

impl ContentRequest {
    pub fn new(files: Vec<String>) -> Self {
        let content_rules = files
            .iter()
            .map(|path| (path.clone(), content_rule(path)))
            .collect();
        Self {
            files,
            content_rules,
            limits: ContentLimits {
                manifest_bytes: MAX_DETECTION_MANIFEST_BYTES,
                total_content_bytes: MAX_DETECTION_TOTAL_CONTENT_BYTES,
            },
        }
    }
}
