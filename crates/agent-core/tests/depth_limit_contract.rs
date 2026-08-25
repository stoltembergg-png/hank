use agent_core::{DepthDecision, DepthLimiter, InvocationGraph, ProjectId};
use agent_protocol::{AgentId, InvocationRequest, InvocationStatus, SessionId};

fn request(project: ProjectId, id: uuid::Uuid, depth: u16) -> InvocationRequest {
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
        depth,
        status: InvocationStatus::Pending,
    }
}

#[test]
// @spec:AC-901
fn root_and_child_at_or_below_max_depth_pass() {
    let project = ProjectId::new();
    let graph = InvocationGraph::default();
    assert_eq!(
        DepthLimiter::check(&graph, None, &request(project, uuid::Uuid::new_v4(), 0), 2),
        DepthDecision::Pass { depth: 0 }
    );
    assert_eq!(
        DepthLimiter::check(&graph, None, &request(project, uuid::Uuid::new_v4(), 2), 2),
        DepthDecision::RejectDepthMismatch
    );
}

#[test]
// @spec:AC-902
fn over_limit_and_missing_ancestry_fail_closed() {
    let project = ProjectId::new();
    let graph = InvocationGraph::default();
    assert_eq!(
        DepthLimiter::check(&graph, None, &request(project, uuid::Uuid::new_v4(), 3), 2),
        DepthDecision::RejectDepthLimit
    );
    assert_eq!(
        DepthLimiter::check(
            &graph,
            Some(uuid::Uuid::new_v4()),
            &request(project, uuid::Uuid::new_v4(), 1),
            2
        ),
        DepthDecision::RejectGraphIncomplete
    );
}

#[test]
// @spec:AC-903
fn repeated_check_does_not_mutate_graph_or_grow_depth() {
    let project = ProjectId::new();
    let graph = InvocationGraph::default();
    let candidate = request(project, uuid::Uuid::new_v4(), 0);
    let first = DepthLimiter::check(&graph, None, &candidate, 2);
    let second = DepthLimiter::check(&graph, None, &candidate, 2);
    assert_eq!(first, second);
    assert!(graph.request(candidate.invocation_id).is_none());
}
