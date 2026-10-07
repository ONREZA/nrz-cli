//! Build tools and serving intent are independently frozen before execution.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{ApplicationRuntimeDeclaration, PythonMinor};

pub const SOURCE_BUILD_CONTEXT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum BuildToolchainFamily {
    #[serde(rename = "NODE")]
    Node,
    #[serde(rename = "BUN")]
    Bun,
    #[serde(rename = "PYTHON")]
    Python,
    #[serde(rename = "NATIVE")]
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildToolchainDeclaration {
    pub family: BuildToolchainFamily,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python_version: Option<PythonMinor>,
}

impl BuildToolchainDeclaration {
    pub fn validate(&self) -> Result<(), String> {
        if self.python_version.is_some() && self.family != BuildToolchainFamily::Python {
            return Err("build pythonVersion is only valid for the PYTHON toolchain".into());
        }
        Ok(())
    }

    /// Admission must persist this resolved minor so future catalog defaults
    /// cannot change an already frozen build snapshot.
    pub fn resolved_python_minor(&self) -> Option<PythonMinor> {
        (self.family == BuildToolchainFamily::Python)
            .then(|| self.python_version.unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceBuildContext {
    pub schema_version: u32,
    pub build_toolchain: BuildToolchainDeclaration,
    #[serde(deserialize_with = "deserialize_application_runtime")]
    pub application_runtime: Option<ApplicationRuntimeDeclaration>,
}

impl SourceBuildContext {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SOURCE_BUILD_CONTEXT_SCHEMA_VERSION {
            return Err("unsupported source build context schemaVersion".into());
        }
        self.build_toolchain.validate()?;
        if self.build_toolchain.family == BuildToolchainFamily::Python
            && self.build_toolchain.python_version.is_none()
        {
            return Err("frozen Python build toolchain requires an explicit pythonVersion".into());
        }
        if let Some(runtime) = &self.application_runtime {
            runtime.validate()?;
            if runtime.family == crate::ApplicationRuntimeFamily::Python
                && runtime.python_version.is_none()
            {
                return Err(
                    "frozen Python serving runtime requires an explicit pythonVersion".into(),
                );
            }
        }
        Ok(())
    }
}

// Absence and explicit null differ at this authority boundary: STATIC carries
// null, while a missing field is an incomplete context rather than an inference.
fn deserialize_application_runtime<'de, D>(
    deserializer: D,
) -> Result<Option<ApplicationRuntimeDeclaration>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer)
}
