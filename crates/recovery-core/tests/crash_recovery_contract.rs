use std::collections::BTreeSet;

use recovery_core::{
    InMemoryStorage, RecoveryCallbacks, RecoveryClass, RecoveryClassification, RecoveryCoordinator,
    RecoveryError, RecoveryMarker, RecoveryMode, RecoveryOutcome, RevalidationRequest,
    MAX_PENDING_CLASSES,
};

#[derive(Default)]
struct Recorder {
    replay_count: usize,
    revalidate_count: usize,
    request: Option<RevalidationRequest>,
}

impl RecoveryCallbacks for Recorder {
    fn on_replay(&mut self, _marker: &RecoveryMarker) -> Result<(), RecoveryError> {
        self.replay_count += 1;
        Ok(())
    }

    fn on_revalidate(&mut self, request: &RevalidationRequest) -> Result<(), RecoveryError> {
        self.revalidate_count += 1;
        self.request = Some(request.clone());
        Ok(())
    }
}

fn marker(id: &str, classes: Vec<RecoveryClass>) -> RecoveryMarker {
    RecoveryMarker {
        project_id: "project-default".into(),
        recovery_id: id.into(),
        epoch: 4,
        last_known_good_epoch: 3,
        pending_classes: classes,
        actor: "actor-opaque".into(),
        capability_set: vec!["cap-read".into()],
        credential_set: vec!["cred-ref-1".into()],
        last_safe_action: "sensitive action must not escape".into(),
    }
}

#[test]
fn ac_1501_classifies_startup_states() {
    // @spec:AC-1501
    let coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
    assert_eq!(
        coordinator.classify(&RecoveryMarker {
            project_id: "project-default".into(),
            recovery_id: "clean".into(),
            epoch: 4,
            last_known_good_epoch: 4,
            pending_classes: Vec::new(),
            actor: String::new(),
            capability_set: Vec::new(),
            credential_set: Vec::new(),
            last_safe_action: String::new(),
        }),
        RecoveryClassification::Clean
    );
    assert_eq!(
        coordinator.classify(&marker(
            "recoverable",
            vec![
                RecoveryClass::TransactionWrite,
                RecoveryClass::JournalAppend
            ],
        )),
        RecoveryClassification::Recoverable
    );
    assert_eq!(
        coordinator.classify(&marker("unknown", vec![RecoveryClass::UnknownEffect])),
        RecoveryClassification::Unknown
    );
    assert_eq!(
        coordinator.classify(&marker("corrupt", vec![RecoveryClass::CorruptMarker])),
        RecoveryClassification::Corrupt
    );
}

#[test]
fn ac_1502_replay_is_idempotent() {
    // @spec:AC-1502
    let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
    let mut callbacks = Recorder::default();
    let marker = marker("replay-once", vec![RecoveryClass::JournalAppend]);

    assert_eq!(
        coordinator.replay(&marker, &mut callbacks).unwrap(),
        RecoveryOutcome::Replayed {
            recovery_id: "replay-once".into(),
        }
    );
    assert_eq!(
        coordinator.replay(&marker, &mut callbacks).unwrap(),
        RecoveryOutcome::AlreadyReplayed {
            recovery_id: "replay-once".into(),
        }
    );
    assert_eq!(callbacks.replay_count, 1);
    assert_eq!(coordinator.storage().audit().len(), 1);
}

#[test]
fn ac_1503_safe_mode_quarantines_unknown_state() {
    // @spec:AC-1503
    let mut coordinator =
        RecoveryCoordinator::with_mode(InMemoryStorage::new(), RecoveryMode::Safe);
    let mut callbacks = Recorder::default();
    let marker = marker("unknown-effect", vec![RecoveryClass::UnknownEffect]);

    let outcome = coordinator.replay(&marker, &mut callbacks).unwrap();
    assert_eq!(
        outcome,
        RecoveryOutcome::Quarantined {
            recovery_id: "unknown-effect".into(),
            classes: BTreeSet::from([RecoveryClass::UnknownEffect]),
        }
    );
    assert_eq!(callbacks.replay_count, 0);
    assert_eq!(callbacks.revalidate_count, 0);
}

