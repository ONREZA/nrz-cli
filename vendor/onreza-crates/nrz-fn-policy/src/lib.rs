//! Static function policy analyzer for ONREZA Functions v1.
//!
//! Parses the bounded function module graph with oxc and reports every denied
//! capability and disallowed import in a single pass, without executing user
//! code. The platform (`artifact-ingest`) runs this as the authoritative publish
//! gate; `nrz-cli` runs the same logic for a local preview.
//!
//! The analyzer holds no policy data of its own: the allowed module/runtime API
//! sets arrive via [`PolicyConfig`], which callers build from the generated
//! `nrz-contract` runtime policy so there is a single source of truth.

mod declaration;
mod resolve;
mod scan;

use std::collections::{BTreeMap, BTreeSet};

use resolve::{LocalResolution, is_local_specifier, resolve_local_import};
use scan::{ScanCapability, scan_module};
use serde::{Deserialize, Serialize};

pub use declaration::{
    DeclaredFunctionTrigger, FunctionConfigDeclaration, FunctionConfigError, FunctionEntryAnalysis,
    analyze_function_entry,
};

pub const POLICY_VERSION: &str = "onreza-functions-policy/v1";
const MAX_SCANNED_MODULES: usize = 512;
const DEFAULT_MODULE_DENIAL_REASON: &str = "Public ONREZA Functions v1 source is self-contained; package imports are not available, and only allowlisted node:* utility specifiers are accepted";

/// Policy data the analyzer enforces, built by callers from the generated
/// runtime policy contract.
///
/// Enforcement is default-deny: any bare specifier that is not local and not in
/// `allowed_module_specifiers` is rejected. The denied builtin list is not part
/// of the config because it never affects the decision — only the human-readable
/// reason, which lives in this crate.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyConfig {
    pub local_import_extensions: Vec<String>,
    pub allowed_module_specifiers: Vec<String>,
    pub allowed_bun_properties: Vec<String>,
}

