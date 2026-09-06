// @generated vendored copy of platform crates/nrz-runtime-artifact/src/launch.rs.
// Do not edit; regenerate via 'NRZ_CLI_DIR=<path> moon run workspace:sync-nrz-cli-crates'.

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
        "BUN"
    };
    let mut value = json!({ "profile": profile, "args": [], "cwd": "." });
    if let Some(readiness) = config.and_then(|value| value.get("readiness")) {
        value["readiness"] = readiness.clone();
    }
    let launch = serde_json::from_value(value)?;
    verify_runtime_launch(&launch)?;
    Ok(launch)
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
