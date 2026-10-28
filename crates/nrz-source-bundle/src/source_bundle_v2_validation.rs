use std::collections::HashMap;

use nrz_contract::{SourceBundleV2ManifestWire, source_bundle_v2 as wire};

use super::{
    MAX_SAFE_INTEGER, SourceBundleV2ManifestBudget, SourceBundleV2ManifestError,
    SourceBundleV2ManifestSummary,
};
use crate::path_graph::{PathWorkBudget, PathWorkError};
use crate::{SourceArchivePathIndex, SourceLogicalManifestLayer};

#[derive(Debug)]
pub(super) enum ValidationError {
    Manifest(SourceBundleV2ManifestError),
    WorkLimit,
}

impl From<SourceBundleV2ManifestError> for ValidationError {
    fn from(error: SourceBundleV2ManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl From<serde_json::Error> for ValidationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Manifest(error.into())
    }
}

impl From<PathWorkError> for ValidationError {
    fn from(cause: PathWorkError) -> Self {
        match cause {
            PathWorkError::Exhausted => Self::WorkLimit,
            PathWorkError::Invalid(cause) => error(cause.to_string()),
        }
    }
}

type Result<T> = std::result::Result<T, ValidationError>;

pub(super) fn validate(
    manifest: &SourceBundleV2ManifestWire,
    budget: SourceBundleV2ManifestBudget,
) -> std::result::Result<SourceBundleV2ManifestSummary, SourceBundleV2ManifestError> {
    validate_with_work(manifest, budget, &mut PathWorkBudget::unlimited()).map_err(|error| {
        match error {
            ValidationError::Manifest(error) => error,
            ValidationError::WorkLimit => unreachable!("unlimited path work cannot be exhausted"),
        }
    })
}

pub(super) fn validate_with_work(
    manifest: &SourceBundleV2ManifestWire,
    budget: SourceBundleV2ManifestBudget,
    work: &mut PathWorkBudget,
) -> Result<SourceBundleV2ManifestSummary> {
    if [
        budget.max_file_count,
        budget.max_logical_bytes,
        budget.max_static_file_bytes,
    ]
    .into_iter()
    .any(|limit| limit > MAX_SAFE_INTEGER)
    {
        return invalid("caller budgets must be nonnegative safe integers");
    }
    validate_metadata(
        &manifest.layers,
        &manifest.routes,
        &manifest.capabilities,
        &manifest.entrypoints,
    )?;
    let mut summary = SourceBundleV2ManifestSummary {
        file_count: manifest.files.len() as u64,
        logical_static_bytes: 0,
        artifact_size_bytes: 0,
        max_static_file_size_bytes: 0,
    };
    if summary.file_count > budget.max_file_count {
        return invalid("file count exceeds caller budget");
    }
    let mut previous: Option<&str> = None;
    let mut entries = Vec::with_capacity(manifest.files.len());
    let mut by_path = HashMap::with_capacity(manifest.files.len());
    let mut logical_bytes = 0_u64;
    for file in &manifest.files {
        let (path, role, layer_name, size) = file_fields(file);
        source_path(path)?;
        if previous.is_some_and(|previous| previous.as_bytes() >= path.as_bytes()) {
            return invalid("files must be strictly ordered by UTF-8 path bytes");
        }
        previous = Some(path);
        logical_bytes = logical_bytes
            .checked_add(size)
            .filter(|size| *size <= MAX_SAFE_INTEGER)
            .ok_or_else(|| error("logical file sizes exceed the safe integer range"))?;
        if logical_bytes > budget.max_logical_bytes {
            return invalid("logical file sizes exceed caller budget");
        }
        if role == &wire::FileRole::Static {
            summary.logical_static_bytes += size;
        } else {
            summary.artifact_size_bytes += size;
        }
        if matches!(
            role,
            wire::FileRole::Static | wire::FileRole::Prerender | wire::FileRole::Config
        ) {
            summary.max_static_file_size_bytes = summary.max_static_file_size_bytes.max(size);
            if size > budget.max_static_file_bytes {
                return invalid("static file size exceeds caller budget");
            }
        }
        let target = match file {
            wire::FileEntry::File { blob_digest, .. } => {
                let digest = &blob_digest.value;
                if digest.len() != 64
                    || !digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return invalid(
                        "file blobDigest value must be 64 lowercase hexadecimal digits",
                    );
                }
                None
            }
            wire::FileEntry::Symlink { link_target, .. } => {
                if size != 0 {
                    return invalid("symlink size must be zero");
                }
                Some(link_target.0.as_str())
            }
        };
        entries.push((path, target));
        by_path.insert(path, (role, layer_name));
    }
    let index = SourceArchivePathIndex::from_entries_with_budget(entries.iter().copied(), work)?;
    for (path, target) in entries {
        if target.is_some() {
            index.validate_symlink_with_budget(path, work)?;
        }
    }
    let by_layer: HashMap<_, _> = manifest
        .layers
        .iter()
        .map(|layer| (layer.name.0.as_str(), layer))
        .collect();
    for layer in &manifest.layers {
        if layer.target == wire::LayerTarget::Compute {
            let name = &layer.name.0;
            let entrypoint = layer.entrypoint.as_ref().unwrap().0.as_str();
            let (role, owner) = by_path
                .get(entrypoint)
                .copied()
                .ok_or_else(|| error("COMPUTE entrypoint is absent from files"))?;
            if role != &wire::FileRole::Compute || owner != Some(name.as_str()) {
                return invalid("COMPUTE entrypoint must be owned by its COMPUTE layer");
            }
        }
    }
    for (role, owner) in by_path.values().copied() {
        let owner = owner
            .map(|name| {
                by_layer
                    .get(name)
                    .copied()
                    .ok_or_else(|| error("file references an unknown layer"))
            })
            .transpose()?;
        if matches!(role, wire::FileRole::Compute | wire::FileRole::Dependency)
            && owner.is_none_or(|layer| layer.target != wire::LayerTarget::Compute)
        {
            return invalid("compute/dependency file requires a COMPUTE owner");
        }
        if role == &wire::FileRole::Prerender && owner.is_none() {
            return invalid("prerender file requires a layer owner");
        }
    }
    Ok(summary)
}

