//! Storage abstraction for the recovery coordinator.
//!
//! Adapters that talk to real storage (SQLite, sled, fsync) belong to later
//! PRs. This card ships the trait contract and an `InMemoryStorage` fixture.
//! The trait is explicitly bound to one project namespace so replay claims
//! cannot cross project boundaries.

use std::collections::BTreeMap;

use crate::coordinator::{RecoveryAuditEntry, RecoveryError};
use crate::marker::RecoveryMarker;

/// Maximum number of audit entries retained by the in-memory fixture.
pub const MAX_AUDIT_ENTRIES: usize = 128;

/// Durable result of a replay claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayClaim {
    /// This caller acquired the claim and may execute the callback.
    Acquired,
    /// The recovery ID was durably completed by an earlier caller.
    AlreadyCompleted,
    /// Another caller currently owns the claim.
    InProgress,
    /// An earlier caller failed; automatic retry is refused.
    PreviouslyFailed,
}

/// Durable completion state for a replay claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayCompletion {
    /// The callback completed and its effect is considered applied.
    Succeeded,
    /// The callback failed; the marker must be quarantined.
    Failed,
}

/// Abstract storage used by the startup coordinator.
///
/// A production implementation must make marker writes, replay claims, and
/// completion transitions durable and atomic. It must also retain audit data
/// without storing raw actor, capability, credential, or action material.
pub trait RecoveryStorage {
    /// Returns the project this storage instance is permanently bound to.
    fn project_id(&self) -> &str;

    /// Loads the current marker, if any. `None` means no marker exists.
    fn load_marker(&self) -> Result<Option<RecoveryMarker>, RecoveryError>;

    /// Atomically writes a marker: full visibility or no visibility.
    fn write_marker(&mut self, marker: &RecoveryMarker) -> Result<(), RecoveryError>;

    /// Atomically claims a recovery ID in this storage's project namespace.
    fn claim_replay(&mut self, recovery_id: &str) -> Result<ReplayClaim, RecoveryError>;

    /// Durably records whether the claimed callback succeeded or failed.
    fn complete_replay(
        &mut self,
        recovery_id: &str,
        completion: ReplayCompletion,
    ) -> Result<(), RecoveryError>;

    /// Appends a bounded, redacted recovery audit entry.
    fn append_audit(&mut self, entry: RecoveryAuditEntry) -> Result<(), RecoveryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayState {
    InProgress,
    Completed,
    Failed,
}

/// Non-persistent storage fixture used by unit and contract tests.
///
/// It models the required state transitions but is not suitable for
/// production because all state is lost when the process exits.
#[derive(Debug, Clone)]
pub struct InMemoryStorage {
    project_id: String,
    marker: Option<RecoveryMarker>,
    claims: BTreeMap<String, ReplayState>,
    audit: Vec<RecoveryAuditEntry>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::for_project("project-default")
    }
}

impl InMemoryStorage {
    /// Creates a fixture bound to the default project namespace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a fixture bound to one explicit project namespace.
    pub fn for_project(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            marker: None,
            claims: BTreeMap::new(),
            audit: Vec::new(),
        }
    }

    /// Returns retained audit entries in insertion order.
    pub fn audit(&self) -> &[RecoveryAuditEntry] {
        &self.audit
    }
}

impl RecoveryStorage for InMemoryStorage {
    fn project_id(&self) -> &str {
        &self.project_id
    }

    fn load_marker(&self) -> Result<Option<RecoveryMarker>, RecoveryError> {
        Ok(self.marker.clone())
    }

    fn write_marker(&mut self, marker: &RecoveryMarker) -> Result<(), RecoveryError> {
        if marker.exceeds_bound() {
            return Err(RecoveryError::Storage(
                "recovery marker exceeds bounded size".into(),
            ));
        }
        if marker.project_id != self.project_id {
            return Err(RecoveryError::ProjectMismatch);
        }
        self.marker = Some(marker.clone());
        Ok(())
    }

