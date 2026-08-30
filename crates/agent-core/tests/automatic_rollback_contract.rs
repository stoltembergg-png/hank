use agent_core::automatic_rollback::*;

fn valid() -> RollbackRequest {
    RollbackRequest::new("active-2", "lkg-1", "policy-1", RollbackTrigger::Crash).unwrap()
}

// @spec:AC-1367
#[test]
fn rollback_restores_lkg_quarantines_candidate_and_is_idempotent() {
    let first = Rollback::execute(valid()).unwrap();
    let second = Rollback::execute(valid()).unwrap();
    assert_eq!(first.status(), RollbackStatus::Recovered);
    assert_eq!(first.previous_version(), "lkg-1");
    assert!(first.quarantined());
    assert_eq!(first.rollback_id(), second.rollback_id());
    assert!(!first.can_activate());
}

// @spec:AC-1368
#[test]
fn missing_lkg_and_policy_mismatch_are_blocked_without_activation() {
    let missing = RollbackRequest::new("active-2", "", "policy-1", RollbackTrigger::Regression);
    assert!(matches!(missing, Err(RollbackError::NoLastKnownGood)));
    let mut mismatch = valid();
    mismatch.policy_revision = "other".into();
    assert!(matches!(
        Rollback::execute(mismatch),
        Err(RollbackError::PolicyMismatch)
    ));
}
