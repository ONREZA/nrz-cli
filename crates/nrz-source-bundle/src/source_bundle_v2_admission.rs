//! Shared local admission for untrusted manifest bodies and their verified metadata.
//!
//! The pool accounts for owned inputs and conservative requested-heap estimates.
//! Transport buffers, caller clones, allocator arenas and worker stacks belong to
//! the service's process/cgroup budget. Callers choose capacity and path-work policy.

use std::fmt::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::{
    SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES, SourceBundleV2ManifestBudget,
    VerifiedSourceBundleV2Manifest, decode_with_work_budget, memory, preflight,
    validation::ValidationError,
};
use crate::path_graph::PathWorkBudget;

#[derive(Debug, thiserror::Error)]
pub enum AdmissionError {
    #[error("manifest exceeds the SOURCE_BUNDLE_V2 byte limit")]
    ManifestByteLimit,
    #[error("manifest input length differs from its reserved length")]
    InputLength,
    #[error("manifest memory capacity unavailable (requested {requested_bytes} bytes)")]
    MemoryUnavailable { requested_bytes: u64 },
    #[error("manifest memory estimate overflows or uses an unsupported layout")]
    UnsupportedEstimate,
    #[error("manifest buffer allocation failed")]
    AllocationFailure,
    #[error("manifest file count exceeds caller resource budget")]
    FileCountLimit,
    #[error("manifest path verification work budget exhausted")]
    PathWorkLimit,
    #[error("invalid SOURCE_BUNDLE_V2 manifest: {0}")]
    InvalidManifest(String),
}

/// Clones share capacity. Acquisition never waits; the owner decides backpressure.
#[derive(Clone, Debug)]
pub struct ManifestMemoryPool(Arc<PoolState>);

#[derive(Debug)]
struct PoolState {
    capacity: u64,
    reserved: AtomicU64,
}

impl ManifestMemoryPool {
    #[must_use]
    pub fn new(capacity_bytes: u64) -> Self {
        Self(Arc::new(PoolState {
            capacity: capacity_bytes,
            reserved: AtomicU64::new(0),
        }))
    }

    #[must_use]
    pub fn reserved_bytes(&self) -> u64 {
        self.0.reserved.load(Ordering::Relaxed)
    }

    /// Total configured capacity, unchanged by reservations on any clone.
    #[must_use]
    pub fn capacity_bytes(&self) -> u64 {
        self.0.capacity
    }

    /// Acquire capacity before allocating the body. Feed bounded transport chunks
    /// through `append`; the input owns both its buffer and its reservation.
    pub fn reserve_input(&self, expected_bytes: usize) -> Result<ManifestInput, AdmissionError> {
        if expected_bytes > SOURCE_BUNDLE_V2_MAX_MANIFEST_BYTES {
            return Err(AdmissionError::ManifestByteLimit);
        }
        let reservation = self.reserve(expected_bytes as u64)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(expected_bytes)
            .map_err(|_| AdmissionError::AllocationFailure)?;
        Ok(ManifestInput {
            bytes,
            expected_bytes,
            reservation,
        })
    }

    fn reserve(&self, bytes: u64) -> Result<MemoryReservation, AdmissionError> {
        self.0
            .reserved
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |reserved| {
                reserved
                    .checked_add(bytes)
                    .filter(|total| *total <= self.0.capacity)
            })
            .map_err(|_| AdmissionError::MemoryUnavailable {
                requested_bytes: bytes,
            })?;
        Ok(MemoryReservation {
            pool: self.clone(),
            bytes,
        })
    }
}

#[derive(Debug)]
pub(super) struct MemoryReservation {
    pool: ManifestMemoryPool,
    bytes: u64,
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        self.pool
            .0
            .reserved
            .fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

impl MemoryReservation {
    pub(super) fn bytes(&self) -> u64 {
        self.bytes
    }
}

/// Dropping a partial body releases its allocation before returning pool capacity.
#[derive(Debug)]
pub struct ManifestInput {
    bytes: Vec<u8>,
    expected_bytes: usize,
    reservation: MemoryReservation,
}

impl ManifestInput {
    pub fn append(&mut self, chunk: &[u8]) -> Result<(), AdmissionError> {
        if chunk.len() > self.expected_bytes - self.bytes.len() {
            return Err(AdmissionError::InputLength);
        }
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }

