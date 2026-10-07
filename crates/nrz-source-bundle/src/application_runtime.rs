//! Application launcher declarations, independent of installation tooling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SourceLogicalManifest;

pub const APPLICATION_RUNTIME_CONFIG_KEY: &str = "applicationRuntime";

/// Newly built Bun declarations use the supported major; retained execution
/// compatibility has its own publication-owned contract.
pub fn supported_bun_build_target(target: &str) -> bool {
    let Some(version) = target.strip_prefix("bun-") else {
        return false;
    };
    let parts = version.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts[0] == "1"
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ApplicationRuntimeFamily {
    #[serde(rename = "BUN", alias = "bun")]
    Bun,
    #[serde(rename = "NODE", alias = "node")]
    Node,
    #[serde(rename = "PYTHON", alias = "python")]
    Python,
    #[serde(rename = "EXECUTABLE", alias = "executable")]
    Executable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRuntimeIntent {
    pub family: ApplicationRuntimeFamily,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationRuntimeDeclaration {
    pub family: ApplicationRuntimeFamily,
    #[serde(
        default,
        rename = "pythonVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub python_version: Option<crate::PythonMinor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    pub args: Vec<String>,
}

impl ApplicationRuntimeDeclaration {
    pub fn validate(&self) -> Result<(), String> {
        if self.python_version.is_some() && self.family != ApplicationRuntimeFamily::Python {
            return Err("pythonVersion is only valid for PYTHON applications".into());
        }
        self.intent().validate()
    }

    pub fn validate_target(&self, target: Option<&str>) -> Result<(), String> {
        self.validate()?;
        self.intent().validate_target(target)?;
        if self
            .python_version
            .is_some_and(|minor| Some(minor.target()) != target)
        {
            return Err("selected pythonVersion conflicts with frozen build target".into());
        }
        Ok(())
    }
    pub fn intent(&self) -> ApplicationRuntimeIntent {
        ApplicationRuntimeIntent {
            family: self.family,
            args: self.args.clone(),
        }
    }
}

impl ApplicationRuntimeIntent {
    pub fn validate(&self) -> Result<(), String> {
        if self.args.len() > 64
            || self
                .args
                .iter()
                .any(|arg| arg.len() > 4096 || arg.contains('\0'))
        {
            return Err("application runtime arguments exceed the launch contract".into());
        }
        Ok(())
    }

    pub fn validate_target(&self, version: Option<&str>) -> Result<(), String> {
        self.validate()?;
        let compatible = match (self.family, version) {
            (ApplicationRuntimeFamily::Bun, None) => true,
            (ApplicationRuntimeFamily::Bun, Some(version)) => supported_bun_build_target(version),
            (ApplicationRuntimeFamily::Node, Some(version)) => {
                matches!(version, "node-22" | "node-24" | "node-26")
            }
            (ApplicationRuntimeFamily::Node, None) => false,
            (ApplicationRuntimeFamily::Python, Some(version)) => {
                crate::PythonMinor::from_target(version).is_some()
            }
            (ApplicationRuntimeFamily::Executable, Some("native-linux-x86_64-glibc")) => true,
            (ApplicationRuntimeFamily::Python | ApplicationRuntimeFamily::Executable, _) => false,
        };
        if !compatible {
            return Err(format!(
                "application runtime {:?} conflicts with admitted runtime target {:?}; select a compatible trusted runtime before building",
                self.family, version
            ));
        }
        Ok(())
    }
}

pub fn layer_application_runtime(
    config: Option<&Value>,
) -> Result<Option<ApplicationRuntimeIntent>, String> {
    let Some(value) = config.and_then(|config| config.get(APPLICATION_RUNTIME_CONFIG_KEY)) else {
        return Ok(None);
    };
    let intent: ApplicationRuntimeIntent = serde_json::from_value(value.clone())
        .map_err(|error| format!("invalid application runtime: {error}"))?;
    intent.validate()?;
    if let Some(config) = config {
        if let Some(family) = config.get("runtimeFamily") {
            let consistent = match family.as_str() {
                Some("PYTHON") => intent.family == ApplicationRuntimeFamily::Python,
                Some("JAVASCRIPT") => intent.family != ApplicationRuntimeFamily::Python,
                _ => false,
            };
            if !consistent {
                return Err("application runtime conflicts with legacy runtime family".into());
            }
        }
        if let Some(binary) = config.get("isBinaryEntry")
            && binary.as_bool() != Some(intent.family == ApplicationRuntimeFamily::Executable)
        {
            return Err("application runtime conflicts with binary entry declaration".into());
        }
    }
    Ok(Some(intent))
}

pub fn validate_build_runtime_version(
    config: Option<&Value>,
    target: Option<&str>,
) -> Result<(), String> {
    let Some(version) = config.and_then(|config| config.get("buildRuntimeVersion")) else {
        return Ok(());
    };
    let version = version
        .as_str()
        .ok_or_else(|| "buildRuntimeVersion must be a string".to_string())?;
    if Some(version) != target {
        return Err(format!(
            "application build runtime version {version} differs from admitted target {target:?}"
        ));
    }
    Ok(())
}

/// Validate layer declarations; return a family only when all declarations agree.
pub fn source_application_runtime(
    manifest: &SourceLogicalManifest,
) -> Result<Option<ApplicationRuntimeFamily>, String> {
    let mut family = None;
    let mut mixed = false;
    for layer in &manifest.layers {
        let Some(intent) = layer_application_runtime(layer.runtime_config.as_ref())? else {
            continue;
        };
        if layer.target != "COMPUTE" {
            return Err("non-COMPUTE layer declares an application runtime".into());
        }
        if family.is_some_and(|family| family != intent.family) {
            mixed = true;
        }
        family = Some(intent.family);
    }
    Ok(if mixed { None } else { family })
}
