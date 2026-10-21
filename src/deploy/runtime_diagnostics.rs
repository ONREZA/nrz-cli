use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Public diagnostic facts only; preparation policy belongs to the server.
#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "Value")]
pub(super) struct PreparationLimit {
    metric: String,
    required: String,
    limit: String,
}

impl TryFrom<Value> for PreparationLimit {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let parse = || {
            let metric = value.get("metric")?.as_str()?;
            if metric.is_empty()
                || metric.len() > 64
                || !metric.as_bytes()[0].is_ascii_uppercase()
                || !metric
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
            {
                return None;
            }
            let required = value.get("required")?.as_str()?;
            let limit = value.get("limit")?.as_str()?;
            if decimal(required)? <= decimal(limit)? {
                return None;
            }
            Some(Self {
                metric: metric.to_owned(),
                required: required.to_owned(),
                limit: limit.to_owned(),
            })
        };
        parse().ok_or("invalid preparation limit diagnostic")
    }
}

fn decimal(value: &str) -> Option<u128> {
    if value.is_empty()
        || value.len() > 39
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return None;
    }
    value.parse().ok()
}

impl std::fmt::Display for PreparationLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "preparation limit: {}; required: {}; limit: {}",
            self.metric, self.required, self.limit
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(try_from = "Value", rename_all = "camelCase")]
pub(super) struct AuthorityHead {
    kind: String,
    id: String,
    generation: String,
    head_sha256: String,
}

impl TryFrom<Value> for AuthorityHead {
    type Error = &'static str;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let parse = || {
            let kind = value.get("kind")?.as_str()?;
            if !matches!(kind, "APPLICATION_PLACEMENT" | "ENVIRONMENT_POLICY") {
                return None;
            }
            let id = value.get("id")?.as_str()?;
            if id.len() != 36 || uuid::Uuid::parse_str(id).ok()?.to_string() != id {
                return None;
            }
            let generation = value.get("generation")?.as_str()?;
            if decimal(generation)? == 0 || generation.parse::<u64>().is_err() {
                return None;
            }
            let head_sha256 = value.get("headSha256")?.as_str()?;
            if head_sha256.len() != 64
                || !head_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return None;
            }
            Some(Self {
                kind: kind.to_owned(),
                id: id.to_owned(),
                generation: generation.to_owned(),
                head_sha256: head_sha256.to_owned(),
            })
        };
        parse().ok_or("invalid authority head diagnostic")
    }
}

// A malformed optional diagnostic must not erase the authoritative base failure.
pub(super) fn deserialize_optional<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}

pub(super) fn deserialize_profile_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        deserialize_optional::<D, String>(deserializer)?.filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value.as_bytes()[0].is_ascii_alphanumeric()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        }),
    )
}

fn runtime_integer<T: TryFrom<u64>>(value: &Value) -> Option<T> {
    let number = value.as_number()?;
    let integer = if let Some(integer) = number.as_u64() {
        integer
    } else {
        // Generated opaque JSON values round-trip numeric fields through f64.
        let float = number.as_f64()?;
        if !float.is_finite()
            || !(0.0..=9_007_199_254_740_991.0).contains(&float)
            || float.fract() != 0.0
        {
            return None;
        }
        float as u64
    };
    T::try_from(integer).ok()
}

pub(super) fn deserialize_runtime_integer<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: TryFrom<u64>,
{
    Ok(runtime_integer(&Value::deserialize(deserializer)?))
}

pub(super) fn deserialize_runtime_ports<'de, D>(deserializer: D) -> Result<Vec<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(runtime_integer)
        .collect())
}
