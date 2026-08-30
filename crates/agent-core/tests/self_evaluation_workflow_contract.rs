use agent_core::self_evaluation_workflow::*;

fn valid() -> EvaluationRequest {
    EvaluationRequest::new(
        "candidate-1",
        "project-1",
        "owner-1",
        "version-1",
        "sha-1",
        true,
        true,
        true,
    )
    .unwrap()
}

// @spec:AC-1353
#[test]
fn exact_snapshot_requires_all_stages() {
    let record = SelfEvaluationWorkflow::start(valid()).unwrap();
    assert_eq!(record.status(), DecisionStatus::Blocked);
    assert_eq!(
        record.required_stages(),
        &[Stage::Validation, Stage::Tests, Stage::Security]
    );
    assert_eq!(record.candidate_id(), "candidate-1");
    assert!(!record.can_activate());
}

// @spec:AC-1354
#[test]
fn missing_stage_or_crash_stays_blocked_and_approved_is_not_activation() {
    let mut request = valid();
    request.tests_present = false;
    let blocked = SelfEvaluationWorkflow::start(request).unwrap();
    assert_eq!(blocked.status(), DecisionStatus::Blocked);
    assert!(blocked.reason().contains("tests"));
    assert!(!blocked.can_activate());
    let crashed = SelfEvaluationWorkflow::from_outcome(valid(), EvaluatorOutcome::Crashed).unwrap();
    assert_eq!(crashed.status(), DecisionStatus::Blocked);
    assert_eq!(crashed.snapshot_sha(), "sha-1");
    let approved =
        SelfEvaluationWorkflow::from_outcome(valid(), EvaluatorOutcome::Approved).unwrap();
    assert_eq!(approved.status(), DecisionStatus::Approved);
    assert!(!approved.can_activate());
}
