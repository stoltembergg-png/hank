use agent_core::ids::{AgentId, ProjectId, SessionId};
use agent_runtime::agent_scheduler::{
    AgentDispatchInput, AgentDispatchRequest, AgentSchedulerError,
};

fn input() -> AgentDispatchInput {
    AgentDispatchInput {
        project_id: ProjectId::new(),
        agent_id: AgentId::new(),
        session_id: SessionId::new(),
        job_id: "job-a".into(),
        run_id: "run-a".into(),
        autonomy_allowed: true,
        budget_remaining: 10_000,
        max_tokens: 2_000,
        cancelled: false,
    }
}

// @spec:AC-1261
#[test]
fn active_agent_request_is_bounded_and_idempotent() {
    let request = AgentDispatchRequest::prepare(input()).unwrap();
    assert_eq!(request.max_tokens, 2_000);
    assert_eq!(
        request.idempotency_key,
        format!("scheduler:agent:{}:run-a", request.project_id)
    );
    assert_eq!(request.job_id, "job-a");
}

// @spec:AC-1262
#[test]
fn disabled_autonomy_and_budget_fail_before_provider_boundary() {
    let mut denied = input();
    denied.autonomy_allowed = false;
    assert_eq!(
        AgentDispatchRequest::prepare(denied),
        Err(AgentSchedulerError::AutonomyDenied)
    );
    let mut exhausted = input();
    exhausted.budget_remaining = 0;
    assert_eq!(
        AgentDispatchRequest::prepare(exhausted),
        Err(AgentSchedulerError::BudgetExhausted)
    );
}

// @spec:AC-1263
#[test]
fn cancellation_and_capability_free_request_are_bounded() {
    let mut cancelled = input();
    cancelled.cancelled = true;
    assert_eq!(
        AgentDispatchRequest::prepare(cancelled),
        Err(AgentSchedulerError::Cancelled)
    );
    let request = AgentDispatchRequest::prepare(input()).unwrap();
    assert!(!format!("{request:?}").contains("capability"));
}
