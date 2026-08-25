use agent_core::{AgentGroup, AgentGroupSession, AgentId};
use agent_protocol::{InvocationError, InvocationStatus, SessionId};
use tool_core::{DelegationError, DelegationTool, PendingDelegationLedger};

fn session() -> AgentGroupSession {
    let project = agent_core::ProjectId::new();
    let mut group = AgentGroup::new(
        project,
        "research".into(),
        AgentId::new(),
        agent_protocol::TraceId::new(),
    );
    let member = AgentId::new();
    let owner = group.owner_id;
    group
        .add_member(member, project, owner, "worker".into())
        .unwrap();
    AgentGroupSession::from_group(&group).unwrap()
}

#[test]
// @spec:AC-887
fn delegation_builds_pending_request_for_valid_member_without_execution() {
    let value = session();
    let caller = value.memberships[0].agent_id;
    let callee = value.memberships[1].agent_id;
    let request = DelegationTool::build(
        &value,
        caller,
        callee,
        "summarize the bounded context".into(),
        vec!["project://session/brief".into()],
        100,
        uuid::Uuid::new_v4(),
    )
    .unwrap();
    assert_eq!(request.status, InvocationStatus::Pending);
    assert_eq!(request.session_id, SessionId::from(value.id));
}

#[test]
// @spec:AC-888
fn unknown_target_oversized_task_or_invalid_context_is_denied() {
    let value = session();
    let caller = value.memberships[0].agent_id;
    assert_eq!(
        DelegationTool::build(
            &value,
            caller,
            AgentId::new(),
            "safe".into(),
            vec![],
            100,
            uuid::Uuid::new_v4(),
        ),
        Err(DelegationError::TargetNotMember)
    );
    let callee = value.memberships[1].agent_id;
    assert_eq!(
        DelegationTool::build(
            &value,
            caller,
            callee,
            "x".repeat(5000),
            vec![],
            100,
            uuid::Uuid::new_v4(),
        ),
        Err(DelegationError::InvalidInvocation(
            InvocationError::InvalidTask
        ))
    );
}

#[test]
// @spec:AC-889
fn pending_ledger_dedupes_and_cancels_without_worker_call() {
    let value = session();
    let caller = value.memberships[0].agent_id;
    let callee = value.memberships[1].agent_id;
    let id = uuid::Uuid::new_v4();
    let request =
        DelegationTool::build(&value, caller, callee, "safe".into(), vec![], 100, id).unwrap();
    let mut ledger = PendingDelegationLedger::default();
    assert!(ledger.register(request.clone()));
    assert!(!ledger.register(request));
    assert!(ledger.cancel(id));
    assert!(!ledger.cancel(id));
    assert!(ledger.pending.is_empty());
}
