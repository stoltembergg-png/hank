//! Storage abstraction for the recovery coordinator.
//!
//! Adapters that talk to real storage (SQLite, sled, fsync) belong to
//! later PRs. This card ships only the trait and an `InMemoryStorage`
//! used by the unit tests. The trait is the contract that future
//! adapters must satisfy; the coordinator only depends on the trait.

use crate::marker::RecoveryMarker;

/// Abstract storage the coordinator uses to load and persist recovery
/// state. Implementations must be safe to call from a single-threaded
/// startup path; the trait does not promise concurrency.
pub trait RecoveryStorage {
    /// Loads the current marker, if any. Returns `None` if no marker
    /// has ever been written (clean first run).
    fn load_marker(&self) -> Option<RecoveryMarker>;

    /// Atomically writes a new marker. Implementations must guarantee
    /// that either the full marker is visible to a subsequent
    /// `load_marker` call, or no marker is visible.
    fn write_marker(&mut self, marker: &RecoveryMarker) -> Result<(), RecoveryError>;

    /// Appends a recovery outcome entry to the audit log. The log is
    /// bounded; implementations may evict older entries.
    fn append_audit(&mut self, entry: &RecoveryAuditEntry) -> Result<(), RecoveryError>;
}

/// Bounded audit log entry. No secret material; the redactor in the
/// coordinator is responsible for producing a safe value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAuditEntry {
    pub recovery_id: String,
    pub outcome_kind: String,
    pub quarantined_ids: Vec<String>,
    pub redacted_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecoveryError {
    #[error("recovery storage is unavailable")]
    StorageUnavailable,
    #[error("recovery marker exceeds bounded size")]
    MarkerTooLarge,
    #[error("recovery storage rejected the write")]
    WriteRejected,
}

/// In-memory implementation used by the unit tests and by the contract
/// documentation. Not safe for production; persistence is lost across
/// process restarts.
#[derive(Debug, Default, Clone)]
pub struct InMemoryStorage {
    marker: Option<RecoveryMarker>,
    audit: Vec<RecoveryAuditEntry>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn audit(&self) -> &[RecoveryAuditEntry] {
        &self.audit
    }
}

impl RecoveryStorage for InMemoryStorage {
    fn load_marker(&self) -> Option<RecoveryMarker> {
        self.marker.clone()
    }

    fn write_marker(&mut self, marker: &RecoveryMarker) -> Result<(), RecoveryError> {
        if marker.exceeds_bound() {
            return Err(RecoveryError::MarkerTooLarge);
        }
        self.marker = Some(marker.clone());
        Ok(())
    }

    fn append_audit(&mut self, entry: &RecoveryAuditEntry) -> Result<(), RecoveryError> {
        self.audit.push(entry.clone());
        Ok(())
    }
}
