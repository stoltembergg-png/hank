//! Startup recovery coordinator.
//!
//! The coordinator classifies a bounded marker before any resume attempt,
//! refuses ambiguous/corrupt work in safe mode, and remembers replayed
//! recovery IDs for idempotence. It does not execute production effects;
//! callbacks are explicit dependency-injection seams for later adapters.

use crate::marker::{RecoveryClass, RecoveryMarker, RevalidationRequest};
use crate::storage::{RecoveryAuditEntry, RecoveryError, RecoveryStorage};
use std::collections::{BTreeMap, BTreeSet};

/// Coordinator default mode. Safe mode is fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    Safe,
    Resume,
}

/// Classification of the previous process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryClassification {
    Clean,
    Recoverable,
    Unknown,
    Corrupt,
}

/// Result of an attempted recovery replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Replayed {
        recovery_id: String,
    },
    AlreadyReplayed {
        recovery_id: String,
    },
    Quarantined {
        recovery_id: String,
    },
    RevalidateRequired {
        recovery_id: String,
        request: RevalidationRequest,
    },
}

/// Safe, redacted representation of a marker for crash bundles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedCrashBundle {
    pub recovery_id: String,
    pub epoch: u64,
    pub last_known_good_epoch: u64,
    pub pending_classes: BTreeSet<RecoveryClass>,
    pub last_safe_action: String,
}

impl RedactedCrashBundle {
    pub fn to_json(&self) -> String {
        let classes = self
            .pending_classes
            .iter()
            .map(|class| format!("\"{class:?}\""))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"recovery_id\":{},\"epoch\":{},\"last_known_good_epoch\":{},\"pending_classes\":[{}],\"last_safe_action\":\"[REDACTED]\"}}",
            json_string(&self.recovery_id),
            self.epoch,
            self.last_known_good_epoch,
            classes,
        )
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str("\\uFFFD"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Callbacks for explicit recovery effects. The coordinator never invents
/// or performs effects by itself.
pub trait RecoveryCallbacks {
    fn on_replay(&mut self, marker: &RecoveryMarker) -> Result<(), RecoveryError>;
    fn on_revalidate(&mut self, request: &RevalidationRequest) -> Result<(), RecoveryError>;
}

/// No-op callback implementation useful for classification-only callers.
#[derive(Debug, Default)]
pub struct NoopCallbacks;

impl RecoveryCallbacks for NoopCallbacks {
    fn on_replay(&mut self, _marker: &RecoveryMarker) -> Result<(), RecoveryError> {
        Ok(())
    }

    fn on_revalidate(&mut self, _request: &RevalidationRequest) -> Result<(), RecoveryError> {
        Ok(())
    }
}

/// In-memory coordinator state. A future persistent adapter may replace the
/// replay ledger, but the state transitions and bounds remain the same.
pub struct RecoveryCoordinator<S> {
    storage: S,
    mode: RecoveryMode,
    replayed: BTreeMap<String, RecoveryOutcome>,
}

impl<S: RecoveryStorage> RecoveryCoordinator<S> {
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            mode: RecoveryMode::Safe,
            replayed: BTreeMap::new(),
        }
    }

    pub fn with_mode(storage: S, mode: RecoveryMode) -> Self {
        Self {
            storage,
            mode,
            replayed: BTreeMap::new(),
        }
    }

