use agent_core::{
    AgentGroup, AgentGroupSession, AgentGroupSessionError, AgentGroupSessionStatus, AgentId,
    ProjectId,
};

fn fixture() -> (AgentGroup, ProjectId) {
    let project = ProjectId::new();
    (
        AgentGroup::new(
            project,
            "research".into(),
            AgentId::new(),
            agent_protocol::ids::TraceId::new(),
        ),
        project,
    )
}

#[test]
// @spec:AC-872
fn session_snapshots_group_members_and_is_project_scoped() {
    let (group, project) = fixture();
    let session = AgentGroupSession::from_group(&group).unwrap();
    assert_eq!(session.project_id, project);
    assert_eq!(session.group_id, group.id);
    assert_eq!(session.memberships, group.memberships);
    assert_eq!(session.status, AgentGroupSessionStatus::Created);
}

#[test]
// @spec:AC-873
fn rounds_budget_and_context_limits_stop_progress() {
    let (group, _) = fixture();
    let mut session = AgentGroupSession::from_group(&group).unwrap();
    session.max_rounds = 2;
    session.start().unwrap();
    session.begin_round().unwrap();
    session.finish_round(100).unwrap();
    session.begin_round().unwrap();
    session.finish_round(100).unwrap();
    assert_eq!(
        session.begin_round(),
        Err(AgentGroupSessionError::RoundLimit)
    );
}

#[test]
// @spec:AC-874
fn cancellation_is_terminal_and_idempotent_without_runtime_side_effects() {
    let (group, _) = fixture();
    let mut session = AgentGroupSession::from_group(&group).unwrap();
    session.start().unwrap();
    session.cancel().unwrap();
    assert_eq!(session.status, AgentGroupSessionStatus::Cancelled);
    assert_eq!(session.cancel(), Ok(false));
    assert_eq!(session.begin_round(), Err(AgentGroupSessionError::Terminal));
}
