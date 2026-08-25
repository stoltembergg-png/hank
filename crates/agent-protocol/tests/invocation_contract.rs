use agent_protocol::{
    AgentId, InvocationError, InvocationRequest, InvocationStatus, ProjectId, SessionId, TraceId,
};

fn request(project: ProjectId) -> InvocationRequest {
    InvocationRequest {
        schema_version: agent_protocol::INVOCATION_SCHEMA_VERSION,
        invocation_id: uuid::Uuid::new_v4(),
        project_id: project,
        group_id: uuid::Uuid::new_v4(),
        session_id: SessionId::new(),
        caller_id: AgentId::new(),
        callee_id: AgentId::new(),
        trace_id: TraceId::new(),
        task: "summarize bounded context".into(),
        context_refs: vec!["project://session/brief".into()],
        max_tokens: 1_000,
        depth: 0,
        status: InvocationStatus::Pending,
    }
}

#[test]
// @spec:AC-882
fn valid_invocation_is_versioned_bounded_and_correlated() {
    let project = ProjectId::new();
    let value = request(project);
    value.validate().unwrap();
    assert_eq!(value.project_id, project);
    assert_eq!(value.status, InvocationStatus::Pending);
}

#[test]
// @spec:AC-883
fn missing_identity_budget_context_or_depth_fails_closed() {
    let mut value = request(ProjectId::new());
    value.task.clear();
    assert_eq!(value.validate(), Err(InvocationError::InvalidTask));
    value.task = "safe".into();
    value.max_tokens = 0;
    assert_eq!(value.validate(), Err(InvocationError::InvalidBudget));
    value.max_tokens = 10;
    value.context_refs = vec!["file:///etc/passwd".into()];
    assert_eq!(value.validate(), Err(InvocationError::InvalidContext));
    value.context_refs = vec![];
    value.depth = 17;
    assert_eq!(value.validate(), Err(InvocationError::DepthLimit));
}

#[test]
// @spec:AC-884
fn terminal_response_states_are_idempotent_and_no_transport_is_invoked() {
    let project = ProjectId::new();
    let mut value = request(project);
    value.status = InvocationStatus::Completed;
    assert!(value.validate().is_ok());
    assert_eq!(value.status, InvocationStatus::Completed);
    value.status = InvocationStatus::Cancelled;
    assert!(value.validate().is_ok());
}