pub(super) fn validate_metadata(
    layers: &[wire::Layer],
    routes: &[wire::Route],
    capabilities: &[wire::Capability],
    entrypoints: &[wire::NonEmptyString],
) -> std::result::Result<(), SourceBundleV2ManifestError> {
    metadata(layers, routes, capabilities, entrypoints).map_err(|cause| match cause {
        ValidationError::Manifest(cause) => cause,
        ValidationError::WorkLimit => unreachable!("metadata validation has no inventory work"),
    })
}
fn metadata(
    layers: &[wire::Layer],
    routes: &[wire::Route],
    capabilities: &[wire::Capability],
    entrypoints: &[wire::NonEmptyString],
) -> Result<()> {
    if layers.len() > 10 || routes.len() > 200 || capabilities.len() > 1 {
        return invalid("layer, route or capability count exceeds its supported bound");
    }
    let mut by_layer = HashMap::with_capacity(layers.len());
    for layer in layers {
        let name = &layer.name.0;
        if name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
            || by_layer.insert(name.as_str(), layer).is_some()
        {
            return invalid("invalid or duplicate layer name");
        }
        if let Some(root) = &layer.root_path {
            source_path(&root.0)?;
        }
        validate_runtime_config(layer.runtime_config.as_ref())?;
        match layer.target {
            wire::LayerTarget::Static
                if layer.entrypoint.is_some()
                    || layer.runtime_config.as_ref().is_some_and(|config| {
                        config.application_runtime.is_some() || config.network.is_some()
                    }) =>
            {
                return invalid("STATIC layer declares executable launch fields");
            }
            wire::LayerTarget::Static => {}
            wire::LayerTarget::Compute => {
                if layer.runtime_config.as_ref().is_none_or(|config| {
                    config.application_runtime.is_none() || config.network.is_none()
                }) {
                    return invalid("COMPUTE layer requires applicationRuntime and network");
                }
                let entrypoint = layer
                    .entrypoint
                    .as_ref()
                    .map(|entrypoint| entrypoint.0.as_str())
                    .ok_or_else(|| error("COMPUTE layer requires an entrypoint"))?;
                source_path(entrypoint)?;
                if let Some(root) = &layer.root_path
                    && !entrypoint
                        .strip_prefix(root.0.as_str())
                        .is_some_and(|suffix| suffix.starts_with('/'))
                {
                    return invalid("COMPUTE entrypoint is outside layer rootPath");
                }
            }
        }
    }
    if !entrypoints
        .iter()
        .map(|entrypoint| entrypoint.0.as_str())
        .eq(layers
            .iter()
            .filter(|layer| layer.target == wire::LayerTarget::Compute)
            .filter_map(|layer| {
                layer
                    .entrypoint
                    .as_ref()
                    .map(|entrypoint| entrypoint.0.as_str())
            }))
    {
        return invalid("entrypoints must equal COMPUTE entrypoints in layer order");
    }
    for route in routes {
        if !by_layer.contains_key(route.layer_name.0.as_str()) {
            return invalid("route references an unknown layer");
        }
        validate_route(route)?;
    }
    // Reuse V1 launch checks for at most ten layers; file and route metadata stay borrowed.
    let layers: Vec<SourceLogicalManifestLayer> = layers
        .iter()
        .map(|layer| serde_json::from_value(serde_json::to_value(layer)?))
        .collect::<std::result::Result<_, _>>()?;
    let capabilities: Vec<_> = capabilities
        .iter()
        .map(|_| crate::COMPUTE_NETWORK_SCHEMA_VERSION.to_string())
        .collect();
    crate::network::validate_source_network_layers(
        &capabilities,
        &layers,
        routes.iter().map(|route| route.layer_name.0.as_str()),
    )
    .map_err(error)?;
    crate::application_runtime::source_application_runtime_layers(&layers).map_err(error)?;
    crate::manifest::validate_source_route_header_maps(
        routes.iter().filter_map(|route| route.headers.as_ref()),
    )
    .map_err(error)?;
    Ok(())
}

