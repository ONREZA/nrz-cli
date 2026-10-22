use std::{fmt, mem::size_of};

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

use super::{SourceBundleVerificationFailure, failure};
use crate::manifest::{
    SourceLogicalManifestFile, SourceLogicalManifestLayer, SourceLogicalManifestRoute,
};
use crate::path_graph::{MAX_SYMLINK_EXPANSIONS, PATH_INDEX_CHILD_BYTES, PATH_INDEX_NODE_BYTES};

#[derive(Default)]
struct Memory {
    heap: u64,
    strings: u64,
    paths: u64,
    path_components: u64,
    max_path_bytes: u64,
    max_path_components: u64,
    symlinks: bool,
    slots: u64,
    array_len: u64,
    string_len: u64,
    string_slashes: u64,
    files: u64,
    layers: u64,
    routes: u64,
}

impl Memory {
    fn merge(&mut self, child: Self) {
        self.heap = self.heap.saturating_add(child.heap);
        self.strings = self.strings.saturating_add(child.strings);
        self.paths = self.paths.saturating_add(child.paths);
        self.path_components = self.path_components.saturating_add(child.path_components);
        self.max_path_bytes = self.max_path_bytes.max(child.max_path_bytes);
        self.max_path_components = self.max_path_components.max(child.max_path_components);
        self.symlinks |= child.symlinks;
        self.slots = self.slots.saturating_add(child.slots);
    }

    fn field(&mut self, key: &str, child: Self, root: bool) {
        if root {
            match key {
                "files" => self.files = child.array_len,
                "layers" => self.layers = child.array_len,
                "routes" => self.routes = child.array_len,
                _ => {}
            }
        }
        if key == "path" || key == "linkTarget" {
            self.paths = self.paths.saturating_add(child.string_len);
            self.path_components = self
                .path_components
                .saturating_add(child.string_slashes + 1);
            self.max_path_bytes = self.max_path_bytes.max(child.string_len);
            self.max_path_components = self.max_path_components.max(child.string_slashes + 1);
            self.symlinks |= key == "linkTarget";
        }
        self.merge(child);
    }

    fn forecast(self, wire_bytes: u64) -> u64 {
        let files = self.files;
        let directories = self.path_components.saturating_add(1);
        let graph_nodes = files.saturating_add(directories);
        // Value plus typed runtimeConfig, including their strings and collection storage.
        let json = self.heap.saturating_mul(2);
        let typed = vector_bytes(files, size_of::<SourceLogicalManifestFile>())
            .saturating_add(vector_bytes(
                self.layers,
                size_of::<SourceLogicalManifestLayer>(),
            ))
            .saturating_add(vector_bytes(
                self.routes,
                size_of::<SourceLogicalManifestRoute>(),
            ));
        let indices = hash_bytes(files, size_of::<(String, SourceLogicalManifestFile)>())
            .saturating_add(hash_bytes(files, size_of::<String>()))
            .saturating_add(vector_bytes(graph_nodes, PATH_INDEX_NODE_BYTES))
            .saturating_add(hash_bytes(graph_nodes, PATH_INDEX_CHILD_BYTES));
        // Symlink traversal can revisit a target up to the shared graph expansion limit.
        let traversal = if self.symlinks {
            let components = self
                .max_path_components
                .saturating_mul(MAX_SYMLINK_EXPANSIONS as u64);
            let visited = graph_nodes.min(components.saturating_add(MAX_SYMLINK_EXPANSIONS as u64));
            self.max_path_bytes
                .saturating_mul(MAX_SYMLINK_EXPANSIONS as u64)
                .saturating_add(vector_bytes(components, size_of::<&str>()))
                .saturating_add(vector_bytes(visited, size_of::<&str>()))
                .saturating_add(hash_bytes(visited, size_of::<&str>()))
        } else {
            0
        };
        let paths = self.paths.saturating_mul(3).saturating_add(traversal);
        // Canonical serializer retains child strings, their joined body, and its result.
        let canonical = wire_bytes
            .saturating_mul(4)
            .saturating_add(vector_bytes(self.slots, size_of::<String>()));
        json.saturating_add(typed)
            .saturating_add(indices)
            .saturating_add(paths)
            .saturating_add(canonical)
            .saturating_add(self.strings.saturating_mul(3))
    }
}