#[test]
fn ac_1504_crash_bundle_keeps_only_non_sensitive_fields() {
    // @spec:AC-1504
    let coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
    let marker = marker("bundle-id", vec![RecoveryClass::JournalAppend]);
    let json = coordinator.redacted_bundle(&marker).to_json();

    assert!(json.contains("bundle-id"));
    assert!(json.contains("\"epoch\":4"));
    assert!(json.contains("\"last_known_good_epoch\":3"));
    assert!(json.contains("JournalAppend"));
    assert!(json.contains("[REDACTED]"));
    assert!(!json.contains("sensitive action must not escape"));
    assert!(!json.contains("actor-opaque"));
    assert!(!json.contains("cap-read"));
    assert!(!json.contains("cred-ref-1"));
}

#[test]
fn ac_1505_revalidation_is_required_and_receives_opaque_sets() {
    // @spec:AC-1505
    let mut coordinator =
        RecoveryCoordinator::with_mode(InMemoryStorage::new(), RecoveryMode::Safe);
    let mut callbacks = Recorder::default();
    let marker = RecoveryMarker {
        project_id: "project-default".into(),
        recovery_id: "stale-privileges".into(),
        epoch: 4,
        last_known_good_epoch: 3,
        pending_classes: vec![
            RecoveryClass::CredentialRevocationPending,
            RecoveryClass::CapabilityRotationPending,
        ],
        actor: "actor-opaque".into(),
        capability_set: vec!["cap-read".into(), "cap-write".into()],
        credential_set: vec!["cred-ref-1".into(), "cred-ref-2".into()],
        last_safe_action: "redact me".into(),
    };

    let outcome = coordinator.replay(&marker, &mut callbacks).unwrap();
    let request = match outcome {
        RecoveryOutcome::RevalidateRequired { request, .. } => request,
        other => panic!("expected revalidation, got {other:?}"),
    };
    assert_eq!(callbacks.revalidate_count, 1);
    assert_eq!(request, callbacks.request.unwrap());
    assert_eq!(
        request.capability_set,
        BTreeSet::from(["cap-read".into(), "cap-write".into()])
    );
    assert_eq!(
        request.credential_set,
        BTreeSet::from(["cred-ref-1".into(), "cred-ref-2".into()])
    );
}

#[test]
fn mixed_revalidation_and_quarantine_is_fail_closed() {
    let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
    let mut callbacks = Recorder::default();
    let marker = marker(
        "mixed",
        vec![
            RecoveryClass::CapabilityRotationPending,
            RecoveryClass::DatabaseMigration,
        ],
    );
    assert!(matches!(
        coordinator.replay(&marker, &mut callbacks).unwrap(),
        RecoveryOutcome::Quarantined { .. }
    ));
    assert_eq!(callbacks.revalidate_count, 0);
}

#[test]
fn inverted_epoch_is_corrupt_and_never_replayed() {
    let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
    let mut callbacks = Recorder::default();
    let mut marker = marker("inverted", vec![RecoveryClass::JournalAppend]);
    marker.last_known_good_epoch = 5;
    assert_eq!(
        coordinator.classify(&marker),
        RecoveryClassification::Corrupt
    );
    assert!(matches!(
        coordinator.replay(&marker, &mut callbacks).unwrap(),
        RecoveryOutcome::Quarantined { .. }
    ));
    assert_eq!(callbacks.replay_count, 0);
}

#[test]
fn bounded_marker_is_corrupt_and_never_replayed() {
    let mut coordinator = RecoveryCoordinator::new(InMemoryStorage::new());
    let mut callbacks = Recorder::default();
    let marker = marker(
        "too-many-classes",
        vec![RecoveryClass::JournalAppend; MAX_PENDING_CLASSES + 1],
    );

    assert_eq!(
        coordinator.classify(&marker),
        RecoveryClassification::Corrupt
    );
    assert!(matches!(
        coordinator.replay(&marker, &mut callbacks).unwrap(),
        RecoveryOutcome::Quarantined { .. }
    ));
    assert_eq!(callbacks.replay_count, 0);
}
