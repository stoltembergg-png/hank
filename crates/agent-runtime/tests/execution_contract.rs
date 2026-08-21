use agent_core::ids::{AgentId, SessionId};
use agent_runtime::execution::{
    Execution, ExecutionConcurrency, ExecutionError, ExecutionEvent, ExecutionState,
};

fn execution() -> Execution {
    Execution::new(
        "execution-1",
        SessionId::new(),
        AgentId::new(),
        "corr-1",
        1,
        100,
        10_000,
    )
    .unwrap()
}

#[test]
fn success_path_has_deterministic_non_overwritable_terminal_state() {
    let mut execution = execution();
    assert_eq!(execution.state(), ExecutionState::Preparing);
    execution.apply(ExecutionEvent::Start).unwrap();
    execution
        .apply(ExecutionEvent::ProviderInvoked("invocation-1".into()))
        .unwrap();
    execution.apply(ExecutionEvent::Completed).unwrap();
    assert_eq!(execution.state(), ExecutionState::Completed);
    assert!(matches!(
        execution.apply(ExecutionEvent::Cancelled),
        Err(ExecutionError::TerminalState)
    ));
    assert!(execution.terminal_reason().is_some());
}

#[test]
fn illegal_transitions_and_duplicate_invocations_fail_without_mutation() {
    let mut execution = execution();
    assert!(matches!(
        execution.apply(ExecutionEvent::Completed),
        Err(ExecutionError::IllegalTransition { .. })
    ));
    execution.apply(ExecutionEvent::Start).unwrap();
    execution
        .apply(ExecutionEvent::ProviderInvoked("invocation-1".into()))
        .unwrap();
    assert!(matches!(
        execution.apply(ExecutionEvent::ProviderInvoked("invocation-1".into())),
        Err(ExecutionError::DuplicateInvocation)
    ));
    assert_eq!(execution.state(), ExecutionState::Running);
}

#[test]
fn streaming_cancel_error_and_generation_fencing_are_explicit() {
    let mut execution = execution();
    execution.apply(ExecutionEvent::Start).unwrap();
    execution.apply(ExecutionEvent::StreamStarted).unwrap();
    assert_eq!(execution.state(), ExecutionState::Streaming);
    execution
        .apply(ExecutionEvent::Failed("provider_unavailable".into()))
        .unwrap();
    assert_eq!(execution.state(), ExecutionState::Failed);
    assert!(matches!(
        execution.accept_generation(2),
        Err(ExecutionError::StaleGeneration)
    ));
    assert_eq!(execution.generation(), 1);
}

#[test]
fn cancellation_wins_race_and_terminal_state_is_exactly_once() {
    let mut execution = execution();
    execution.apply(ExecutionEvent::Start).unwrap();
    execution.apply(ExecutionEvent::Cancelled).unwrap();
    assert_eq!(execution.state(), ExecutionState::Cancelled);
    assert!(matches!(
        execution.apply(ExecutionEvent::Completed),
        Err(ExecutionError::TerminalState)
    ));
}

#[test]
fn budget_is_bounded_and_failure_is_redacted() {
    let mut execution = execution();
    execution.apply(ExecutionEvent::Start).unwrap();
    assert!(execution.record_usage(101, 0).is_err());
    assert_eq!(execution.state(), ExecutionState::Failed);
    let debug = format!("{execution:?}");
    assert!(!debug.contains("api_key"));
    assert!(!debug.contains("secret"));
}

#[test]
fn bounded_concurrency_rejects_extra_turn_and_releases_after_drop() {
    let limiter = ExecutionConcurrency::new(1).unwrap();
    let lease = limiter.try_acquire().unwrap();
    assert!(matches!(
        limiter.try_acquire(),
        Err(ExecutionError::ConcurrencyLimit)
    ));
    drop(lease);
    assert!(limiter.try_acquire().is_ok());
}

#[test]
fn snapshot_recovery_preserves_terminality_and_identity() {
    let mut execution = execution();
    execution.apply(ExecutionEvent::Start).unwrap();
    execution.apply(ExecutionEvent::Completed).unwrap();
    let snapshot = execution.snapshot();
    let recovered = Execution::restore(snapshot).unwrap();
    assert_eq!(recovered.state(), ExecutionState::Completed);
    assert!(matches!(
        recovered
            .clone()
            .apply(ExecutionEvent::Failed("late".into())),
        Err(ExecutionError::TerminalState)
    ));
    let debug = format!("{recovered:?}");
    assert!(debug.contains("execution-1"));
    assert!(!debug.contains("late"));
}
