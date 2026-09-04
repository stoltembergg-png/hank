//! Startup recovery coordinator.
//!
//! The coordinator classifies a bounded marker before any resume attempt,
//! refuses ambiguous/corrupt work in safe mode, and delegates durable
//! idempotence to [`RecoveryStorage`].

use std::collections::BTreeSet;
use std::fmt::Write as _;

use thiserror::Error;

use crate::marker::{RecoveryClass, RecoveryMarker, RevalidationRequest};
use crate::storage::{RecoveryStorage, ReplayClaim, ReplayCompletion};

/// Recovery behavior selected for startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    /// Permit only classified, bounded recovery and fail closed otherwise.
    Safe,
    /// Resume only when no pending effect requires recovery.
    Resume,
}

/// Classification produced before any callback is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryClassification {
    /// No pending effect exists and the marker is at a known-good epoch.
    Clean,
    /// Only transaction/journal effects are pending and the epoch is valid.
    Recoverable,
    /// The marker is structurally valid but its effects are not safe to infer.
    Unknown,
    /// The marker violates bounds or epoch invariants.
    Corrupt,
}

/// Result of processing one marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    /// The recovery ID was durably completed earlier; no callback ran.
    AlreadyReplayed { recovery_id: String },
    /// Another process currently owns the durable recovery claim.
    ReplayInProgress { recovery_id: String },
    /// The marker was replayed or revalidation completed successfully.
    Replayed { recovery_id: String },
    /// Fresh external validation is required before privileged automation resumes.
    RevalidateRequired {
        recovery_id: String,
        request: RevalidationRequest,
    },
    /// The marker was isolated and must not be replayed automatically.
    Quarantined {
        recovery_id: String,
        classes: BTreeSet<RecoveryClass>,
    },
}

/// Redacted data retained in an audit entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedCrashBundle {
    /// Opaque recovery identifier.
    pub recovery_id: String,
    /// Execution epoch from the marker.
    pub epoch: u64,
    /// Last known good epoch from the marker.
    pub last_known_good_epoch: u64,
    /// Pending class names; no actor or credential material is included.
    pub pending_classes: BTreeSet<RecoveryClass>,
}

/// Redacted outcome retained in an audit entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAuditOutcome {
    /// The recovery ID was already completed.
    AlreadyReplayed { recovery_id: String },
    /// Another process owns the recovery claim.
    ReplayInProgress { recovery_id: String },
    /// A replay or revalidation completed.
    Replayed { recovery_id: String },
    /// Revalidation completed with counts of opaque references only.
    RevalidateRequired {
        recovery_id: String,
        capability_count: usize,
        credential_count: usize,
    },
    /// Recovery was quarantined, retaining class names but no opaque values.
    Quarantined {
        recovery_id: String,
        classes: BTreeSet<RecoveryClass>,
    },
}

impl From<&RecoveryOutcome> for RecoveryAuditOutcome {
    fn from(outcome: &RecoveryOutcome) -> Self {
        match outcome {
            RecoveryOutcome::AlreadyReplayed { recovery_id } => Self::AlreadyReplayed {
                recovery_id: recovery_id.clone(),
            },
            RecoveryOutcome::ReplayInProgress { recovery_id } => Self::ReplayInProgress {
                recovery_id: recovery_id.clone(),
            },
            RecoveryOutcome::Replayed { recovery_id } => Self::Replayed {
                recovery_id: recovery_id.clone(),
            },
            RecoveryOutcome::RevalidateRequired {
                recovery_id,
                request,
            } => Self::RevalidateRequired {
                recovery_id: recovery_id.clone(),
                capability_count: request.capability_set.len(),
                credential_count: request.credential_set.len(),
            },
            RecoveryOutcome::Quarantined {
                recovery_id,
                classes,
            } => Self::Quarantined {
                recovery_id: recovery_id.clone(),
                classes: classes.clone(),
            },
        }
    }
}

/// Audit event emitted after a durable recovery transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAuditEntry {
    /// Redacted marker data.
    pub bundle: RedactedCrashBundle,
    /// Redacted result of the recovery action.
    pub outcome: RecoveryAuditOutcome,
}