    pub fn mode(&self) -> RecoveryMode {
        self.mode
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    pub fn classify(&self, marker: &RecoveryMarker) -> RecoveryClassification {
        if marker.exceeds_bound() || marker.recovery_id.trim().is_empty() {
            return RecoveryClassification::Corrupt;
        }
        if marker.is_clean() {
            return RecoveryClassification::Clean;
        }
        if marker.epoch == 0 && !marker.pending_classes.is_empty() {
            return RecoveryClassification::Corrupt;
        }
        if marker
            .pending_classes
            .contains(&RecoveryClass::DatabaseMigration)
        {
            return RecoveryClassification::Corrupt;
        }
        if marker
            .pending_classes
            .iter()
            .any(|class| class.is_quarantined())
        {
            return RecoveryClassification::Unknown;
        }
        if marker.pending_classes.iter().all(|class| {
            matches!(
                class,
                RecoveryClass::TransactionWrite | RecoveryClass::JournalAppend
            )
        }) {
            return RecoveryClassification::Recoverable;
        }
        RecoveryClassification::Unknown
    }

    /// Replays a marker through explicit callbacks. The exact same recovery
    /// ID is idempotent, including when it was quarantined or required
    /// revalidation. A successful replay is recorded in storage audit.
    pub fn replay<C: RecoveryCallbacks>(
        &mut self,
        marker: &RecoveryMarker,
        callbacks: &mut C,
    ) -> Result<RecoveryOutcome, RecoveryError> {
        if marker.exceeds_bound() || marker.recovery_id.trim().is_empty() {
            let outcome = self.quarantine(marker);
            self.record_outcome(marker, &outcome)?;
            if !marker.recovery_id.trim().is_empty() {
                self.replayed
                    .insert(marker.recovery_id.clone(), outcome.clone());
            }
            return Ok(outcome);
        }
        if let Some(previous) = self.replayed.get(&marker.recovery_id) {
            return Ok(match previous {
                RecoveryOutcome::Replayed { recovery_id }
                | RecoveryOutcome::AlreadyReplayed { recovery_id }
                | RecoveryOutcome::Quarantined { recovery_id }
                | RecoveryOutcome::RevalidateRequired { recovery_id, .. } => {
                    RecoveryOutcome::AlreadyReplayed {
                        recovery_id: recovery_id.clone(),
                    }
                }
            });
        }

        let revalidation_classes = marker.revalidatable_classes();
        if !revalidation_classes.is_empty()
            && marker.quarantined_classes().is_empty()
            && marker.epoch > 0
        {
            let request = marker.revalidation_request();
            callbacks.on_revalidate(&request)?;
            let outcome = RecoveryOutcome::RevalidateRequired {
                recovery_id: marker.recovery_id.clone(),
                request,
            };
            self.record_outcome(marker, &outcome)?;
            self.replayed
                .insert(marker.recovery_id.clone(), outcome.clone());
            return Ok(outcome);
        }

        let classification = self.classify(marker);
        let outcome = match classification {
            RecoveryClassification::Clean => RecoveryOutcome::Replayed {
                recovery_id: marker.recovery_id.clone(),
            },
            RecoveryClassification::Unknown | RecoveryClassification::Corrupt => {
                self.quarantine(marker)
            }
            RecoveryClassification::Recoverable => {
                callbacks.on_replay(marker)?;
                RecoveryOutcome::Replayed {
                    recovery_id: marker.recovery_id.clone(),
                }
            }
        };

        self.record_outcome(marker, &outcome)?;
        self.replayed
            .insert(marker.recovery_id.clone(), outcome.clone());
        Ok(outcome)
    }

    pub fn redact_crash_bundle(&self, marker: &RecoveryMarker) -> RedactedCrashBundle {
        RedactedCrashBundle {
            recovery_id: marker.recovery_id.clone(),
            epoch: marker.epoch,
            last_known_good_epoch: marker.last_known_good_epoch,
            pending_classes: marker.pending_classes.iter().copied().collect(),
            last_safe_action: "[REDACTED]".into(),
        }
    }

    fn quarantine(&self, marker: &RecoveryMarker) -> RecoveryOutcome {
        RecoveryOutcome::Quarantined {
            recovery_id: marker.recovery_id.clone(),
        }
    }

    fn record_outcome(
        &mut self,
        marker: &RecoveryMarker,
        outcome: &RecoveryOutcome,
    ) -> Result<(), RecoveryError> {
        let (kind, quarantined_ids) = match outcome {
            RecoveryOutcome::Replayed { .. } => ("replayed", Vec::new()),
            RecoveryOutcome::AlreadyReplayed { .. } => ("already_replayed", Vec::new()),
            RecoveryOutcome::Quarantined { recovery_id } => {
                ("quarantined", vec![recovery_id.clone()])
            }
            RecoveryOutcome::RevalidateRequired { .. } => ("revalidate_required", Vec::new()),
        };
        let entry = RecoveryAuditEntry {
            recovery_id: marker.recovery_id.clone(),
            outcome_kind: kind.into(),
            quarantined_ids,
            redacted_summary: self.redact_crash_bundle(marker).to_json(),
        };
        self.storage.append_audit(&entry)
    }
}

impl<S: RecoveryStorage> RecoveryCoordinator<S> {
    /// Loads the current marker from storage and classifies it. A missing
    /// marker means a clean first run.
    pub fn startup_classification(&self) -> RecoveryClassification {
        self.storage
            .load_marker()
            .map(|marker| self.classify(&marker))
            .unwrap_or(RecoveryClassification::Clean)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marker::RecoveryClass;
    use crate::storage::InMemoryStorage;

    fn marker(id: &str, epoch: u64, classes: &[RecoveryClass]) -> RecoveryMarker {
        RecoveryMarker {
            recovery_id: id.into(),
            epoch,
            last_known_good_epoch: 0,
            pending_classes: classes.to_vec(),
            actor: "actor-opaque".into(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: "do not expose this value".into(),
        }
    }

    #[test]
    fn classifies_clean_recoverable_unknown_and_corrupt() {
        let coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        assert_eq!(
            coordinator.classify(&RecoveryMarker {
                recovery_id: "clean".into(),
                epoch: 2,
                last_known_good_epoch: 2,
                pending_classes: Vec::new(),
                actor: String::new(),
                capability_set: Vec::new(),
                credential_set: Vec::new(),
                last_safe_action: String::new(),
            }),
            RecoveryClassification::Clean
        );
        assert_eq!(
            coordinator.classify(&marker("recover", 2, &[RecoveryClass::JournalAppend])),
            RecoveryClassification::Recoverable
        );
        assert_eq!(
            coordinator.classify(&marker(
                "unknown",
                2,
                &[RecoveryClass::ToolExecutionPending]
            )),
            RecoveryClassification::Unknown
        );
        assert_eq!(
            coordinator.classify(&marker("corrupt", 2, &[RecoveryClass::DatabaseMigration])),
            RecoveryClassification::Corrupt
        );
    }

    #[test]
    fn crash_bundle_redacts_last_safe_action() {
        let coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        let marker = marker("bundle", 2, &[RecoveryClass::JournalAppend]);
        let json = coordinator.redact_crash_bundle(&marker).to_json();
        assert!(json.contains("[REDACTED]"));
        assert!(!json.contains("do not expose this value"));
        assert!(json.contains("JournalAppend"));
    }

    #[test]
    fn missing_marker_is_clean_first_run() {
        let coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
        assert_eq!(
            coordinator.startup_classification(),
            RecoveryClassification::Clean
        );
    }
}