    /// Consume a complete body, reserve each phase before allocation, and retain
    /// the verification peak allowance until the admitted proof is dropped.
    pub fn verify(
        self,
        budget: SourceBundleV2ManifestBudget,
        max_path_work: u64,
    ) -> Result<AdmittedSourceBundleV2Manifest, AdmissionError> {
        if self.bytes.len() != self.expected_bytes {
            return Err(AdmissionError::InputLength);
        }
        let profile = nrz_contract::artifact_json_v1::allocation_profile(&self.bytes);
        let pool = &self.reservation.pool;
        let counts = {
            let _scratch = pool.reserve(
                memory::preflight_reservation(&profile)
                    .ok_or(AdmissionError::UnsupportedEstimate)?,
            )?;
            preflight::inspect(&self.bytes, budget.max_file_count, profile).map_err(|error| {
                match error {
                    preflight::PreflightError::Invalid(error) => invalid(error),
                    preflight::PreflightError::ResourceLimit(limit) => match limit {
                        preflight::PreflightResourceLimit::FileCount => {
                            AdmissionError::FileCountLimit
                        }
                        preflight::PreflightResourceLimit::CounterOverflow => {
                            AdmissionError::UnsupportedEstimate
                        }
                        preflight::PreflightResourceLimit::AllocationFailure => {
                            AdmissionError::AllocationFailure
                        }
                    },
                }
            })?
        };
        let reservation = pool.reserve(
            memory::verification_reservation(&counts).ok_or(AdmissionError::UnsupportedEstimate)?,
        )?;
        let verified =
            decode_with_work_budget(&self.bytes, budget, &mut PathWorkBudget::new(max_path_work))
                .map_err(|error| match error {
                ValidationError::Manifest(error) => invalid(error),
                ValidationError::WorkLimit => AdmissionError::PathWorkLimit,
            })?;
        drop(self);
        Ok(AdmittedSourceBundleV2Manifest {
            verified,
            reservation,
        })
    }
}

/// An immutable proof whose data cannot outlive its memory reservation.
///
/// ```compile_fail,E0505
/// use nrz_source_bundle::source_bundle_v2::admission::AdmittedSourceBundleV2Manifest;
/// fn detach(admitted: AdmittedSourceBundleV2Manifest) {
///     let verified = admitted.verified();
///     drop(admitted);
///     let _ = verified.wire();
/// }
/// ```
#[derive(Debug)]
pub struct AdmittedSourceBundleV2Manifest {
    // Declaration order frees the wire before returning its capacity to the pool.
    verified: VerifiedSourceBundleV2Manifest,
    reservation: MemoryReservation,
}

impl AdmittedSourceBundleV2Manifest {
    /// Prepare an independent Static inventory using new permits from this proof's pool.
    pub fn prepare_static_projection(
        &self,
        max_path_work: u64,
    ) -> Result<
        super::static_projection::PreparedStaticProjection,
        super::static_projection::StaticProjectionError,
    > {
        super::static_projection::prepare(self, max_path_work)
    }

    pub(super) fn reserve_memory(&self, bytes: u64) -> Result<MemoryReservation, AdmissionError> {
        self.reservation.pool.reserve(bytes)
    }

    #[must_use]
    pub fn verified(&self) -> &VerifiedSourceBundleV2Manifest {
        &self.verified
    }

    #[must_use]
    pub fn reserved_bytes(&self) -> u64 {
        self.reservation.bytes
    }
}

fn invalid(error: impl fmt::Display) -> AdmissionError {
    // Error payloads can contain attacker-controlled strings; release them under
    // the phase permit and return only a bounded diagnostic to the caller.
    struct Diagnostic(String);
    impl fmt::Write for Diagnostic {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            let remaining = 512 - self.0.len();
            let mut end = text.len().min(remaining);
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            self.0.push_str(&text[..end]);
            if end < text.len() {
                Err(fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let mut diagnostic = Diagnostic(String::with_capacity(512));
    let _ = write!(diagnostic, "{error}");
    AdmissionError::InvalidManifest(diagnostic.0)
}

#[cfg(test)]
#[path = "source_bundle_v2_admission_tests.rs"]
mod tests;