impl RedactedCrashBundle {
    /// Serializes the bounded audit payload without sensitive marker fields.
    pub fn to_json(&self) -> String {
        let classes = self
            .pending_classes
            .iter()
            .map(|class| format!("\"{class:?}\""))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"recovery_id\":\"{}\",\"epoch\":{},\"last_known_good_epoch\":{},\"pending_classes\":[{}],\"redacted\":\"[REDACTED]\"}}",
            json_escape(&self.recovery_id),
            self.epoch,
            self.last_known_good_epoch,
            classes
        )
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\u{08}' => escaped.push_str("\\b"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\u{0c}' => escaped.push_str("\\f"),
            '\r' => escaped.push_str("\\r"),
            character if character <= '\u{1f}' => {
                write!(escaped, "\\u{character:04x}", character = character as u32)
                    .expect("writing to String cannot fail");
            }
            character => escaped.push(character),
        }
    }
    escaped
}

/// Errors returned by storage or recovery callbacks.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    /// Storage rejected or could not persist a recovery transition.
    #[error("recovery storage error: {0}")]
    Storage(String),
    /// A callback rejected recovery.
    #[error("recovery callback error: {0}")]
    Callback(String),
    /// The marker belongs to a different project than the storage instance.
    #[error("recovery marker project does not match storage project")]
    ProjectMismatch,
}

/// Callbacks for effects that are safe only after classification.
pub trait RecoveryCallbacks {
    /// Replay a transaction/journal effect idempotently.
    fn on_replay(&mut self, marker: &RecoveryMarker) -> Result<(), RecoveryError>;
    /// Revalidate opaque capability and credential references.
    fn on_revalidate(&mut self, request: &RevalidationRequest) -> Result<(), RecoveryError>;
}

/// Coordinates startup classification and durable recovery transitions.
pub struct RecoveryCoordinator<S> {
    mode: RecoveryMode,
    storage: S,
}

impl<S: RecoveryStorage> RecoveryCoordinator<S> {
    /// Creates a coordinator in fail-closed safe mode.
    pub fn new(storage: S) -> Self {
        Self {
            mode: RecoveryMode::Safe,
            storage,
        }
    }

    /// Creates a coordinator with an explicit startup mode.
    pub fn with_mode(storage: S, mode: RecoveryMode) -> Self {
        Self { mode, storage }
    }

    /// Returns the configured startup mode.
    pub fn mode(&self) -> RecoveryMode {
        self.mode
    }

    /// Returns the storage adapter.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Returns mutable access to the storage adapter.
    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    /// Returns the marker classification without invoking callbacks.
    pub fn classify(&self, marker: &RecoveryMarker) -> RecoveryClassification {
        if marker.exceeds_bound()
            || marker.project_id.trim().is_empty()
            || marker.recovery_id.trim().is_empty()
            || marker.last_known_good_epoch > marker.epoch
            || (marker.epoch == 0 && !marker.pending_classes.is_empty())
            || marker
                .pending_classes
                .contains(&RecoveryClass::CorruptMarker)
            || marker
                .pending_classes
                .contains(&RecoveryClass::DatabaseMigration)
        {
            return RecoveryClassification::Corrupt;
        }
        if marker.is_clean() {
            return RecoveryClassification::Clean;
        }
        if marker.epoch > 0
            && marker.pending_classes.iter().all(|class| {
                matches!(
                    class,
                    RecoveryClass::TransactionWrite | RecoveryClass::JournalAppend
                )
            })
        {
            return RecoveryClassification::Recoverable;
        }
        RecoveryClassification::Unknown
    }

    /// Loads and classifies the atomically stored marker, if one exists.
    pub fn startup_classification(&self) -> Result<Option<RecoveryClassification>, RecoveryError> {
        self.storage
            .load_marker()
            .map(|marker| marker.map(|value| self.classify(&value)))
    }

