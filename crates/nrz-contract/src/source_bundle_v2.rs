//! Rust-owned SOURCE_BUNDLE_V2 wire model. Cross-field and admission checks belong
//! to nrz-source-bundle; schema annotations describe the public structural contract.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

pub(crate) fn optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
        #[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
        pub enum $name { $(#[serde(rename = $wire)] $variant),+ }
    };
}

string_enum!(SchemaVersion { V2 => "SOURCE_BUNDLE_V2.0" });
string_enum!(Capability { ComputeNetworkV1 => "COMPUTE_NETWORK_V1" });
string_enum!(DigestAlgorithm { Blake3 => "BLAKE3" });
string_enum!(FileRole { Static => "static", Compute => "compute", Prerender => "prerender", Dependency => "dependency", Config => "config" });
string_enum!(LayerTarget { Static => "STATIC", Compute => "COMPUTE" });
string_enum!(ApplicationRuntimeFamily { Bun => "BUN", Node => "NODE", Python => "PYTHON", Executable => "EXECUTABLE" });
string_enum!(NetworkSchemaVersion { V1 => "COMPUTE_NETWORK_V1" });
string_enum!(NetworkExposure { Public => "PUBLIC", Private => "PRIVATE" });
string_enum!(NetworkProtocol { Http => "http", Grpc => "grpc", Websocket => "websocket", Tcp => "tcp" });
string_enum!(PortBinding { Primary => "primary", Fixed => "fixed" });
string_enum!(NetworkReadinessProtocol { Tcp => "tcp", Http => "http" });
string_enum!(HttpMethod { Get => "GET", Post => "POST", Put => "PUT", Delete => "DELETE", Patch => "PATCH", Head => "HEAD", Options => "OPTIONS" });

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(
    feature = "codegen",
    schemars(crate = "schemars08", title = "ONREZA Source Bundle V2")
)]
pub struct OnrezaSourceBundleV2 {
    pub schema_version: SchemaVersion,
    pub capabilities: Vec<Capability>,
    pub files: Vec<FileEntry>,
    #[cfg_attr(feature = "codegen", schemars(length(max = 10)))]
    pub layers: Vec<Layer>,
    #[cfg_attr(feature = "codegen", schemars(length(max = 200)))]
    pub routes: Vec<Route>,
    pub entrypoints: Vec<NonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "entryType", rename_all = "lowercase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum FileEntry {
    File {
        path: NonEmptyString,
        role: FileRole,
        #[serde(
            rename = "layerName",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        layer_name: Option<LayerName>,
        #[serde(
            rename = "contentType",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        content_type: Option<String>,
        #[serde(rename = "blobDigest")]
        blob_digest: BlobDigest,
        #[cfg_attr(
            feature = "codegen",
            schemars(range(min = 0, max = 9007199254740991u64))
        )]
        size: u64,
        executable: bool,
    },
    Symlink {
        path: NonEmptyString,
        role: FileRole,
        #[serde(
            rename = "layerName",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        layer_name: Option<LayerName>,
        #[serde(
            rename = "contentType",
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        content_type: Option<String>,
        #[serde(rename = "linkTarget")]
        link_target: NonEmptyString,
        #[cfg_attr(feature = "codegen", schemars(range(min = 0, max = 0)))]
        size: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct BlobDigest {
    pub algorithm: DigestAlgorithm,
    #[cfg_attr(feature = "codegen", schemars(regex(pattern = "^[0-9a-f]{64}$")))]
    pub value: String,
}

macro_rules! string_type {
    ($name:ident, $($constraint:meta),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
        #[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
        pub struct $name(#[cfg_attr(feature = "codegen", schemars($($constraint),+))] pub String);
    };
}

string_type!(NonEmptyString, length(min = 1));
string_type!(
    LayerName,
    length(min = 1, max = 64),
    regex(pattern = r"^[^\x00-\x1f\x7f-\x9f]*$")
);
string_type!(
    PortName,
    length(min = 1, max = 64),
    regex(pattern = "^[a-zA-Z0-9][a-zA-Z0-9_-]*$")
);
string_type!(
    RuntimeArgument,
    length(max = 4096),
    regex(pattern = r"^[^\u0000]*$")
);
string_type!(ConditionName, length(min = 1, max = 64));
string_type!(ConditionValue, length(max = 512));

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct Layer {
    pub name: LayerName,
    pub target: LayerTarget,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub root_path: Option<NonEmptyString>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub entrypoint: Option<NonEmptyString>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub runtime_config: Option<RuntimeConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct RuntimeConfig {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub network: Option<Network>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(
        feature = "codegen",
        schemars(regex(
            pattern = r"^(BUN:CURRENT|NODE:[1-9][0-9]{0,8}|PYTHON:[1-9][0-9]{0,8}\.(0|[1-9][0-9]{0,8}))$"
        ))
    )]
    pub runtime_policy_lineage: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(
        feature = "codegen",
        schemars(range(min = 1, max = 9007199254740991u64))
    )]
    pub timeout_ms: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(feature = "codegen", schemars(range(min = 32, max = 8192)))]
    pub memory_mb: Option<u64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub application_runtime: Option<ApplicationRuntime>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(
        feature = "codegen",
        schemars(regex(
            pattern = r"^(bun-1\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)|node-(22|24|26)|python-3\.(12|13|14)|native-linux-x86_64-glibc)$"
        ))
    )]
    pub build_runtime_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct ApplicationRuntime {
    pub family: ApplicationRuntimeFamily,
    #[cfg_attr(feature = "codegen", schemars(length(max = 64)))]
    pub args: Vec<RuntimeArgument>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct Network {
    pub schema_version: NetworkSchemaVersion,
    pub exposure: NetworkExposure,
    #[cfg_attr(feature = "codegen", schemars(length(min = 1, max = 16)))]
    pub ports: Vec<NetworkPort>,
    pub readiness: NetworkReadiness,
    #[cfg_attr(feature = "codegen", schemars(length(max = 128)))]
    pub allow_from: Vec<NetworkAllowFrom>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct NetworkPort {
    pub name: PortName,
    pub protocol: NetworkProtocol,
    #[cfg_attr(feature = "codegen", schemars(range(min = 1, max = 65535)))]
    pub port: u16,
    pub binding: PortBinding,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct NetworkReadiness {
    pub protocol: NetworkReadinessProtocol,
    pub port: PortName,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(feature = "codegen", schemars(length(min = 1, max = 2048)))]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct NetworkAllowFrom {
    #[cfg_attr(
        feature = "codegen",
        schemars(schema_with = "crate::artifact_schemas::service_id")
    )]
    pub service_id: String,
    pub layer_name: LayerName,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub struct Route {
    pub pattern: NonEmptyString,
    pub layer_name: LayerName,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(feature = "codegen", schemars(range(min = -2147483648i64, max = 2147483647)))]
    pub priority: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    pub methods: Option<Vec<HttpMethod>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(
        feature = "codegen",
        schemars(schema_with = "crate::artifact_schemas::headers")
    )]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "optional_non_null"
    )]
    #[cfg_attr(feature = "codegen", schemars(length(min = 1, max = 16)))]
    pub fallthrough_when: Option<Vec<FallthroughCondition>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
#[cfg_attr(feature = "codegen", derive(schemars08::JsonSchema))]
#[cfg_attr(feature = "codegen", schemars(crate = "schemars08"))]
pub enum FallthroughCondition {
    Header {
        name: ConditionName,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        value: Option<ConditionValue>,
    },
    Query {
        name: ConditionName,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "optional_non_null"
        )]
        value: Option<ConditionValue>,
    },
}
