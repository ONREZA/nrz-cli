use crate::{
    RuntimeArtifactError, RuntimeLaunchWire, RuntimeReadinessProtocol, invariant,
    verify_safe_relative_path,
};
use serde_json::{Value, json};

/// Compile existing SOURCE_BUNDLE declarations into a closed runtime profile.
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
    if let Some(intent) = &application {
        intent
            .validate_target(runtime_version)
            .map_err(RuntimeArtifactError::Invariant)?;
        if config.is_some_and(|value| {
            value.get("runtimeFamily") == Some(&json!("PYTHON"))
                || value.get("isBinaryEntry") == Some(&json!(true))
        }) {
            return invariant("application runtime conflicts with Python or binary entry");
        }
    }
    let profile = if config
        .and_then(|value| value.get("isBinaryEntry"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        "EXECUTABLE"
    } else if config
        .and_then(|value| value.get("runtimeFamily"))
        .and_then(Value::as_str)
        == Some("PYTHON")
    {
        "CPYTHON_3_14"
    } else {
        match runtime_version {
            None => "BUN",
            Some("node-22") => "NODE_22",
            Some("node-24") => "NODE_24",
            Some("node-26") => "NODE_26",
            Some(version) if version.starts_with("bun-") => "BUN",
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
    Ok(launch)
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
