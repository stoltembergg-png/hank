use workflow_core::{
    ExecutionError, NodeRunState, RunState, WorkflowGraph, WorkflowNode, WorkflowNodeType,
    WorkflowRun,
};

fn graph() -> WorkflowGraph {
    let workflow_id = "workflow-execution".to_string();
    let mut graph = WorkflowGraph::new(workflow_id.clone(), 1).unwrap();
    for node_id in ["a", "b", "c"] {
        let mut node = WorkflowNode::new(
            node_id.to_string(),
            workflow_id.clone(),
            1,
            WorkflowNodeType::Condition,
            serde_json::json!({}),
        )
        .unwrap();
        if node_id == "a" {
            node.retry.max_attempts = 2;
        }
        graph.add_node(node).unwrap();
    }
    let mut edge = workflow_core::WorkflowEdge::new("a-b", "a", "b");
    edge.workflow_id = workflow_id.clone();
    graph.add_edge(edge).unwrap();
    let mut edge = workflow_core::WorkflowEdge::new("a-c", "a", "c");
    edge.workflow_id = workflow_id;
    graph.add_edge(edge).unwrap();
    graph
}

// @spec:AC-964
// @spec:AC-965
#[test]
fn run_accepts_valid_graph_and_releases_nodes_deterministically() {
    let mut run = WorkflowRun::start("run-1", &graph(), 1).unwrap();
    assert_eq!(run.state(), RunState::Running);
    assert_eq!(run.ready_nodes(), vec!["a"]);
    run.dispatch("a").unwrap();
    assert_eq!(run.node_state("a"), Some(NodeRunState::InFlight));
    assert_eq!(run.ready_nodes(), Vec::<String>::new());
    assert_eq!(run.dispatch("b"), Err(ExecutionError::Backpressure));
    run.complete("a").unwrap();
    assert_eq!(run.ready_nodes(), vec!["b", "c"]);
    run.dispatch("b").unwrap();
    run.complete("b").unwrap();
    run.dispatch("c").unwrap();
    run.complete("c").unwrap();
    assert_eq!(run.state(), RunState::Completed);
}

// @spec:AC-966
#[test]
fn failure_retry_and_cancel_are_terminal_and_fail_closed() {
    let mut run = WorkflowRun::start("run-2", &graph(), 2).unwrap();
    run.dispatch("a").unwrap();
    assert_eq!(run.retry("a", "timeout").unwrap().attempt, 2);
    assert_eq!(run.node_state("a"), Some(NodeRunState::Ready));
    run.dispatch("a").unwrap();
    run.fail("a", "provider_error").unwrap();
    assert_eq!(run.node_failure_code("a"), Some("provider_error"));
    assert_eq!(run.state(), RunState::Failed);
    assert_eq!(run.complete("a"), Err(ExecutionError::Terminal));

    let mut cancelled = WorkflowRun::start("run-3", &graph(), 2).unwrap();
    cancelled.dispatch("a").unwrap();
    cancelled.cancel().unwrap();
    assert_eq!(cancelled.state(), RunState::Cancelled);
    assert_eq!(cancelled.dispatch("a"), Err(ExecutionError::Terminal));
    assert_eq!(cancelled.cancel(), Err(ExecutionError::Terminal));
}

#[test]
fn invalid_identity_and_graph_fail_before_run_mutation() {
    assert!(matches!(
        WorkflowRun::start("", &graph(), 1),
        Err(ExecutionError::InvalidIdentity)
    ));
    assert!(matches!(
        WorkflowRun::start("run-4", &graph(), 0),
        Err(ExecutionError::Backpressure)
    ));
}