    /// Processes one marker with durable claim, completion, and audit transitions.
    pub fn replay(
        &mut self,
        marker: &RecoveryMarker,
        callbacks: &mut impl RecoveryCallbacks,
    ) -> Result<RecoveryOutcome, RecoveryError> {
        if marker.project_id != self.storage.project_id() {
            return Err(RecoveryError::ProjectMismatch);
        }

        let classification = self.classify(marker);
        let revalidation_classes = marker.revalidatable_classes();
        let quarantine_classes = marker.quarantined_classes();
        if classification == RecoveryClassification::Corrupt
            || (classification == RecoveryClassification::Unknown
                && (revalidation_classes.is_empty() || !quarantine_classes.is_empty()))
        {
            let outcome = self.quarantine(marker);
            self.record_outcome(marker, &outcome)?;
            return Ok(outcome);
        }

        match self.storage.claim_replay(&marker.recovery_id)? {
            ReplayClaim::AlreadyCompleted => {
                return Ok(RecoveryOutcome::AlreadyReplayed {
                    recovery_id: marker.recovery_id.clone(),
                });
            }
            ReplayClaim::InProgress => {
                return Ok(RecoveryOutcome::ReplayInProgress {
                    recovery_id: marker.recovery_id.clone(),
                });
            }
            ReplayClaim::PreviouslyFailed => {
                let outcome = self.quarantine(marker);
                self.record_outcome(marker, &outcome)?;
                return Ok(outcome);
            }
            ReplayClaim::Acquired => {}
        }

        let action = if self.mode == RecoveryMode::Safe
            && !revalidation_classes.is_empty()
            && quarantine_classes.is_empty()
        {
            let request = marker.revalidation_request();
            callbacks
                .on_revalidate(&request)
                .map(|()| RecoveryOutcome::RevalidateRequired {
                    recovery_id: marker.recovery_id.clone(),
                    request,
                })
        } else {
            match (self.mode, classification) {
                (RecoveryMode::Resume, RecoveryClassification::Recoverable) => {
                    Ok(self.quarantine(marker))
                }
                (RecoveryMode::Safe, RecoveryClassification::Recoverable) => callbacks
                    .on_replay(marker)
                    .map(|()| RecoveryOutcome::Replayed {
                        recovery_id: marker.recovery_id.clone(),
                    }),
                (_, RecoveryClassification::Clean) => Ok(RecoveryOutcome::Replayed {
                    recovery_id: marker.recovery_id.clone(),
                }),
                _ => Ok(self.quarantine(marker)),
            }
        };

        let outcome = match action {
            Ok(value) => value,
            Err(error) => {
                let _ = self
                    .storage
                    .complete_replay(&marker.recovery_id, ReplayCompletion::Failed);
                return Err(error);
            }
        };
        let completion = if self.mode == RecoveryMode::Resume
            && classification == RecoveryClassification::Recoverable
        {
            ReplayCompletion::Deferred
        } else {
            ReplayCompletion::Succeeded
        };
        self.storage
            .complete_replay(&marker.recovery_id, completion)?;
        self.record_outcome(marker, &outcome)?;
        Ok(outcome)
    }

    /// Converts a marker into the bounded audit representation.
    pub fn redacted_bundle(&self, marker: &RecoveryMarker) -> RedactedCrashBundle {
        RedactedCrashBundle {
            recovery_id: marker.recovery_id.clone(),
            epoch: marker.epoch,
            last_known_good_epoch: marker.last_known_good_epoch,
            pending_classes: marker.pending_classes.iter().copied().collect(),
        }
    }

    fn quarantine(&self, marker: &RecoveryMarker) -> RecoveryOutcome {
        RecoveryOutcome::Quarantined {
            recovery_id: marker.recovery_id.clone(),
            classes: marker.pending_classes.iter().copied().collect(),
        }
    }

    fn record_outcome(
        &mut self,
        marker: &RecoveryMarker,
        outcome: &RecoveryOutcome,
    ) -> Result<(), RecoveryError> {
        self.storage.append_audit(RecoveryAuditEntry {
            bundle: self.redacted_bundle(marker),
            outcome: outcome.into(),
        })
    }
}

/// No-op callbacks for callers that only need classification.
#[derive(Debug, Default)]
pub struct NoopCallbacks;

impl RecoveryCallbacks for NoopCallbacks {
    /// Does nothing because no effect replay was requested.
    fn on_replay(&mut self, _marker: &RecoveryMarker) -> Result<(), RecoveryError> {
        Ok(())
    }