/// In-memory function source set: module path (relative to the bundle root)
/// mapped to its source text.
pub type SourceSet = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyViolation {
    pub capability: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyReport {
    pub policy_version: String,
    pub status: PolicyStatus,
    pub entrypoint: String,
    pub checked_modules: u32,
    pub violations: Vec<PolicyViolation>,
}

/// Run the publish-time function policy scan over the in-memory module graph
/// rooted at `entrypoint`. Aggregates every violation; never executes code.
pub fn run_function_policy_check(
    config: &PolicyConfig,
    entrypoint: &str,
    sources: &SourceSet,
) -> PolicyReport {
    let extension_set: BTreeSet<&str> = config
        .local_import_extensions
        .iter()
        .map(String::as_str)
        .collect();

    let normalized_entry = match normalize_entrypoint(entrypoint) {
        Ok(value) => value,
        Err(violation) => return failed_report(entrypoint, 0, vec![violation]),
    };

    if !has_supported_extension(&normalized_entry, &extension_set) {
        return failed_report(
            entrypoint,
            0,
            vec![PolicyViolation {
                capability: "entrypoint".to_string(),
                reason: UNSUPPORTED_SOURCE_REASON.to_string(),
                importer: Some(normalized_entry.clone()),
                specifier: Some(entrypoint.to_string()),
            }],
        );
    }

    let mut violations: Vec<PolicyViolation> = Vec::new();
    let mut scanned: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = vec![normalized_entry];
    let mut checked_modules: u32 = 0;

    while let Some(module) = queue.pop() {
        if scanned.contains(&module) {
            continue;
        }
        if scanned.len() >= MAX_SCANNED_MODULES {
            violations.push(PolicyViolation {
                capability: "module graph".to_string(),
                reason: format!(
                    "Function module graph exceeded {MAX_SCANNED_MODULES} scanned files"
                ),
                importer: Some(module),
                specifier: None,
            });
            break;
        }
        scanned.insert(module.clone());
        checked_modules += 1;

        if !has_supported_extension(&module, &extension_set) {
            violations.push(PolicyViolation {
                capability: "module format".to_string(),
                reason: UNSUPPORTED_SOURCE_REASON.to_string(),
                importer: Some(module.clone()),
                specifier: None,
            });
            continue;
        }

        let Some(source) = sources.get(&module) else {
            continue;
        };

        let result = scan_module(&module, source, &config.allowed_bun_properties);

        if result.parse_failed {
            violations.push(PolicyViolation {
                capability: "module parse".to_string(),
                reason: "Function module could not be parsed as ESM TypeScript/JavaScript"
                    .to_string(),
                importer: Some(module.clone()),
                specifier: None,
            });
            continue;
        }

        for capability in result.capabilities {
            violations.push(capability_violation(capability, &module));
        }

        for specifier in &result.imports {
            if is_local_specifier(specifier) {
                match resolve_local_import(
                    &module,
                    specifier,
                    &config.local_import_extensions,
                    &|candidate| sources.contains_key(candidate),
                ) {
                    LocalResolution::Module(resolved) => queue.push(resolved),
                    LocalResolution::NotFound => {}
                    LocalResolution::Escapes => violations.push(PolicyViolation {
                        capability: "module graph".to_string(),
                        reason: "Local imports must stay inside the function bundle".to_string(),
                        importer: Some(module.clone()),
                        specifier: Some(specifier.clone()),
                    }),
                }
                continue;
            }

            if config
                .allowed_module_specifiers
                .iter()
                .any(|allowed| allowed == specifier)
            {
                continue;
            }

            violations.push(PolicyViolation {
                capability: format!("module:{specifier}"),
                reason: denied_module_reason(specifier),
                importer: Some(module.clone()),
                specifier: Some(specifier.clone()),
            });
        }

        if result.computed_dynamic_import {
            violations.push(PolicyViolation {
                capability: "computed dynamic import".to_string(),
                reason: "Computed dynamic imports are not available in ONREZA Functions v1"
                    .to_string(),
                importer: Some(module.clone()),
                specifier: None,
            });
        }
    }

    PolicyReport {
        policy_version: POLICY_VERSION.to_string(),
        status: if violations.is_empty() {
            PolicyStatus::Passed
        } else {
            PolicyStatus::Failed
        },
        entrypoint: entrypoint.to_string(),
        checked_modules,
        violations,
    }
}

const UNSUPPORTED_SOURCE_REASON: &str =
    "ONREZA Functions v1 supports ESM TypeScript/JavaScript modules only";

fn capability_violation(capability: ScanCapability, importer: &str) -> PolicyViolation {
    let (capability, reason) = match capability {
        ScanCapability::BunAmbient => (
            "Bun ambient runtime API",
            "Ambient Bun runtime APIs are not available in ONREZA Functions v1",
        ),
        ScanCapability::Worker => (
            "Worker",
            "Nested Workers are not available in ONREZA Functions v1",
        ),
        ScanCapability::ParentMessageChannel => (
            "postMessage",
            "The parent Worker message channel is not available in ONREZA Functions v1",
        ),
        ScanCapability::ProcessControl => (
            "process control",
            "Process control APIs are not available in ONREZA Functions v1",
        ),
        ScanCapability::CommonJsExports => (
            "CommonJS module syntax",
            "ONREZA Functions v1 supports ESM modules only",
        ),
        ScanCapability::CommonJsRequire => (
            "CommonJS require",
            "ONREZA Functions v1 supports ESM imports only",
        ),
    };
    PolicyViolation {
        capability: capability.to_string(),
        reason: reason.to_string(),
        importer: Some(importer.to_string()),
        specifier: None,
    }
}

fn denied_module_reason(specifier: &str) -> String {
    let normalized = specifier.strip_prefix("node:").unwrap_or(specifier);
    known_denied_module_reason(specifier)
        .or_else(|| known_denied_module_reason(normalized))
        .unwrap_or(DEFAULT_MODULE_DENIAL_REASON)
        .to_string()
}

fn known_denied_module_reason(specifier: &str) -> Option<&'static str> {
    let reason = match specifier {
        "bun" => {
            "Direct imports from bun expose SQL, Redis, S3, shell and other ambient runtime APIs"
        }
        "bun:ffi" => "Native FFI is not available in ONREZA Functions v1",
        "bun:sqlite" => "SQLite is not available in ONREZA Functions v1",
        "child_process" => "Subprocess APIs are not available in ONREZA Functions v1",
        "cluster" => "Process clustering is not available in ONREZA Functions v1",
        "dgram" => "Raw UDP sockets are not available in ONREZA Functions v1",
        "dns" | "dns/promises" => "Direct DNS APIs are not available in ONREZA Functions v1",
        "fs" | "fs/promises" => "Raw filesystem APIs are not available in ONREZA Functions v1",
        "http" | "http2" => "Listening/raw HTTP sockets are not available in ONREZA Functions v1",
        "https" => "Listening/raw HTTPS sockets are not available in ONREZA Functions v1",
        "inspector" => "Inspector APIs are not available in ONREZA Functions v1",
        "module" => "Dynamic module loading is not available in ONREZA Functions v1",
        "net" => "Raw TCP sockets are not available in ONREZA Functions v1",
        "process" => "Ambient process APIs are not available in ONREZA Functions v1",
        "tls" => "Raw TLS sockets are not available in ONREZA Functions v1",
        "vm" => "VM APIs are not available in ONREZA Functions v1",
        "worker_threads" => "Nested Workers are not available in ONREZA Functions v1",
        _ => return None,
    };
    Some(reason)
}

fn normalize_entrypoint(entrypoint: &str) -> Result<String, PolicyViolation> {
    if entrypoint.starts_with('/') {
        return Err(PolicyViolation {
            capability: "entrypoint".to_string(),
            reason: "Function entrypoint must be relative to the bundle root".to_string(),
            importer: None,
            specifier: Some(entrypoint.to_string()),
        });
    }

    let mut stack: Vec<&str> = Vec::new();
    for segment in entrypoint.split('/').filter(|segment| !segment.is_empty()) {
        match segment {
            "." => {}
            ".." => {
                if stack.pop().is_none() {
                    return Err(PolicyViolation {
                        capability: "entrypoint".to_string(),
                        reason: "Function entrypoint must stay inside the bundle root".to_string(),
                        importer: None,
                        specifier: Some(entrypoint.to_string()),
                    });
                }
            }
            other => stack.push(other),
        }
    }
    Ok(stack.join("/"))
}

fn has_supported_extension(path: &str, extensions: &BTreeSet<&str>) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    let Some(dot) = file.rfind('.') else {
        return false;
    };
    extensions.contains(&file[dot..])
}

fn failed_report(
    entrypoint: &str,
    checked_modules: u32,
    violations: Vec<PolicyViolation>,
) -> PolicyReport {
    PolicyReport {
        policy_version: POLICY_VERSION.to_string(),
        status: PolicyStatus::Failed,
        entrypoint: entrypoint.to_string(),
        checked_modules,
        violations,
    }
}
