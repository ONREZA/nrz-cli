//! Check private source runtime semantics, then bind packages and the same admitted
//! targets/dependency policies into one graph. No publication or READY authority.

use std::path::Path;

use nrz_runtime_artifact::{
    MAX_LAYER_DEPENDENCIES, MAX_RUNTIME_LAYERS,
    application_package_v1::ApplicationPackagePolicy,
    runtime_artifact_graph_v3::{GraphV3Error, VerifiedRuntimeArtifactGraphV3},
    source_graph_v3::{
        LayerBuildTarget, SourceDependencyMaterializationV3, compile_source_bundle_v2_runtime_graph,
    },
};

use crate::{
    application_erofs_tools::ApplicationErofsTools,
    application_package::{
        APPLICATION_EROFS_PROFILE_V1, ApplicationMaterializationError,
        ApplicationMaterializationLimits, MaterializedApplicationPackages,
        materialize_application_packages,
    },
    source_v2_preparation::PreparedSourceV2,
    source_v2_runtime::{
        AdmittedDependencyRoot, RuntimeVerificationError, RuntimeVerificationLimits,
        verify_prepared_source_v2_runtime,
    },
};

/// The trusted dependency owner must already bind these materialization bytes to
/// this frozen source root. This API checks metadata policy, not dependency images.
pub struct AdmittedRuntimeDependency<'a> {
    pub root: AdmittedDependencyRoot<'a>,
    pub materialization: SourceDependencyMaterializationV3<'a>,
}

pub struct MaterializedSourceV2Runtime {
    packages: MaterializedApplicationPackages,
    graph: VerifiedRuntimeArtifactGraphV3,
    dependencies: Option<super::source_v2_dependencies::DependencyStorage>,
}

