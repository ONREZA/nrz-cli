//! Bounded canonical JSON bytes for future artifact schema adapters.
//!
//! Schema fields, file counts, paths and domain bounds remain the adapter's responsibility.

use std::fmt;
use std::io::{self, Write};

use serde_json::Value;

#[path = "artifact_json_v1_value.rs"]
mod plain_value;

pub const PROFILE_ID: &str = "ARTIFACT_JSON_V1.0";
/// The largest container depth accepted by serde_json's default recursion limit.
pub const MAX_SUPPORTED_DEPTH: usize = 127;
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Caller-owned budgets. Scalars have depth zero; each open array/object adds one.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_depth: usize,
}

#[derive(Debug)]
pub enum Error {
    UnsupportedDepthLimit,
    ByteLimit,
    DepthLimit,
    InvalidUtf8(std::str::Utf8Error),
    InvalidJson(serde_json::Error),
    InvalidNumber,
    NonCanonical,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDepthLimit => write!(
                formatter,
                "artifact JSON depth budget exceeds {MAX_SUPPORTED_DEPTH}"
            ),
            Self::ByteLimit => formatter.write_str("artifact JSON byte budget exceeded"),
            Self::DepthLimit => formatter.write_str("artifact JSON depth budget exceeded"),
            Self::InvalidUtf8(error) => write!(formatter, "invalid artifact JSON UTF-8: {error}"),
            Self::InvalidJson(error) => write!(formatter, "invalid artifact JSON: {error}"),
            Self::InvalidNumber => {
                formatter.write_str("artifact JSON number is not a safe integer")
            }
            Self::NonCanonical => formatter.write_str("artifact JSON bytes are not canonical"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            Self::InvalidJson(error) => Some(error),
            _ => None,
        }
    }
}

/// Encode a JSON value with UTF-8 byte-sorted keys and normalized safe integers.
///
/// Float producers use serde_json's standard f64 representation. With
/// `arbitrary_precision`, spellings that cannot roundtrip through that representation
/// are rejected rather than silently rounded; arbitrary decimal syntax is unsupported.
pub fn encode(value: &Value, limits: Limits) -> Result<Vec<u8>, Error> {
    validate_limits(limits)?;
    let mut output = Output {
        writer: Vec::new(),
        written: 0,
        max_bytes: limits.max_bytes,
    };
    encode_value(value, 0, limits.max_depth, &mut output)?;
    Ok(output.writer)
}

/// Reject noncanonical spellings after byte/depth checks and native JSON parsing.
pub fn decode(bytes: &[u8], limits: Limits) -> Result<Value, Error> {
    validate_limits(limits)?;
    if bytes.len() > limits.max_bytes {
        return Err(Error::ByteLimit);
    }
    let text = std::str::from_utf8(bytes).map_err(Error::InvalidUtf8)?;
    check_depth(bytes, limits.max_depth)?;
    let value = plain_value::from_str(text).map_err(Error::InvalidJson)?;
    let mut output = Output {
        writer: Compare {
            expected: bytes,
            position: 0,
            matches: true,
        },
        written: 0,
        max_bytes: limits.max_bytes,
    };
    encode_value(&value, 0, limits.max_depth, &mut output)?;
    if !output.writer.matches || output.writer.position != bytes.len() {
        return Err(Error::NonCanonical);
    }
    Ok(value)
}

fn validate_limits(limits: Limits) -> Result<(), Error> {
    if limits.max_depth > MAX_SUPPORTED_DEPTH {
        return Err(Error::UnsupportedDepthLimit);
    }
    Ok(())
}

/// Raw byte maxima for parser allocation funding, including malformed input.
/// These statistics neither validate JSON nor authorize admission by themselves.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AllocationProfile {
    pub max_escaped_string_bytes: u64,
    pub max_unquoted_run_bytes: u64,
    pub numeric_heap_candidate: bool,
}

#[must_use]
pub fn allocation_profile(bytes: &[u8]) -> AllocationProfile {
    scan(bytes).0
}

fn check_depth(bytes: &[u8], max_depth: usize) -> Result<(), Error> {
    if scan(bytes).1 > max_depth {
        return Err(Error::DepthLimit);
    }
    Ok(())
}