    /// Does nothing because no external validation is configured.
    fn on_revalidate(&mut self, _request: &RevalidationRequest) -> Result<(), RecoveryError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NoopCallbacks, RecoveryCallbacks, RecoveryClassification, RecoveryCoordinator,
        RecoveryError, RecoveryMode, RecoveryOutcome,
    };
    use crate::{InMemoryStorage, RecoveryClass, RecoveryMarker, RevalidationRequest};

    fn marker(classes: Vec<RecoveryClass>) -> RecoveryMarker {
        RecoveryMarker {
            project_id: "project-default".into(),
            recovery_id: "r-1".into(),
            epoch: 1,
            last_known_good_epoch: 0,
            pending_classes: classes,
            actor: "actor-opaque".into(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: "sensitive action must not escape".into(),
        }
    }

    #[derive(Default)]
    struct Callbacks {
        replays: usize,
        revalidations: usize,
        request: Option<RevalidationRequest>,
    }

    impl RecoveryCallbacks for Callbacks {
        fn on_replay(&mut self, _marker: &RecoveryMarker) -> Result<(), RecoveryError> {
            self.replays += 1;
            Ok(())
        }

        fn on_revalidate(&mut self, request: &RevalidationRequest) -> Result<(), RecoveryError> {
            self.revalidations += 1;
            self.request = Some(request.clone());
            Ok(())
        }
    }

    #[test]
    fn classifies_clean_recoverable_and_unknown() {
        let coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let mut clean = marker(Vec::new());
        clean.last_known_good_epoch = 1;
        assert_eq!(coordinator.classify(&clean), RecoveryClassification::Clean);
        assert_eq!(
            coordinator.classify(&marker(vec![RecoveryClass::JournalAppend])),
            RecoveryClassification::Recoverable
        );
        assert_eq!(
            coordinator.classify(&marker(vec![RecoveryClass::UnknownEffect])),
            RecoveryClassification::Unknown
        );
    }

    #[test]
    fn safe_mode_replays_and_claims_durably() {
        let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let mut callbacks = Callbacks::default();
        let marker = marker(vec![RecoveryClass::TransactionWrite]);
        assert!(matches!(
            coordinator.replay(&marker, &mut callbacks),
            Ok(RecoveryOutcome::Replayed { .. })
        ));
        assert!(matches!(
            coordinator.replay(&marker, &mut callbacks),
            Ok(RecoveryOutcome::AlreadyReplayed { .. })
        ));
        assert_eq!(callbacks.replays, 1);
    }

    #[test]
    fn unknown_effect_is_quarantined_without_callback() {
        let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let mut callbacks = Callbacks::default();
        let outcome = coordinator
            .replay(
                &marker(vec![RecoveryClass::DatabaseMigration]),
                &mut callbacks,
            )
            .expect("quarantine must be a successful classification");
        assert!(matches!(outcome, RecoveryOutcome::Quarantined { .. }));
        assert_eq!(callbacks.replays, 0);
        assert_eq!(callbacks.revalidations, 0);
    }

    #[test]
    fn safe_mode_revalidates_opaque_references() {
        let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let mut callbacks = Callbacks::default();
        let mut marker = marker(vec![RecoveryClass::CredentialRevocationPending]);
        marker.capability_set = vec!["cap-read".into()];
        marker.credential_set = vec!["cred-ref".into()];
        let outcome = coordinator
            .replay(&marker, &mut callbacks)
            .expect("revalidation callback must run");
        assert!(matches!(
            outcome,
            RecoveryOutcome::RevalidateRequired { .. }
        ));
        assert_eq!(callbacks.revalidations, 1);
        assert_eq!(callbacks.request.expect("request").credential_set.len(), 1);
    }

    #[test]
    fn mixed_quarantine_classes_take_precedence() {
        let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let mut callbacks = Callbacks::default();
        let outcome = coordinator
            .replay(
                &marker(vec![
                    RecoveryClass::CredentialRevocationPending,
                    RecoveryClass::UnknownEffect,
                ]),
                &mut callbacks,
            )
            .expect("mixed marker must quarantine");
        assert!(matches!(outcome, RecoveryOutcome::Quarantined { .. }));
        assert_eq!(callbacks.revalidations, 0);
    }

    #[test]
    fn inverted_epoch_is_corrupt_and_never_replayed() {
        let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let mut callbacks = Callbacks::default();
        let mut marker = marker(vec![RecoveryClass::JournalAppend]);
        marker.last_known_good_epoch = 2;
        let outcome = coordinator
            .replay(&marker, &mut callbacks)
            .expect("corrupt marker must quarantine");
        assert!(matches!(outcome, RecoveryOutcome::Quarantined { .. }));
        assert_eq!(callbacks.replays, 0);
    }

    #[test]
    fn clean_marker_does_not_claim_replay() {
        let mut coordinator =
            RecoveryCoordinator::with_mode(InMemoryStorage::new(), RecoveryMode::Safe);
        let mut clean = marker(Vec::new());
        clean.last_known_good_epoch = 1;
        let outcome = coordinator
            .replay(&clean, &mut NoopCallbacks)
            .expect("clean marker");
        assert!(matches!(outcome, RecoveryOutcome::Replayed { .. }));
        assert!(!coordinator.storage().audit().is_empty());
        let second = coordinator
            .replay(&clean, &mut NoopCallbacks)
            .expect("second clean marker");
        assert!(matches!(second, RecoveryOutcome::AlreadyReplayed { .. }));
    }
}
