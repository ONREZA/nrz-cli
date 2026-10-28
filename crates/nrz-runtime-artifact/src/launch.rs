use crate::{
    RuntimeArtifactError, RuntimeLaunchWire, RuntimeProfile, RuntimeReadinessProtocol, invariant,
    verify_safe_relative_path,
};
use serde_json::{Value, json};

/// Compile existing SOURCE_BUNDLE declarations into an immutable runtime profile.
/// No host command, shell expansion or environment secrets enter this artifact.
pub fn source_layer_launch(
    config: Option<&Value>,
) -> Result<RuntimeLaunchWire, RuntimeArtifactError> {
    source_layer_launch_for_target(config, None)
}

/// Compile against an independently admitted target, never the source's build witness.
pub fn source_layer_launch_for_target(
    config: Option<&Value>,
    runtime_version: Option<&str>,
) -> Result<RuntimeLaunchWire, RuntimeArtifactError> {
    nrz_source_bundle::validate_build_runtime_version(config, runtime_version)
        .map_err(RuntimeArtifactError::Invariant)?;
    let application = nrz_source_bundle::layer_application_runtime(config)
        .map_err(RuntimeArtifactError::Invariant)?;
    // An absent compiler witness carries only the explicit language family.
    // A present witness was checked against independent custody above.
    let runtime_version =
        runtime_version.or_else(|| match application.as_ref().map(|intent| intent.family) {
            Some(nrz_source_bundle::ApplicationRuntimeFamily::Node) => Some("node"),
            Some(nrz_source_bundle::ApplicationRuntimeFamily::Python) => Some("python"),
            _ if config
                .and_then(|config| config.get("runtimeFamily"))
                .and_then(Value::as_str)
                == Some("PYTHON") =>
            {
                Some("python")
            }
            _ => None,
        });
    if let Some(intent) = &application {
        intent
            .validate_target(runtime_version)
            .map_err(RuntimeArtifactError::Invariant)?;
    }
    let profile = if application.as_ref().is_some_and(|intent| {
        intent.family == nrz_source_bundle::ApplicationRuntimeFamily::Executable
    }) || config
        .and_then(|value| value.get("isBinaryEntry"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        "EXECUTABLE".to_owned()
    } else if application
        .as_ref()
        .is_some_and(|intent| intent.family == nrz_source_bundle::ApplicationRuntimeFamily::Python)
        || config
            .and_then(|value| value.get("runtimeFamily"))
            .and_then(Value::as_str)
            == Some("PYTHON")
    {
        runtime_version
            .and_then(RuntimeProfile::python_build_target)
            .ok_or_else(|| {
                RuntimeArtifactError::Invariant(
                    "Python layer requires a canonical frozen target".into(),
                )
            })?
            .to_string()
    } else {
        match runtime_version {
            None => "BUN".to_owned(),
            Some(version) if RuntimeProfile::node_build_target(version).is_some() => {
                RuntimeProfile::node_build_target(version)
                    .ok_or_else(|| {
                        RuntimeArtifactError::Invariant("invalid Node build target syntax".into())
                    })?
                    .to_string()
            }
            Some(version)
                if nrz_source_bundle::application_runtime::supported_bun_build_target(version) =>
            {
                "BUN".to_owned()
            }
            Some(version) => {
                return invariant(format!("unsupported JavaScript runtime target '{version}'"));
            }
        }
    };
    let mut value = json!({ "profile": profile, "args": [], "cwd": "." });
    if let Some(application) = application {
        value["args"] = json!(application.args);
    }
    if let Some(readiness) = config.and_then(|value| value.get("readiness")) {
        value["readiness"] = readiness.clone();
    }
    let launch = serde_json::from_value(value)?;
    verify_runtime_launch(&launch)?;
    if let Some(family) = config
        .and_then(|config| config.get("runtimeFamily"))
        .and_then(Value::as_str)
    {
        verify_runtime_launch_family(launch.profile, family)?;
    }
    Ok(launch)
}

pub(crate) fn verify_runtime_launch_family(
    profile: RuntimeProfile,
    family: &str,
) -> Result<(), RuntimeArtifactError> {
    use nrz_source_bundle::ApplicationRuntimeFamily as Family;
    let application_family = match profile {
        RuntimeProfile::Bun => Family::Bun,
        RuntimeProfile::NodeAny | RuntimeProfile::Node(_) => Family::Node,
        RuntimeProfile::CpythonAny | RuntimeProfile::Cpython(_, _) => Family::Python,
        RuntimeProfile::Executable => Family::Executable,
    };
    if !application_family.matches_legacy_runtime_family(family) {
        return invariant("runtime profile conflicts with runtimeFamily");
    }
    Ok(())
}

/// Project source configuration after launch declarations have been consumed.
pub fn source_layer_runtime_config(config: Option<&Value>) -> Result<Value, RuntimeArtifactError> {
    let mut config = config.cloned().unwrap_or_else(|| json!({}));
    let object = config.as_object_mut().ok_or_else(|| {
        RuntimeArtifactError::Invariant("source runtimeConfig must be an object".into())
    })?;
    for key in [
        nrz_source_bundle::RUNTIME_READINESS_CONFIG_KEY,
        "isBinaryEntry",
        nrz_source_bundle::APPLICATION_RUNTIME_CONFIG_KEY,
        "buildRuntimeVersion",
    ] {
        object.remove(key);
    }
    Ok(config)
}

pub fn verify_runtime_launch(launch: &RuntimeLaunchWire) -> Result<(), RuntimeArtifactError> {
    verify_safe_relative_path("runtime launch cwd", launch.cwd.as_str())?;
    if launch.args.len() > 64 || launch.args.iter().any(|arg| arg.as_str().contains('\0')) {
        return invariant("runtime launch arguments are invalid");
    }
    if let Some(readiness) = &launch.readiness {
        let path = readiness.path.as_ref().map(|path| path.as_str());
        let valid = match readiness.protocol {
            RuntimeReadinessProtocol::Tcp => path.is_none(),
            RuntimeReadinessProtocol::Http => {
                path.is_some_and(|path| path.starts_with('/') && !path.contains(['\r', '\n', '\0']))
            }
        };
        if !valid {
            return invariant("invalid runtime readiness path");
        }
    }
    Ok(())
}

/// Build-minor provenance carried by a closed launch profile. Execution revision
/// authorization and successor compatibility remain separate owner decisions.
pub fn python_minor_for_profile(profile: RuntimeProfile) -> Option<nrz_source_bundle::PythonMinor> {
    match profile {
        RuntimeProfile::Cpython(major, minor) => {
            nrz_source_bundle::PythonMinor::from_version(&format!("{major}.{minor}"))
        }
        _ => None,
    }
}