    fn claim_replay(&mut self, recovery_id: &str) -> Result<ReplayClaim, RecoveryError> {
        if recovery_id.trim().is_empty() {
            return Err(RecoveryError::Storage("recovery ID is empty".into()));
        }
        Ok(match self.claims.get(recovery_id).copied() {
            None => {
                self.claims
                    .insert(recovery_id.to_owned(), ReplayState::InProgress);
                ReplayClaim::Acquired
            }
            Some(ReplayState::InProgress) => ReplayClaim::InProgress,
            Some(ReplayState::Completed) => ReplayClaim::AlreadyCompleted,
            Some(ReplayState::Failed) => ReplayClaim::PreviouslyFailed,
        })
    }

    fn complete_replay(
        &mut self,
        recovery_id: &str,
        completion: ReplayCompletion,
    ) -> Result<(), RecoveryError> {
        let Some(state) = self.claims.get_mut(recovery_id) else {
            return Err(RecoveryError::Storage("replay claim is missing".into()));
        };
        if *state != ReplayState::InProgress {
            return Err(RecoveryError::Storage("replay claim is not owned".into()));
        }
        *state = match completion {
            ReplayCompletion::Succeeded => ReplayState::Completed,
            ReplayCompletion::Failed => ReplayState::Failed,
        };
        Ok(())
    }

    fn append_audit(&mut self, entry: RecoveryAuditEntry) -> Result<(), RecoveryError> {
        if self.audit.len() >= MAX_AUDIT_ENTRIES {
            self.audit.remove(0);
        }
        self.audit.push(entry);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryStorage, RecoveryStorage, ReplayClaim, ReplayCompletion, MAX_AUDIT_ENTRIES,
    };
    use crate::coordinator::{RecoveryAuditEntry, RecoveryError};
    use crate::marker::RecoveryMarker;

    fn marker(project_id: &str) -> RecoveryMarker {
        RecoveryMarker {
            project_id: project_id.into(),
            recovery_id: "r-1".into(),
            epoch: 1,
            last_known_good_epoch: 0,
            pending_classes: vec![],
            actor: String::new(),
            capability_set: vec![],
            credential_set: vec![],
            last_safe_action: String::new(),
        }
    }

    fn audit() -> RecoveryAuditEntry {
        RecoveryAuditEntry {
            bundle: crate::coordinator::RedactedCrashBundle {
                recovery_id: "r-1".into(),
                epoch: 1,
                last_known_good_epoch: 0,
                pending_classes: Default::default(),
            },
            outcome: crate::coordinator::RecoveryOutcome::Replayed {
                recovery_id: "r-1".into(),
            },
        }
    }

    #[test]
    fn marker_write_is_project_bound() {
        let mut storage = InMemoryStorage::for_project("project-a");
        assert!(storage.write_marker(&marker("project-b")).is_err());
        assert!(storage.load_marker().expect("marker read").is_none());
        assert!(storage.write_marker(&marker("project-a")).is_ok());
    }

    #[test]
    fn replay_claim_completion_is_durable_in_fixture() {
        let mut storage = InMemoryStorage::new();
        assert_eq!(
            storage.claim_replay("r-1").expect("first claim"),
            ReplayClaim::Acquired
        );
        assert_eq!(
            storage.claim_replay("r-1").expect("second claim"),
            ReplayClaim::InProgress
        );
        storage
            .complete_replay("r-1", ReplayCompletion::Succeeded)
            .expect("completion");
        assert_eq!(
            storage.claim_replay("r-1").expect("completed claim"),
            ReplayClaim::AlreadyCompleted
        );
    }

    #[test]
    fn audit_log_is_bounded() -> Result<(), RecoveryError> {
        let mut storage = InMemoryStorage::new();
        for _ in 0..(MAX_AUDIT_ENTRIES + 1) {
            storage.append_audit(audit())?;
        }
        assert_eq!(storage.audit().len(), MAX_AUDIT_ENTRIES);
        Ok(())
    }
}
