use agent_core::{
    CycleDecision, DepthDecision, InvocationGraph, ParallelBatch, ParallelBatchError,
    ParallelChildOutcome, ProjectId,
};
use agent_protocol::{AgentId, InvocationRequest, InvocationStatus, SessionId};

fn request(project: ProjectId, id: uuid::Uuid) -> InvocationRequest {
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
        depth: 0,
        status: InvocationStatus::Pending,
    }
}

#[test]
// @spec:AC-905
fn valid_independent_children_are_planned_in_input_order_with_hard_limits() {
    let project = ProjectId::new();
    let graph = InvocationGraph::default();
    let children = vec![
        (
            request(project, uuid::Uuid::new_v4()),
            CycleDecision::Pass,
            DepthDecision::Pass { depth: 0 },
        ),
        (
            request(project, uuid::Uuid::new_v4()),
            CycleDecision::Pass,
            DepthDecision::Pass { depth: 0 },
        ),
    ];
    let batch = ParallelBatch::prepare(&graph, None, children, 2, 1).unwrap();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch.concurrency_limit(), 1);
    assert_eq!(batch.ids().len(), 2);
}

#[test]
// @spec:AC-906
fn invalid_gate_duplicate_scope_and_fanout_never_enter_batch() {
    let project = ProjectId::new();
    let graph = InvocationGraph::default();
    let id = uuid::Uuid::new_v4();
    let invalid = vec![(
        request(project, id),
        CycleDecision::RejectSelfLoop,
        DepthDecision::RejectDepthLimit,
    )];
    assert!(matches!(
        ParallelBatch::prepare(&graph, None, invalid, 2, 1),
        Err(ParallelBatchError::PreflightRejected)
    ));
    let duplicate = vec![
        (
            request(project, id),
            CycleDecision::Pass,
            DepthDecision::Pass { depth: 0 },
        ),
        (
            request(project, id),
            CycleDecision::Pass,
            DepthDecision::Pass { depth: 0 },
        ),
    ];
    assert!(matches!(
        ParallelBatch::prepare(&graph, None, duplicate, 2, 1),
        Err(ParallelBatchError::Duplicate)
    ));
    let root = request(project, uuid::Uuid::new_v4());
    let root_id = root.invocation_id;
    let mut scoped_graph = InvocationGraph::default();
    scoped_graph.register(root, None).unwrap();
    let other = vec![(
        request(ProjectId::new(), uuid::Uuid::new_v4()),
        CycleDecision::Pass,
        DepthDecision::Pass { depth: 0 },
    )];
    assert!(matches!(
        ParallelBatch::prepare(&scoped_graph, Some(root_id), other, 2, 1),
        Err(ParallelBatchError::ScopeMismatch)
    ));
}

#[test]
// @spec:AC-907
fn cancellation_and_join_are_deterministic_and_cleanup_once() {
    let project = ProjectId::new();
    let graph = InvocationGraph::default();
    let first = request(project, uuid::Uuid::new_v4());
    let second = request(project, uuid::Uuid::new_v4());
    let mut batch = ParallelBatch::prepare(
        &graph,
        None,
        vec![
            (
                first.clone(),
                CycleDecision::Pass,
                DepthDecision::Pass { depth: 0 },
            ),
            (
                second.clone(),
                CycleDecision::Pass,
                DepthDecision::Pass { depth: 0 },
            ),
        ],
        2,
        2,
    )
    .unwrap();
    assert!(batch.cancel());
    assert!(!batch.cancel());
    let joined = batch
        .join(vec![
            ParallelChildOutcome::Cancelled(first.invocation_id),
            ParallelChildOutcome::Cancelled(second.invocation_id),
        ])
        .unwrap();
    assert_eq!(joined.len(), 2);
}
