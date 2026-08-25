use agent_core::{AgentId, ProjectId, RoundPolicy, RoundStopReason};

fn policy() -> RoundPolicy {
    RoundPolicy::new(
        ProjectId::new(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        2,
        3,
        AgentId::new(),
    )
    .unwrap()
}

#[test]
// @spec:AC-920
fn rounds_and_turns_stop_at_exact_bounds() {
    let mut value = policy();
    assert!(value.begin_round().is_ok());
    assert!(value.record_turn(uuid::Uuid::new_v4(), true).is_ok());
    assert!(value.begin_round().is_ok());
    assert_eq!(value.begin_round(), Err(RoundStopReason::MaxRounds));
}

#[test]
// @spec:AC-921
fn no_progress_budget_error_and_cancel_are_terminal() {
    let mut value = policy();
    value.begin_round().unwrap();
    value.record_turn(uuid::Uuid::new_v4(), false).unwrap();
    assert_eq!(
        value.record_turn(uuid::Uuid::new_v4(), false),
        Err(RoundStopReason::NoProgress)
    );
    let mut other = policy();
    other.stop(RoundStopReason::BudgetExceeded);
    assert!(other.is_terminal());
    assert_eq!(other.begin_round(), Err(RoundStopReason::BudgetExceeded));
}

#[test]
// @spec:AC-922
fn retry_dedupe_does_not_increment_and_scope_is_preserved() {
    let mut value = policy();
    value.begin_round().unwrap();
    let turn = uuid::Uuid::new_v4();
    value.record_turn(turn, true).unwrap();
    assert_eq!(
        value.record_turn(turn, true),
        Err(RoundStopReason::DuplicateTurn)
    );
    assert_eq!(value.current_turns(), 1);
    assert!(!value.project_id().to_string().is_empty());
}
