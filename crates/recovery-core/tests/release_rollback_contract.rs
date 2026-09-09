//! Contract tests for the bounded release rollback boundary.
//!
//! These fixtures are transport-neutral and offline. They prove that a failed
//! boot/health/verification selects a verified known-good slot, that a revoked
//! version can never be restored, and that repeated rollback converges without
//! a loop. No updater, signer key, filesystem process or network is exercised.

use recovery_core::{
    HealthOutcome, ReleaseProof, RollbackCoordinator, RollbackDecision, RollbackError,
};

fn proof(version: u32) -> ReleaseProof {
    ReleaseProof::new(format!("artifact-{version}"), "sha256:deadbeef".to_string()).unwrap()
}

#[test]
// @spec:AC-3806
fn known_good_slot_is_recorded_for_verified_proof() {
    let mut coordinator = RollbackCoordinator::new();
    coordinator.record_known_good(proof(3), 1).unwrap();
    assert_eq!(coordinator.known_good_version(), Some(3));
}

#[test]
// @spec:AC-3807
fn failed_boot_or_health_selects_previous_known_good() {
    let mut coordinator = RollbackCoordinator::new();
    coordinator.record_known_good(proof(3), 1).unwrap();
    // A healthy current version continues without rollback.
    assert_eq!(
        coordinator.evaluate(4, HealthOutcome::Healthy, 0).unwrap(),
        RollbackDecision::Continue
    );
    // A failed boot selects the previous known-good (version 3).
    assert_eq!(
        coordinator
            .evaluate(4, HealthOutcome::FailedBoot, 0)
            .unwrap(),
        RollbackDecision::Rollback {
            to_version: 3,
            from_version: 4,
            epoch: 1,
        }
    );
}

#[test]
// @spec:AC-3808
fn revoked_version_cannot_be_restored() {
    let mut coordinator = RollbackCoordinator::new();
    coordinator.record_known_good(proof(3), 1).unwrap();
    coordinator.revoke(3);
    // A failure has no known-good target left, so it fails closed.
    assert_eq!(
        coordinator.evaluate(4, HealthOutcome::FailedHealth, 0),
        Err(RollbackError::NoKnownGoodAvailable)
    );
}

#[test]
// @spec:AC-3809
fn repeated_rollback_converges_without_loop() {
    let mut coordinator = RollbackCoordinator::new();
    coordinator.record_known_good(proof(3), 1).unwrap();
    // First failure rolls back from 5 to 3.
    assert_eq!(
        coordinator
            .evaluate(5, HealthOutcome::FailedVerification, 0)
            .unwrap(),
        RollbackDecision::Rollback {
            to_version: 3,
            from_version: 5,
            epoch: 1,
        }
    );
    // After reaching the known-good, a healthy current version continues.
    assert_eq!(
        coordinator.evaluate(3, HealthOutcome::Healthy, 0).unwrap(),
        RollbackDecision::Continue
    );
    // Exceeding bounded attempts blocks rather than looping.
    assert_eq!(
        coordinator.evaluate(9, HealthOutcome::FailedBoot, 100),
        Err(RollbackError::AttemptsExhausted)
    );
}

#[test]
// @spec:AC-3810
fn invalid_proof_is_rejected_at_construction() {
    // Empty artifact identity is rejected.
    assert!(ReleaseProof::new("", "sha256:deadbeef").is_err());
    // Empty digest is rejected.
    assert!(ReleaseProof::new("artifact-x", "").is_err());
    // Control characters in the identity are rejected.
    assert!(ReleaseProof::new("bad\u{0007}id", "sha256:deadbeef").is_err());
}

#[test]
// @spec:AC-3811
fn known_good_advances_strictly_to_newer_verified_version() {
    let mut coordinator = RollbackCoordinator::new();
    coordinator.record_known_good(proof(3), 1).unwrap();
    // A newer verified version becomes the known-good.
    coordinator.record_known_good(proof(5), 2).unwrap();
    assert_eq!(coordinator.known_good_version(), Some(5));
    // An older version does not advance the known-good slot.
    coordinator.record_known_good(proof(4), 3).unwrap();
    assert_eq!(coordinator.known_good_version(), Some(5));
}

#[test]
// @spec:AC-3812
fn incident_audit_records_rollback_decision_redacted() {
    let mut coordinator = RollbackCoordinator::new();
    coordinator.record_known_good(proof(3), 1).unwrap();
    coordinator
        .evaluate(4, HealthOutcome::FailedBoot, 0)
        .unwrap();
    let audit = coordinator.audit();
    // The audit records the decision without exposing proof material.
    assert!(!audit.is_empty());
    let rendered = audit.iter().map(|e| e.to_string()).collect::<String>();
    assert!(!rendered.contains("sha256:deadbeef"));
}

#[test]
// @spec:AC-3813
fn proof_material_is_redacted_from_observability() {
    let p = proof(7);
    // A proof's Debug form must not leak its raw digest.
    let rendered = format!("{:?}", p);
    assert!(!rendered.contains("sha256:deadbeef"));
}
