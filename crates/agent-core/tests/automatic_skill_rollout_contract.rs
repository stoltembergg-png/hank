use agent_core::automatic_skill_rollout::*;

fn valid() -> RolloutRequest {
    RolloutRequest::new(
        "candidate-1",
        "skill-2",
        "project-1",
        true,
        true,
        true,
        true,
        true,
    )
    .unwrap()
}

// @spec:AC-1369
#[test]
fn valid_evidence_allows_bounded_canary_only() {
    let rollout = Rollout::evaluate(valid(), Health::Stable).unwrap();
    assert_eq!(rollout.status(), RolloutStatus::CanaryReady);
    assert_eq!(rollout.scope(), RolloutScope::ProjectCanary);
    assert!(!rollout.global_activation());
}

// @spec:AC-1370
#[test]
fn missing_evidence_scope_violation_health_failure_and_kill_switch_stop() {
    let mut missing = valid();
    missing.all_evidence = false;
    assert_eq!(
        Rollout::evaluate(missing, Health::Stable).unwrap().status(),
        RolloutStatus::Blocked
    );
    let mut scope = valid();
    scope.scope_allowed = false;
    assert_eq!(
        Rollout::evaluate(scope, Health::Stable).unwrap().status(),
        RolloutStatus::Blocked
    );
    assert_eq!(
        Rollout::evaluate(valid(), Health::Failed).unwrap().status(),
        RolloutStatus::Stopped
    );
    assert_eq!(
        Rollout::evaluate(valid(), Health::KillSwitch)
            .unwrap()
            .status(),
        RolloutStatus::Stopped
    );
}
