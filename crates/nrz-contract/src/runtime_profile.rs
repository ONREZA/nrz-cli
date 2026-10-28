//! Immutable launch declarations do not restrict the version of the serving interpreter.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

fn number(value: &str) -> Option<u32> {
    (!value.is_empty()
        && value.len() <= 9
        && (value.len() == 1 || !value.starts_with('0'))
        && value.bytes().all(|byte| byte.is_ascii_digit()))
    .then(|| value.parse().ok())
    .flatten()
}

fn version_parts(value: &str, lengths: &[usize]) -> Option<Vec<u32>> {
    let parts = value.split('.').map(number).collect::<Option<Vec<_>>>()?;
    lengths.contains(&parts.len()).then_some(parts)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RuntimeProfile {
    Bun,
    NodeAny,
    Node(u32),
    CpythonAny,
    Cpython(u32, u32),
    Executable,
}

#[allow(non_upper_case_globals)]
impl RuntimeProfile {
    pub const Node22: Self = Self::Node(22);
    pub const Node24: Self = Self::Node(24);
    pub const Node26: Self = Self::Node(26);
    pub const Cpython312: Self = Self::Cpython(3, 12);
    pub const Cpython313: Self = Self::Cpython(3, 13);
    pub const Cpython314: Self = Self::Cpython(3, 14);

    pub fn node_build_target(target: &str) -> Option<Self> {
        if target == "node" {
            return Some(Self::NodeAny);
        }
        let parts = version_parts(target.strip_prefix("node-")?, &[1, 3])?;
        Some(Self::Node(parts[0]))
    }

    pub fn python_build_target(target: &str) -> Option<Self> {
        if target == "python" {
            return Some(Self::CpythonAny);
        }
        let parts = version_parts(target.strip_prefix("python-")?, &[2, 3])?;
        Some(Self::Cpython(parts[0], parts[1]))
    }
}

impl FromStr for RuntimeProfile {
    type Err = crate::runtime_artifact_graph::error::ConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "BUN" => Ok(Self::Bun),
            "NODE" => Ok(Self::NodeAny),
            "CPYTHON" => Ok(Self::CpythonAny),
            "EXECUTABLE" => Ok(Self::Executable),
            _ => {
                if let Some(major) = value.strip_prefix("NODE_").and_then(number) {
                    return Ok(Self::Node(major));
                }
                if let Some((major, minor)) = value
                    .strip_prefix("CPYTHON_")
                    .and_then(|version| version.split_once('_'))
                    && let (Some(major), Some(minor)) = (number(major), number(minor))
                {
                    return Ok(Self::Cpython(major, minor));
                }
                Err("invalid runtime profile syntax".into())
            }
        }
    }
}

impl TryFrom<&str> for RuntimeProfile {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for RuntimeProfile {
    type Error = <Self as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl fmt::Display for RuntimeProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bun => formatter.write_str("BUN"),
            Self::NodeAny => formatter.write_str("NODE"),
            Self::CpythonAny => formatter.write_str("CPYTHON"),
            Self::Node(major) => write!(formatter, "NODE_{major}"),
            Self::Cpython(major, minor) => write!(formatter, "CPYTHON_{major}_{minor}"),
            Self::Executable => formatter.write_str("EXECUTABLE"),
        }
    }
}

impl Serialize for RuntimeProfile {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let value = self.to_string();
        value.parse::<Self>().map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&value)
    }
}

impl<'de> Deserialize<'de> for RuntimeProfile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[path = "runtime_profile_tests.rs"]
mod tests;