impl MaterializedSourceV2Runtime {
    pub fn dependencies(&self) -> &[super::source_v2_dependencies::OwnedSourceV2Dependency] {
        self.dependencies
            .as_ref()
            .map_or(&[], |owner| owner.outputs.as_slice())
    }
    pub(crate) fn retain_dependencies(
        &mut self,
        owner: super::source_v2_dependencies::DependencyStorage,
    ) {
        self.dependencies = Some(owner);
    }
    pub fn packages(&self) -> &MaterializedApplicationPackages {
        &self.packages
    }
    pub fn graph(&self) -> &VerifiedRuntimeArtifactGraphV3 {
        &self.graph
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceV2MaterializationError {
    #[error("invalid admitted runtime dependency: {0}")]
    Dependency(&'static str),
    #[error(transparent)]
    Runtime(#[from] RuntimeVerificationError),
    #[error(transparent)]
    Package(#[from] ApplicationMaterializationError),
    #[error(transparent)]
    Graph(#[from] GraphV3Error),
}

/// Failure evidence without a caller-owned cleanup handoff. This carries no
/// native completion proof and cannot be returned as an executor result.
#[derive(Debug, thiserror::Error)]
pub enum SourceV2MaterializationDiagnostic {
    #[error("{0}")]
    Original(#[source] SourceV2MaterializationError),
    #[error("{message}; original cleanup pending: {cleanup}")]
    PendingTool { message: String, cleanup: String },
}

impl SourceV2MaterializationError {
    /// Consumes the original failure, like dropping it, to close its source
    /// handoff. The owner must still prove cleanup through its exact native job.
    pub fn into_diagnostic(self) -> SourceV2MaterializationDiagnostic {
        match self {
            Self::Package(ApplicationMaterializationError::Tool(tool))
                if tool.cleanup_pending() =>
            {
                let message = tool.to_string();
                let cleanup = std::error::Error::source(&tool)
                    .map(ToString::to_string)
                    .unwrap_or_default();
                SourceV2MaterializationDiagnostic::PendingTool { message, cleanup }
            }
            original => SourceV2MaterializationDiagnostic::Original(original),
        }
    }
}

/// Native ELF closure and Python platform checks run before packaging. These
/// checks do not execute the application or prove interpreter ABI/importability.
/// Source identity, target policy and the resulting package descriptors are bound
/// into the returned graph without letting callers substitute a second policy.
pub fn materialize_source_v2_runtime(
    source: PreparedSourceV2,
    staging_parent: &Path,
    tools: &ApplicationErofsTools,
    targets: &[LayerBuildTarget<'_>],
    dependencies: &[AdmittedRuntimeDependency<'_>],
    package_limits: ApplicationMaterializationLimits,
    runtime_limits: RuntimeVerificationLimits,
) -> Result<MaterializedSourceV2Runtime, SourceV2MaterializationError> {
    validate_dependency_bindings(dependencies)?;
    let roots: Vec<_> = dependencies
        .iter()
        .map(|dependency| dependency.root)
        .collect();
    verify_prepared_source_v2_runtime(&source, targets, &roots, runtime_limits)?;
    let packages = materialize_application_packages(source, staging_parent, tools, package_limits)?;
    let descriptors: Vec<_> = packages.descriptors().collect();
    let dependencies: Vec<_> = dependencies
        .iter()
        .map(|dependency| SourceDependencyMaterializationV3 {
            layer_name: dependency.materialization.layer_name,
            mount_point: dependency.materialization.mount_point,
            manifest: dependency.materialization.manifest,
            physical_blob: dependency.materialization.physical_blob,
        })
        .collect();
    let graph = compile_source_bundle_v2_runtime_graph(
        packages.manifest(),
        &descriptors,
        targets,
        &dependencies,
        &ApplicationPackagePolicy {
            canonicalization_profile: APPLICATION_EROFS_PROFILE_V1.into(),
            generator_identity: tools.generator_identity().into(),
        },
    )?;
    Ok(MaterializedSourceV2Runtime {
        packages,
        graph,
        dependencies: None,
    })
}

/// Compile a CLI candidate from its exact build provenance. This grants no
/// publication authority: the owner must independently qualify frozen policy,
/// bytes and dependencies before commit. The legacy target list is ignored;
/// documented runtime support is not a publication admission policy.
pub fn materialize_candidate_source_v2_runtime(
    source: PreparedSourceV2,
    staging_parent: &Path,
    tools: &ApplicationErofsTools,
    _allowed_targets: &[String],
    dependencies: &[AdmittedRuntimeDependency<'_>],
    package_limits: ApplicationMaterializationLimits,
    runtime_limits: RuntimeVerificationLimits,
) -> Result<MaterializedSourceV2Runtime, SourceV2MaterializationError> {
    let mut declared = Vec::new();
    for layer in source
        .manifest()
        .wire()
        .layers
        .iter()
        .filter(|layer| layer.target == nrz_contract::source_bundle_v2::LayerTarget::Compute)
    {
        let target = layer
            .runtime_config
            .as_ref()
            .and_then(|runtime| runtime.build_runtime_version.as_deref())
            .ok_or(SourceV2MaterializationError::Dependency(
                "CLI Compute declaration requires an exact candidate build target",
            ))?;
        declared.push((layer.name.0.clone(), target.to_owned()));
    }
    let targets: Vec<_> = declared
        .iter()
        .map(|(layer, target)| LayerBuildTarget {
            layer_name: layer,
            build_runtime_target: target,
        })
        .collect();
    materialize_source_v2_runtime(
        source,
        staging_parent,
        tools,
        &targets,
        dependencies,
        package_limits,
        runtime_limits,
    )
}

fn validate_dependency_bindings(
    dependencies: &[AdmittedRuntimeDependency<'_>],
) -> Result<(), SourceV2MaterializationError> {
    if dependencies.len() > MAX_RUNTIME_LAYERS * MAX_LAYER_DEPENDENCIES {
        return Err(SourceV2MaterializationError::Dependency("too many roots"));
    }
    for dependency in dependencies {
        let root = dependency.root;
        let descriptor = &dependency.materialization;
        let wire = descriptor.manifest.wire();
        if root.layer_name != descriptor.layer_name
            || descriptor.mount_point.strip_prefix("/output/") != Some(root.source_root)
            || wire.kind.to_string() != root.kind.as_contract_value()
        {
            return Err(SourceV2MaterializationError::Dependency(
                "root, layer or kind mismatch",
            ));
        }
        let target = if wire.compatibility.runtime_family.as_str() == "bun" {
            root.build_runtime_target.strip_prefix("bun-")
        } else {
            Some(root.build_runtime_target)
        };
        let version = wire
            .compatibility
            .runtime_version
            .as_str()
            .strip_prefix("bun-")
            .filter(|_| wire.compatibility.runtime_family.as_str() == "bun")
            .unwrap_or(wire.compatibility.runtime_version.as_str());
        if target != Some(version) {
            return Err(SourceV2MaterializationError::Dependency(
                "frozen build target mismatch",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "source_v2_materialization_tests.rs"]
mod tests;
