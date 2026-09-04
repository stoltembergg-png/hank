//! Crash recovery marker types.
//!
//! A `RecoveryMarker` is a redacted, bounded description of a crashed
//! execution's pending side-effects. It is written atomically by the
//! storage layer before any irreversible effect and read by
//! `RecoveryCoordinator` at startup to classify the state.
//!
//! Secret material, raw prompts, file paths, tokens and page content
//! must never appear in a marker; only opaque identifiers and
//! [`RecoveryClass`] tags.

use std::collections::BTreeSet;

/// Maximum number of [`RecoveryClass`] entries a single marker may hold.
/// Anything beyond this bound is treated as `Corrupt` and rejected.
pub const MAX_PENDING_CLASSES: usize = 32;

/// Bounded set of side-effect classes a crashed execution had pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecoveryClass {
    /// A bounded transactional write to durable storage.
    TransactionWrite,
    /// Append to a structured journal.
    JournalAppend,
    /// A remote session that may have been left open.
    RemoteSessionPending,
    /// A tool call that may have partially executed.
    ToolExecutionPending,
    /// A pending credential revocation.
    CredentialRevocationPending,
    /// A pending capability rotation.
    CapabilityRotationPending,
    /// A migration step that may have partially run.
    DatabaseMigration,
}

impl RecoveryClass {
    pub fn is_quarantined(self) -> bool {
        matches!(
            self,
            RecoveryClass::RemoteSessionPending
                | RecoveryClass::ToolExecutionPending
                | RecoveryClass::DatabaseMigration
        )
    }

    pub fn is_revalidatable(self) -> bool {
        matches!(
            self,
            RecoveryClass::CredentialRevocationPending | RecoveryClass::CapabilityRotationPending
        )
    }
}

/// Maximum number of opaque capability or credential references in a marker.
pub const MAX_OPAQUE_REFS: usize = 32;
/// Maximum size of each opaque reference in bytes.
pub const MAX_OPAQUE_REF_LEN: usize = 128;

/// Opaque references that must be revalidated before privileged resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevalidationRequest {
    pub capability_set: BTreeSet<String>,
    pub credential_set: BTreeSet<String>,
}

impl RevalidationRequest {
    pub fn is_empty(&self) -> bool {
        self.capability_set.is_empty() && self.credential_set.is_empty()
    }
}
/// Marker written atomically before any irreversible effect. Read by
/// `RecoveryCoordinator` at startup. `epoch` is monotonically increasing;
/// `last_known_good_epoch` is the highest epoch that completed cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryMarker {
    pub recovery_id: String,
    pub epoch: u64,
    pub last_known_good_epoch: u64,
    pub pending_classes: Vec<RecoveryClass>,
    pub actor: String,
    pub capability_set: Vec<String>,
    pub credential_set: Vec<String>,
    pub last_safe_action: String,
}

impl RecoveryMarker {
    /// Returns true if the marker exceeds the bounded pending classes.
    pub fn exceeds_bound(&self) -> bool {
        self.pending_classes.len() > MAX_PENDING_CLASSES
            || self.capability_set.len() > MAX_OPAQUE_REFS
            || self.credential_set.len() > MAX_OPAQUE_REFS
            || self
                .capability_set
                .iter()
                .chain(self.credential_set.iter())
                .any(|reference| reference.is_empty() || reference.len() > MAX_OPAQUE_REF_LEN)
    }

    /// Returns true if the marker indicates a clean shutdown
    /// (epoch matches last known good and no pending classes).
    pub fn is_clean(&self) -> bool {
        self.pending_classes.is_empty() && self.epoch == self.last_known_good_epoch
    }

    pub fn revalidation_request(&self) -> RevalidationRequest {
        RevalidationRequest {
            capability_set: self.capability_set.iter().cloned().collect(),
            credential_set: self.credential_set.iter().cloned().collect(),
        }
    }

    /// Returns the set of classes that require revalidation before
    /// any privileged automation resumes.
    pub fn revalidatable_classes(&self) -> BTreeSet<RecoveryClass> {
        self.pending_classes
            .iter()
            .copied()
            .filter(|c| c.is_revalidatable())
            .collect()
    }

    /// Returns the set of classes that are quarantined by default
    /// (cannot be replayed automatically in `RecoveryMode::Safe`).
    pub fn quarantined_classes(&self) -> BTreeSet<RecoveryClass> {
        self.pending_classes
            .iter()
            .copied()
            .filter(|c| c.is_quarantined())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_marker_is_clean() {
        let marker = RecoveryMarker {
            recovery_id: "r-1".into(),
            epoch: 5,
            last_known_good_epoch: 5,
            pending_classes: Vec::new(),
            actor: String::new(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: String::new(),
        };
        assert!(marker.is_clean());
    }

    #[test]
    fn marker_with_pending_classes_is_not_clean() {
        let marker = RecoveryMarker {
            recovery_id: "r-2".into(),
            epoch: 5,
            last_known_good_epoch: 5,
            pending_classes: vec![RecoveryClass::JournalAppend],
            actor: String::new(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: String::new(),
        };
        assert!(!marker.is_clean());
    }

    #[test]
    fn exceeds_bound_is_true_above_max() {
        let classes = vec![RecoveryClass::JournalAppend; MAX_PENDING_CLASSES + 1];
        let marker = RecoveryMarker {
            recovery_id: "r-3".into(),
            epoch: 1,
            last_known_good_epoch: 0,
            pending_classes: classes,
            actor: String::new(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: String::new(),
        };
        assert!(marker.exceeds_bound());
    }
}