fn file_fields(file: &wire::FileEntry) -> (&str, &wire::FileRole, Option<&str>, u64) {
    let (wire::FileEntry::File {
        path,
        role,
        layer_name,
        size,
        ..
    }
    | wire::FileEntry::Symlink {
        path,
        role,
        layer_name,
        size,
        ..
    }) = file;
    (
        &path.0,
        role,
        layer_name.as_ref().map(|name| name.0.as_str()),
        *size,
    )
}

fn validate_runtime_config(config: Option<&wire::RuntimeConfig>) -> Result<()> {
    if let Some(config) = config {
        for (field, value, minimum, maximum) in [
            ("memoryMb", config.memory_mb, 32, 8192),
            ("timeoutMs", config.timeout_ms, 1, MAX_SAFE_INTEGER),
        ] {
            if value.is_some_and(|value| !(minimum..=maximum).contains(&value)) {
                return invalid(format!("runtimeConfig.{field} exceeds its supported bound"));
            }
        }
        if let Some(target) = config.build_runtime_version.as_deref()
            && !crate::application_runtime::supported_bun_build_target(target)
            && target != "native-linux-x86_64-glibc"
            && nrz_contract::RuntimeProfile::node_build_target(target).is_none()
            && nrz_contract::RuntimeProfile::python_build_target(target).is_none()
        {
            return invalid("unsupported buildRuntimeVersion syntax");
        }
        if let Some(lineage) = config.runtime_policy_lineage.as_deref()
            && !valid_runtime_policy_lineage(lineage)
        {
            return invalid("unsupported runtimePolicyLineage syntax");
        }
    }
    Ok(())
}

pub fn valid_runtime_policy_lineage(lineage: &str) -> bool {
    fn version_part(part: &str) -> bool {
        (1..=9).contains(&part.len())
            && !part.starts_with('0')
            && part.bytes().all(|byte| byte.is_ascii_digit())
    }
    lineage == "BUN:CURRENT"
        || lineage.strip_prefix("NODE:").is_some_and(version_part)
        || lineage.strip_prefix("PYTHON:").is_some_and(|version| {
            version.split_once('.').is_some_and(|(major, minor)| {
                version_part(major) && (minor == "0" || version_part(minor))
            })
        })
}

fn validate_route(route: &wire::Route) -> Result<()> {
    if route.pattern.0.is_empty() || route.layer_name.0.chars().count() > 64 {
        return invalid("invalid route pattern or layerName");
    }
    if route.headers.as_ref().is_some_and(|headers| {
        headers.keys().any(|name| {
            !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^`|~".contains(&byte))
        })
    }) {
        return invalid("route header name uses an unsupported character");
    }
    if let Some(conditions) = &route.fallthrough_when {
        if conditions.is_empty() || conditions.len() > 16 {
            return invalid("route fallthrough count is invalid");
        }
        for condition in conditions {
            let (name, value) = match condition {
                wire::FallthroughCondition::Header { name, value }
                | wire::FallthroughCondition::Query { name, value } => {
                    (&name.0, value.as_ref().map(|value| &value.0))
                }
            };
            if name.is_empty()
                || name.encode_utf16().count() > 64
                || value.is_some_and(|value| value.encode_utf16().count() > 512)
            {
                return invalid("route fallthrough name or value exceeds its bound");
            }
        }
    }
    Ok(())
}

fn source_path(path: &str) -> Result<()> {
    crate::normalize_source_path(path).map_err(error)?;
    if path == ".__onreza" || path.starts_with(".__onreza/") {
        return invalid("reserved source metadata namespace");
    }
    Ok(())
}

fn error(message: impl Into<String>) -> ValidationError {
    SourceBundleV2ManifestError::Invalid(message.into()).into()
}

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(error(message))
}
