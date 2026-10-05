//! Application launcher declarations, independent of installation tooling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::SourceLogicalManifest;

pub const APPLICATION_RUNTIME_CONFIG_KEY: &str = "applicationRuntime";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ApplicationRuntimeFamily {
    #[serde(rename = "BUN", alias = "bun")]
    Bun,
    #[serde(rename = "NODE", alias = "node")]
    Node,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    pub args: Vec<String>,
}

impl ApplicationRuntimeDeclaration {
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
            (ApplicationRuntimeFamily::Bun, Some(version)) => version.starts_with("bun-"),
            (ApplicationRuntimeFamily::Node, Some(version)) => {
                matches!(version, "node-22" | "node-24" | "node-26")
            }
            (ApplicationRuntimeFamily::Node, None) => false,
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

/// One admitted application runtime must cover all COMPUTE layers.
pub fn source_application_runtime(
    manifest: &SourceLogicalManifest,
) -> Result<Option<ApplicationRuntimeFamily>, String> {
    let mut family = None;
    for layer in &manifest.layers {
        let Some(intent) = layer_application_runtime(layer.runtime_config.as_ref())? else {
            continue;
        };
        if layer.target != "COMPUTE" {
            return Err("non-COMPUTE layer declares an application runtime".into());
        }
        if family.is_some_and(|family| family != intent.family) {
            return Err("COMPUTE layers declare conflicting application runtime families".into());
        }
        family = Some(intent.family);
    }
    Ok(family)
}
