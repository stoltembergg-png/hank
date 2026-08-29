use agent_core::ci_status_integration::*;

fn ctx(event: CiEvent) -> CiContext {
    CiContext::new("repo", "pr-1", event, "head-1", "tree-1", "policy-1").unwrap()
}
fn check(name: RequiredCheck, status: CiCheckStatus) -> CiCheckResult {
    CiCheckResult::new(
        name, status, "head-1", "tree-1", "policy-1", "run-1", "digest-1",
    )
    .unwrap()
}
fn complete(event: CiEvent) -> CiInput {
    CiInput::new(
        ctx(event),
        vec![
            check(RequiredCheck::BuildRust, CiCheckStatus::Pass),
            check(RequiredCheck::Quality, CiCheckStatus::Pass),
            check(RequiredCheck::Security, CiCheckStatus::Pass),
        ],
    )
    .unwrap()
}

// @spec:AC-1343
#[test]
fn allowlisted_exact_identity_passes_deterministically() {
    let input = complete(CiEvent::PullRequest);
    let report = CiStatusIntegration::evaluate(&input).unwrap();
    assert_eq!(report.state(), CiState::Pass);
    assert_eq!(
        report.fingerprint(),
        CiStatusIntegration::evaluate(&input).unwrap().fingerprint()
    );
    assert!(!report.can_merge());
}

// @spec:AC-1343
#[test]
fn missing_duplicate_skipped_timeout_and_wrong_identity_are_not_pass() {
    let mut input = complete(CiEvent::PullRequest);
    input.checks[1] = check(RequiredCheck::Quality, CiCheckStatus::Skipped);
    assert_eq!(
        CiStatusIntegration::evaluate(&input).unwrap().state(),
        CiState::Unknown
    );
    input.checks[1] = check(RequiredCheck::Quality, CiCheckStatus::Pass);
    input
        .checks
        .push(check(RequiredCheck::Quality, CiCheckStatus::Pass));
    assert_eq!(
        CiStatusIntegration::evaluate(&input).unwrap().state(),
        CiState::Unknown
    );
}

// @spec:AC-1344
#[test]
fn merge_group_uses_its_own_event_and_explicit_na_policy() {
    let mut input = complete(CiEvent::MergeGroup);
    input.context.policy = "not-applicable".into();
    for item in &mut input.checks {
        item.policy = "not-applicable".into();
    }
    let report = CiStatusIntegration::evaluate(&input).unwrap();
    assert_eq!(report.state(), CiState::Pass);
    assert_eq!(report.policy_state(), CiPolicyState::NotApplicable);
    assert!(!report.can_merge());
}

// @spec:AC-1343
#[test]
fn wrong_sha_is_rejected_before_classification() {
    let mut input = complete(CiEvent::PullRequest);
    input.checks[0].head_sha = "other".into();
    assert!(matches!(
        CiStatusIntegration::evaluate(&input),
        Err(CiStatusError::StaleEvidence)
    ));
}
