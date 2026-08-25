use serde_json::json;
use workflow_core::{
    WorkflowEdge, WorkflowEdgeError, WorkflowGraph, WorkflowNode, WorkflowNodeType,
    WORKFLOW_GRAPH_MAX_EDGES,
};

fn node(id: &str) -> WorkflowNode {
    WorkflowNode::new(
        id.into(),
        "workflow-1".into(),
        1,
        WorkflowNodeType::Agent,
        json!({}),
    )
    .unwrap()
}

fn graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("workflow-1".into(), 1).unwrap();
    graph.add_node(node("a")).unwrap();
    graph.add_node(node("b")).unwrap();
    graph.add_node(node("c")).unwrap();
    graph
}

#[test]
// @spec:AC-954 @spec:AC-955
fn valid_dag_is_deterministic_and_bounded() {
    let mut graph = graph();
    graph.add_edge(WorkflowEdge::new("e-2", "b", "c")).unwrap();
    graph.add_edge(WorkflowEdge::new("e-1", "a", "b")).unwrap();
    graph.validate().unwrap();
    assert_eq!(graph.edges[0].edge_id, "e-2");
    assert_eq!(graph.edges[1].edge_id, "e-1");
}

#[test]
// @spec:AC-955
fn cycles_self_edges_orphans_and_duplicates_fail_closed() {
    let mut self_edge = graph();
    assert_eq!(
        self_edge.add_edge(WorkflowEdge::new("self", "a", "a")),
        Err(WorkflowEdgeError::SelfEdge)
    );

    let mut unknown = graph();
    assert_eq!(
        unknown.add_edge(WorkflowEdge::new("missing", "a", "missing")),
        Err(WorkflowEdgeError::UnknownNode)
    );

    let mut duplicate = graph();
    duplicate
        .add_edge(WorkflowEdge::new("e-1", "a", "b"))
        .unwrap();
    assert_eq!(
        duplicate.add_edge(WorkflowEdge::new("e-1", "a", "c")),
        Err(WorkflowEdgeError::DuplicateEdge)
    );

    let mut cycle = graph();
    cycle.add_edge(WorkflowEdge::new("a-b", "a", "b")).unwrap();
    cycle.add_edge(WorkflowEdge::new("b-c", "b", "c")).unwrap();
    cycle.add_edge(WorkflowEdge::new("c-a", "c", "a")).unwrap();
    assert_eq!(cycle.validate(), Err(WorkflowEdgeError::Cycle));
}

#[test]
// @spec:AC-956
fn cross_workflow_edges_are_rejected_without_expression_execution() {
    let mut cross_workflow = graph();
    let mut edge = WorkflowEdge::new("cross", "a", "b");
    edge.workflow_id = "workflow-foreign".into();
    assert_eq!(
        cross_workflow.add_edge(edge),
        Err(WorkflowEdgeError::CrossWorkflow)
    );

    let mut oversized = graph();
    oversized.edges = (0..=WORKFLOW_GRAPH_MAX_EDGES)
        .map(|index| WorkflowEdge::new(format!("e-{index}"), "a", "b"))
        .collect();
    assert_eq!(oversized.validate(), Err(WorkflowEdgeError::TooManyEdges));
}
