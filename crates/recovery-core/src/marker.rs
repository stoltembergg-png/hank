//! Crash recovery marker types.
//!
//! A `RecoveryMarker` is a redacted, bounded description of a crashed
//! execution's pending side-effects. It is written atomically by the
//! storage layer before an irreversible effect and is classified before
//! startup resumes.

use std::collections::BTreeSet;

/// Maximum number of pending classes accepted in one marker.
pub const MAX_PENDING_CLASSES: usize = 32;
/// Maximum number of opaque capability or credential references in a marker.
pub const MAX_OPAQUE_REFS: usize = 32;
/// Maximum size of an opaque reference or marker identity in bytes.
pub const MAX_OPAQUE_REF_LEN: usize = 128;

/// The side-effect categories that can remain pending after a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecoveryClass {
    /// A transaction write that can be replayed by the storage adapter.
    TransactionWrite,
    /// A journal append that can be replayed idempotently.
    JournalAppend,
    /// A database migration requiring an explicit recovery path.
    DatabaseMigration,
    /// A revoked credential requiring revalidation.
    CredentialRevocationPending,
    /// A rotated capability set requiring revalidation.
    CapabilityRotationPending,
    /// An effect whose ownership or semantics are not known.
    UnknownEffect,
    /// A marker that cannot be trusted because its structure is invalid.
    CorruptMarker,
}

impl RecoveryClass {
    /// Returns whether this class needs fresh capability/credential checks.
    pub fn is_revalidatable(self) -> bool {
        matches!(
            self,
            Self::CredentialRevocationPending | Self::CapabilityRotationPending
        )
    }
}

/// Opaque references passed to the revalidation callback.
///
/// The values identify capability and credential records; they are not
/// credential material, bearer tokens, or serialized secrets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevalidationRequest {
    /// Unique capability references derived from the marker.
    pub capability_set: BTreeSet<String>,
    /// Unique credential references derived from the marker.
    pub credential_set: BTreeSet<String>,
}

impl RevalidationRequest {
    /// Returns true when both reference sets are empty.
    pub fn is_empty(&self) -> bool {
        self.capability_set.is_empty() && self.credential_set.is_empty()
    }
}

/// Marker written atomically before any irreversible effect.
///
/// `project_id` and `recovery_id` are bounded opaque identities. The
/// capability and credential vectors contain references only; sensitive
/// material must never be placed in this structure. `last_safe_action` is
/// retained for diagnostics but is deliberately omitted from redacted audit
/// output by the coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryMarker {
    /// Opaque project identity. Storage adapters bind one instance to this value.
    pub project_id: String,
    /// Opaque identifier for this recovery attempt.
    pub recovery_id: String,
    /// Monotonically increasing execution epoch.
    pub epoch: u64,
    /// Highest epoch known to have completed safely.
    pub last_known_good_epoch: u64,
    /// Bounded list of pending side-effect classes.
    pub pending_classes: Vec<RecoveryClass>,
    /// Opaque actor identity, never emitted in the redacted bundle.
    pub actor: String,
    /// Opaque capability references used only to construct a revalidation request.
    pub capability_set: Vec<String>,
    /// Opaque credential references used only to construct a revalidation request.
    pub credential_set: Vec<String>,
    /// Human-readable diagnostic action, never emitted in the redacted bundle.
    pub last_safe_action: String,
}

impl RecoveryMarker {
    /// Returns true when any bounded marker field exceeds its contract limit.
    pub fn exceeds_bound(&self) -> bool {
        self.project_id.len() > MAX_OPAQUE_REF_LEN
            || self.recovery_id.len() > MAX_OPAQUE_REF_LEN
            || self.actor.len() > MAX_OPAQUE_REF_LEN
            || self.pending_classes.len() > MAX_PENDING_CLASSES
            || self.capability_set.len() > MAX_OPAQUE_REFS
            || self.credential_set.len() > MAX_OPAQUE_REFS
            || self
                .capability_set
                .iter()
                .chain(self.credential_set.iter())
                .any(|value| value.len() > MAX_OPAQUE_REF_LEN)
    }

    /// Returns true when the marker has no pending effect and is at a known-good epoch.
    pub fn is_clean(&self) -> bool {
        self.pending_classes.is_empty() && self.epoch == self.last_known_good_epoch
    }

    /// Returns the set of classes that require revalidation before privileged automation.
    pub fn revalidatable_classes(&self) -> BTreeSet<RecoveryClass> {
        self.pending_classes
            .iter()
            .copied()
            .filter(|class| class.is_revalidatable())
            .collect()
    }

    /// Derives bounded opaque references for the revalidation callback.
    pub fn revalidation_request(&self) -> RevalidationRequest {
        RevalidationRequest {
            capability_set: self.capability_set.iter().cloned().collect(),
            credential_set: self.credential_set.iter().cloned().collect(),
        }
    }

    /// Returns classes that must never be replayed automatically.
    pub fn quarantined_classes(&self) -> BTreeSet<RecoveryClass> {
        self.pending_classes
            .iter()
            .copied()
            .filter(|class| {
                matches!(
                    class,
                    RecoveryClass::DatabaseMigration
                        | RecoveryClass::UnknownEffect
                        | RecoveryClass::CorruptMarker
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{RecoveryClass, RecoveryMarker, MAX_PENDING_CLASSES};

    fn marker(classes: Vec<RecoveryClass>) -> RecoveryMarker {
        RecoveryMarker {
            project_id: "project-default".into(),
            recovery_id: "r-1".into(),
            epoch: 1,
            last_known_good_epoch: 0,
            pending_classes: classes,
            actor: String::new(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: String::new(),
        }
    }

    #[test]
    fn clean_requires_no_pending_classes() {
        let mut clean = marker(Vec::new());
        clean.last_known_good_epoch = 1;
        assert!(clean.is_clean());
        clean.pending_classes = vec![RecoveryClass::JournalAppend];
        assert!(!clean.is_clean());
    }

    #[test]
    fn revalidatable_classes_are_selected() {
        let marker = marker(vec![
            RecoveryClass::CredentialRevocationPending,
            RecoveryClass::TransactionWrite,
        ]);
        assert_eq!(marker.revalidatable_classes().len(), 1);
    }

    #[test]
    fn marker_bound_is_enforced() {
        let marker = marker(vec![RecoveryClass::JournalAppend; MAX_PENDING_CLASSES + 1]);
        assert!(marker.exceeds_bound());
    }

    #[test]
    fn revalidation_references_are_deduplicated() {
        let mut marker = marker(vec![RecoveryClass::CapabilityRotationPending]);
        marker.capability_set = vec!["cap-read".into(), "cap-read".into()];
        marker.credential_set = vec!["cred-ref".into()];
        let request = marker.revalidation_request();
        assert_eq!(request.capability_set.len(), 1);
        assert_eq!(request.credential_set.len(), 1);
    }
}
