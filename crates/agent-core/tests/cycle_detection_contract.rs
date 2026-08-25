use agent_core::{CycleDecision, CycleDetector, InvocationGraph, ProjectId};
use agent_protocol::{AgentId, InvocationRequest, InvocationStatus, SessionId};

fn request(
    project: ProjectId,
    id: uuid::Uuid,
    caller: AgentId,
    callee: AgentId,
) -> InvocationRequest {
    InvocationRequest {
        schema_version: agent_protocol::INVOCATION_SCHEMA_VERSION,
        invocation_id: id,
        project_id: project,
        group_id: uuid::Uuid::new_v4(),
        session_id: SessionId::new(),
        caller_id: caller,
        callee_id: callee,
        trace_id: agent_protocol::TraceId::new(),
        task: "safe".into(),
        context_refs: vec![],
        max_tokens: 100,
        depth: 0,
        status: InvocationStatus::Pending,
    }
}

#[test]
// @spec:AC-897
fn self_loop_is_rejected_before_graph_mutation() {
    let project = ProjectId::new();
    let actor = AgentId::new();
    let graph = InvocationGraph::default();
    let result = CycleDetector::check(
        &graph,
        None,
        &request(project, uuid::Uuid::new_v4(), actor, actor),
    );
    assert_eq!(result, CycleDecision::RejectSelfLoop);
}

#[test]
// @spec:AC-898
fn indirect_cycle_is_rejected_and_acyclic_path_passes() {
    let project = ProjectId::new();
    let a = AgentId::new();
    let b = AgentId::new();
    let c = AgentId::new();
    let mut graph = InvocationGraph::default();
    let ab = uuid::Uuid::new_v4();
    graph.register(request(project, ab, a, b), None).unwrap();
    let bc = uuid::Uuid::new_v4();
    graph
        .register(request(project, bc, b, c), Some(ab))
        .unwrap();
    assert_eq!(
        CycleDetector::check(
            &graph,
            Some(bc),
            &request(project, uuid::Uuid::new_v4(), c, a)
        ),
        CycleDecision::RejectAncestorCycle { path_len: 2 }
    );
    assert_eq!(
        CycleDetector::check(
            &graph,
            Some(bc),
            &request(project, uuid::Uuid::new_v4(), c, AgentId::new())
        ),
        CycleDecision::Pass
    );
}

#[test]
// @spec:AC-899
fn incomplete_or_wrong_scope_graph_fails_closed_and_checks_are_idempotent() {
    let project = ProjectId::new();
    let actor = AgentId::new();
    let graph = InvocationGraph::default();
    let candidate = request(project, uuid::Uuid::new_v4(), actor, AgentId::new());
    assert_eq!(
        CycleDetector::check(&graph, Some(uuid::Uuid::new_v4()), &candidate),
        CycleDecision::RejectGraphIncomplete
    );
    assert_eq!(
        CycleDetector::check(
            &graph,
            None,
            &request(
                ProjectId::new(),
                uuid::Uuid::new_v4(),
                actor,
                AgentId::new()
            )
        ),
        CycleDecision::Pass
    );
    assert_eq!(
        CycleDetector::check(&graph, None, &candidate),
        CycleDetector::check(&graph, None, &candidate)
    );
}