fn scan(bytes: &[u8]) -> (AllocationProfile, usize) {
    let mut profile = AllocationProfile::default();
    let mut depth = 0_usize;
    let mut max_depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut string_start = 0_usize;
    let mut has_escape = false;
    let mut run = 0_u64;
    let mut numeric_run = false;
    for (index, &byte) in bytes.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
                has_escape = true;
            } else if byte == b'"' {
                if has_escape {
                    profile.max_escaped_string_bytes = profile
                        .max_escaped_string_bytes
                        .max((index - string_start) as u64);
                }
                in_string = false;
            }
        } else {
            match byte {
                b'"' => {
                    in_string = true;
                    string_start = index + 1;
                    has_escape = false;
                }
                b'[' | b'{' => {
                    depth += 1;
                    max_depth = max_depth.max(depth);
                }
                b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if matches!(
                byte,
                b'"' | b'[' | b'{' | b']' | b'}' | b',' | b':' | b' ' | b'\t' | b'\r' | b'\n'
            ) {
                profile.max_unquoted_run_bytes = profile.max_unquoted_run_bytes.max(run);
                run = 0;
            } else {
                if run == 0 {
                    numeric_run = byte == b'-' || byte.is_ascii_digit();
                }
                run += 1;
                profile.numeric_heap_candidate |= numeric_run && matches!(byte, b'.' | b'e' | b'E');
            }
        }
    }
    profile.max_unquoted_run_bytes = profile.max_unquoted_run_bytes.max(run);
    profile.numeric_heap_candidate |= profile.max_unquoted_run_bytes > 19;
    if in_string && has_escape {
        profile.max_escaped_string_bytes = profile
            .max_escaped_string_bytes
            .max((bytes.len() - string_start) as u64);
    }
    (profile, max_depth)
}

struct Output<W> {
    writer: W,
    written: usize,
    max_bytes: usize,
}

impl<W: Write> Output<W> {
    fn push(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.max_bytes - self.written {
            return Err(Error::ByteLimit);
        }
        self.writer.write_all(bytes).map_err(|_| Error::ByteLimit)?;
        self.written += bytes.len();
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), Error> {
        // Serializing a Rust string can fail only when this bounded writer rejects bytes.
        serde_json::to_writer(self, value).map_err(|_| Error::ByteLimit)
    }
}

impl<W: Write> Write for Output<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.push(bytes).map_err(io::Error::other)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Compare<'a> {
    expected: &'a [u8],
    position: usize,
    matches: bool,
}

impl Write for Compare<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self.position + bytes.len();
        self.matches &= self.expected.get(self.position..end) == Some(bytes);
        self.position = end;
        // Continue after mismatches so number validation and byte limits keep their precedence.
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode_value<W: Write>(
    value: &Value,
    depth: usize,
    max_depth: usize,
    output: &mut Output<W>,
) -> Result<(), Error> {
    match value {
        Value::Null => output.push(b"null"),
        Value::Bool(true) => output.push(b"true"),
        Value::Bool(false) => output.push(b"false"),
        Value::String(value) => output.string(value),
        Value::Number(value) => {
            let integer = if let Some(value) = value.as_i64() {
                value
            } else if let Some(value) = value.as_u64() {
                i64::try_from(value).map_err(|_| Error::InvalidNumber)?
            } else {
                let float = value.as_f64().ok_or(Error::InvalidNumber)?;
                if !float.is_finite()
                    || float.fract() != 0.0
                    || float.abs() > MAX_SAFE_INTEGER as f64
                    || serde_json::Number::from_f64(float).as_ref() != Some(value)
                {
                    return Err(Error::InvalidNumber);
                }
                float as i64
            };
            if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&integer) {
                return Err(Error::InvalidNumber);
            }
            output.push(integer.to_string().as_bytes())
        }
        Value::Array(values) => {
            if depth == max_depth {
                return Err(Error::DepthLimit);
            }
            output.push(b"[")?;
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b",")?;
                }
                encode_value(value, depth + 1, max_depth, output)?;
            }
            output.push(b"]")
        }
        Value::Object(values) => {
            if depth == max_depth {
                return Err(Error::DepthLimit);
            }
            output.push(b"{")?;
            // Explicit sorting also holds when a consumer enables serde_json/preserve_order.
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| left.as_bytes().cmp(right.as_bytes()));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index != 0 {
                    output.push(b",")?;
                }
                output.string(key)?;
                output.push(b":")?;
                encode_value(value, depth + 1, max_depth, output)?;
            }
            output.push(b"}")
        }
    }
}
