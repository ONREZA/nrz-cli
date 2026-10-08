//! Reject known immutable dependency/target conflicts. Success is not ABI proof
//! or execution permission; unknown successor compatibility requires a real probe.
use crate::{RuntimeArtifactError, RuntimeProfile, VerifiedRuntimeArtifactGraph, invariant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionRuntimeFamily {
    Bun,
    Node,
    Python,
}

/// Neutral projection of a separately verified interpreter descriptor.
/// Exact versions, signatures and policy authorization remain their owners' facts.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionRuntimeTarget<'a> {
    pub family: ExecutionRuntimeFamily,
    pub os: &'a str,
    pub architecture: &'a str,
    pub libc: &'a str,
}

pub fn verify_execution_runtime_compatibility(
    graph: &VerifiedRuntimeArtifactGraph,
    layer_name: &str,
    target: ExecutionRuntimeTarget<'_>,
) -> Result<(), RuntimeArtifactError> {
    let layer = graph
        .wire()
        .runtime_layers
        .iter()
        .find(|layer| layer.layer_name.as_str() == layer_name)
        .ok_or_else(|| RuntimeArtifactError::Invariant("execution layer is absent".into()))?;
    let launch = layer
        .launch
        .as_ref()
        .ok_or_else(|| RuntimeArtifactError::Invariant("execution layer has no launch".into()))?;
    let family = match launch.profile {
        RuntimeProfile::Bun => ExecutionRuntimeFamily::Bun,
        RuntimeProfile::Node22 | RuntimeProfile::Node24 | RuntimeProfile::Node26 => {
            ExecutionRuntimeFamily::Node
        }
        RuntimeProfile::Cpython312 | RuntimeProfile::Cpython313 | RuntimeProfile::Cpython314 => {
            ExecutionRuntimeFamily::Python
        }
        RuntimeProfile::Executable => {
            return invariant("native execution cannot acquire an interpreter");
        }
    };
    if family != target.family {
        return invariant("execution runtime changes immutable application family");
    }
    for id in &layer.dependency_materialization_ids {
        let dependency = graph
            .wire()
            .dependencies
            .iter()
            .find(|dependency| dependency.materialization_id.as_str() == id.as_str())
            .expect("verified graph resolves every layer dependency");
        let compatibility = &dependency.compatibility;
        if compatibility.os != target.os
            || compatibility.architecture.to_string() != target.architecture
            || compatibility.libc.to_string() != target.libc
        {
            return invariant("execution runtime conflicts with dependency platform");
        }
        let python = family == ExecutionRuntimeFamily::Python;
        if (python
            && (dependency.kind.to_string() != "PYTHON_SITE_PACKAGES"
                || compatibility.runtime_family.as_str() != "python"))
            || (!python
                && (dependency.kind.to_string() != "JAVASCRIPT_NODE_MODULES"
                    || !matches!(compatibility.runtime_family.as_str(), "bun" | "javascript")))
        {
            return invariant("execution runtime conflicts with dependency family");
        }
        // runtimeVersion is frozen build provenance, not a successor ABI verdict.
        // The legacy abi string and aggregate ELF count cannot distinguish Node
        // module ABI from Node-API, or CPython-specific extensions from abi3.
    }
    Ok(())
}