fn vector_bytes(len: u64, element_bytes: usize) -> u64 {
    if len == 0 {
        return 0;
    }
    // Vec growth can temporarily retain the previous allocation during reallocation.
    len.checked_next_power_of_two()
        .unwrap_or(u64::MAX)
        .max(4)
        .saturating_mul(element_bytes as u64)
        .saturating_mul(2)
}

fn hash_bytes(len: u64, element_bytes: usize) -> u64 {
    if len == 0 {
        return 0;
    }
    // SwissTable uses a 7/8 load factor, power-of-two buckets, and a control byte per bucket.
    let buckets = len
        .saturating_mul(8)
        .checked_div(7)
        .unwrap_or(u64::MAX)
        .saturating_add(1)
        .checked_next_power_of_two()
        .unwrap_or(u64::MAX)
        .max(8);
    buckets
        .saturating_mul(element_bytes as u64 + 1)
        .saturating_add(16)
        .saturating_mul(2)
}

fn object_bytes(entries: u64) -> u64 {
    if entries == 0 {
        return 0;
    }
    // Pinned Rust's BTreeMap has 11 key/value slots, 12 edges, and >=5 keys per non-root node.
    let node = 11 * (size_of::<String>() + size_of::<Value>()) + 14 * size_of::<usize>();
    entries
        .saturating_div(5)
        .saturating_add(1)
        .saturating_mul(node as u64)
}

struct MemoryVisitor {
    root: bool,
}

impl<'de> Deserialize<'de> for Memory {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(MemoryVisitor { root: false })
    }
}

impl<'de> Visitor<'de> for MemoryVisitor {
    type Value = Memory;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, _: bool) -> Result<Memory, E> {
        Ok(Memory::default())
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<Memory, E> {
        Ok(Memory::default())
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<Memory, E> {
        Ok(Memory::default())
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<Memory, E> {
        Ok(Memory::default())
    }
    fn visit_unit<E: de::Error>(self) -> Result<Memory, E> {
        Ok(Memory::default())
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Memory, E> {
        Ok(Memory {
            heap: 2 * value.len() as u64,
            strings: value.len() as u64,
            string_len: value.len() as u64,
            string_slashes: value.bytes().filter(|byte| *byte == b'/').count() as u64,
            ..Memory::default()
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Memory, A::Error> {
        let mut memory = Memory::default();
        while let Some(child) = seq.next_element::<Memory>()? {
            memory.array_len += 1;
            memory.slots += 1;
            memory.merge(child);
        }
        memory.heap = memory
            .heap
            .saturating_add(vector_bytes(memory.array_len, size_of::<Value>()));
        Ok(memory)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Memory, A::Error> {
        let mut memory = Memory::default();
        let mut entries = 0_u64;
        while let Some(key) = map.next_key::<String>()? {
            entries += 1;
            memory.slots += 1;
            memory.heap = memory.heap.saturating_add(2 * key.len() as u64);
            memory.strings = memory.strings.saturating_add(key.len() as u64);
            memory.field(&key, map.next_value::<Memory>()?, self.root);
        }
        memory.heap = memory.heap.saturating_add(object_bytes(entries));
        Ok(memory)
    }
}

/// Forecast Value, typed manifest, canonical JSON, and path-validation storage without materializing them.
/// The caller separately reserves four times the raw body's length for bytes and serde scratch/keys.
pub fn source_bundle_verification_manifest_memory_bytes(
    body: &[u8],
) -> Result<u64, SourceBundleVerificationFailure> {
    let mut deserializer = serde_json::Deserializer::from_slice(body);
    let memory = deserializer
        .deserialize_any(MemoryVisitor { root: true })
        .and_then(|memory| {
            deserializer.end()?;
            Ok(memory)
        })
        .map_err(|error| {
            failure(
                "SOURCE_LOGICAL_MANIFEST_INVALID",
                format!("Embedded logical manifest is invalid JSON: {error}"),
            )
        })?;
    checked_forecast(memory.forecast(body.len() as u64))
}

fn checked_forecast(bytes: u64) -> Result<u64, SourceBundleVerificationFailure> {
    if bytes == u64::MAX {
        return Err(failure(
            "SOURCE_MANIFEST_BUDGET_EXCEEDED",
            "Logical manifest working-memory forecast exceeds u64",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "verification_memory_tests.rs"]
mod tests;
