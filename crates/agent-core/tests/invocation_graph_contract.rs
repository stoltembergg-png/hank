use agent_core::{InvocationGraph, InvocationGraphError, InvocationNodeStatus, ProjectId};
use agent_protocol::{AgentId, InvocationRequest, InvocationStatus, SessionId};

fn request(project: ProjectId, id: uuid::Uuid, parent: Option<uuid::Uuid>) -> InvocationRequest {
    InvocationRequest {
        schema_version: agent_protocol::INVOCATION_SCHEMA_VERSION,
        invocation_id: id,
        project_id: project,
        group_id: uuid::Uuid::new_v4(),
        session_id: SessionId::new(),
        caller_id: AgentId::new(),
        callee_id: AgentId::new(),
        trace_id: agent_protocol::TraceId::new(),
        task: "safe".into(),
        context_refs: vec![],
        max_tokens: 100,
        depth: parent.map(|_| 1).unwrap_or(0),
        status: InvocationStatus::Pending,
    }
}

#[test]
// @spec:AC-892
fn graph_registers_pending_node_and_parent_with_scope() {
    let project = ProjectId::new();
    let mut graph = InvocationGraph::default();
    let root = uuid::Uuid::new_v4();
    graph.register(request(project, root, None), None).unwrap();
    let child = uuid::Uuid::new_v4();
    graph
        .register(request(project, child, Some(root)), Some(root))
        .unwrap();
    assert_eq!(graph.status(child), Some(InvocationNodeStatus::Pending));
}

#[test]
// @spec:AC-893
fn missing_parent_wrong_project_and_duplicate_fail_closed() {
    let project = ProjectId::new();
    let mut graph = InvocationGraph::default();
    let id = uuid::Uuid::new_v4();
    assert_eq!(
        graph.register(request(project, id, None), Some(uuid::Uuid::new_v4())),
        Err(InvocationGraphError::MissingParent)
    );
    graph.register(request(project, id, None), None).unwrap();
    assert_eq!(
        graph.register(request(project, id, None), None),
        Err(InvocationGraphError::Duplicate)
    );
    assert_eq!(
        graph.register(
            request(ProjectId::new(), uuid::Uuid::new_v4(), Some(id)),
            Some(id)
        ),
        Err(InvocationGraphError::ScopeMismatch)
    );
}

#[test]
// @spec:AC-894
fn pending_node_can_cancel_idempotently_without_execution() {
    let project = ProjectId::new();
    let mut graph = InvocationGraph::default();
    let id = uuid::Uuid::new_v4();
    graph.register(request(project, id, None), None).unwrap();
    assert!(graph.cancel(id));
    assert!(!graph.cancel(id));
    assert_eq!(graph.status(id), Some(InvocationNodeStatus::Cancelled));
}
