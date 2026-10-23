use nrz_contract::{artifact_json_v1::AllocationProfile, source_bundle_v2 as wire};
use serde_json::Value;

use super::preflight::StructuralCounts;

const FIXED_BYTES: u64 = 1024 * 1024;
const PREFLIGHT_FIXED_BYTES: u64 = 64 * 1024;
const NUMERIC_FIXED_BYTES: u64 = 4 * 1024 * 1024;

/// Pinned parser growth, previous escaped path and numeric fallback, including
/// simultaneous realloc blocks. Short integer tokens cannot enter lexical Bigint.
pub(super) fn preflight_reservation(profile: &AllocationProfile) -> Option<u64> {
    let fixed = if profile.numeric_heap_candidate {
        NUMERIC_FIXED_BYTES
    } else {
        PREFLIGHT_FIXED_BYTES
    };
    profile
        .max_escaped_string_bytes
        .checked_mul(5)?
        .checked_add(profile.max_unquoted_run_bytes.checked_mul(3)?)?
        .checked_add(fixed)
}

/// Pinned Rust 1.98.1 / serde 1.0.229 / serde_json 1.0.151 heap envelope.
/// Includes old/new growth allocations; RSS, worker stacks and allocator arenas
/// require separate process headroom/backstops. Keep this permit until proof drop.
pub(super) fn verification_reservation(counts: &StructuralCounts) -> Option<u64> {
    if !supported_layout() {
        return None;
    }
    let value_bytes = std::mem::size_of::<Value>() as u64;
    let dom = dom_containers(
        counts.object_members,
        counts.objects,
        counts.array_elements,
        value_bytes,
    )?;
    // AP Number reparsing retains amortized capacity up to twice its owned
    // payload length; short scalar Number strings require at most20 bytes each.
    let strings = counts
        .decoded_string_bytes
        .checked_add(counts.owned_parser_string_bytes)?
        .checked_add(counts.strings.checked_mul(64)?)?
        .checked_add(counts.json_values.checked_mul(32)?)?;
    let parse = preflight_reservation(&counts.allocation_profile)?
        .checked_add(strings)?
        .checked_add(dom)?;
    let canonical = strings
        .checked_add(dom)?
        .checked_add(counts.object_members.checked_mul(16)?)?;

    // Unbounded typed Vec elements: FileEntry112, all remaining elements <=56.
    // Larger Layer304/Route136 are preflight bounded to10/200; the fixed allowance
    // covers those Vecs and the at-most-ten V1 adapter layer structs.
    let wire_containers = counts
        .files
        .checked_mul(672)?
        .checked_add(
            counts
                .array_elements
                .checked_sub(counts.files)?
                .checked_mul(336)?,
        )?
        .checked_add(192 * 1024)?
        .checked_add(dom_maps(
            counts.object_members,
            counts.objects,
            value_bytes,
        )?)?;
    // Internally tagged serde enums buffer Content32 / (Content,Content)64.
    let content = counts
        .array_elements
        .checked_mul(192)?
        .checked_add(counts.object_members.checked_mul(384)?)?;
    // Debug escaping <=6x; Vec growth or boxed-string shrink can retain old+new
    // buffers <=3x that output. Preflight excludes accepted arbitrary file strings.
    let diagnostics = counts.max_error_string_bytes.checked_mul(18)?;
    let convert = strings
        .checked_add(dom)?
        .checked_add(content)?
        .checked_add(wire_containers)?
        .checked_add(diagnostics)?;

    // V1 layer adaptation retains runtime-config Values and temporarily clones
    // network/application subtrees; only root.layers counters are duplicated.
    let metadata = counts
        .metadata_decoded_string_bytes
        .checked_mul(3)?
        .checked_add(counts.metadata_strings.checked_mul(192)?)?
        .checked_add(
            dom_containers(
                counts.metadata_object_members,
                counts.metadata_objects,
                counts.metadata_array_elements,
                value_bytes,
            )?
            .checked_mul(2)?,
        )?
        .checked_add(counts.metadata_array_elements.checked_mul(480)?)?;
    let semantic = strings
        .checked_add(wire_containers)?
        .checked_add(counts.files.checked_mul(192)?)?
        .checked_add(crate::SourceArchivePathIndex::working_memory_bytes(
            counts.unique_path_nodes,
            counts.max_path_bytes,
        )?)?
        .checked_add(metadata)?
        .checked_add(diagnostics)?;
    let peak = parse
        .max(canonical)
        .max(convert)
        .max(semantic)
        .checked_add(FIXED_BYTES)?;
    peak.checked_add(peak.checked_add(3)?.checked_div(4)?)
}

fn dom_containers(members: u64, objects: u64, elements: u64, value_bytes: u64) -> Option<u64> {
    dom_maps(members, objects, value_bytes)?
        .checked_add(elements.checked_mul(value_bytes.checked_mul(6)?)?)
}

fn dom_maps(members: u64, objects: u64, value_bytes: u64) -> Option<u64> {
    if value_bytes == 32 {
        // BTree nodes <=728B; nonroot nodes have >=5 members, root may have1.
        members
            .checked_mul(146)?
            .checked_add(objects.checked_mul(768)?)
    } else {
        // preserve_order: IndexMap payload104B, Vec old/new <=624B/member,
        // plus raw hash-table growth/control bytes and allocation slack.
        members.checked_mul(768)
    }
}

fn supported_layout() -> bool {
    matches!(std::mem::size_of::<Value>(), 32 | 72)
        && std::mem::size_of::<wire::FileEntry>() <= 112
        && std::mem::size_of::<wire::Layer>() <= 304
        && std::mem::size_of::<wire::Route>() <= 136
        && std::mem::size_of::<wire::FallthroughCondition>() <= 56
        && std::mem::size_of::<wire::NetworkAllowFrom>() <= 56
}

#[cfg(test)]
#[path = "source_bundle_v2_memory_tests.rs"]
mod tests;
