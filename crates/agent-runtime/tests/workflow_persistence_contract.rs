use agent_protocol::{AgentId, ProjectId, WorkflowId};
use agent_runtime::migrations::run_migrations;
use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::workflow_repo::{SqliteWorkflowRepository, WorkflowPersistenceError};
use workflow_core::{Workflow, WorkflowEdge, WorkflowGraph, WorkflowNode, WorkflowNodeType};

async fn seed_project(storage: &SqliteStorage, project_id: ProjectId) {
    sqlx::query(
        "INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, ?, 'active', ?, '2026-01-01', '2026-01-01', '{}')",
    )
    .bind(project_id.to_string())
    .bind("workflow-test-project")
    .bind(AgentId::new().to_string())
    .execute(storage.pool())
    .await
    .unwrap();
}

fn definition() -> (Workflow, WorkflowGraph) {
    let project_id = ProjectId::new();
    let workflow = Workflow::new(
        project_id,
        AgentId::new(),
        "persisted-workflow".into(),
        "policy-default".into(),
    )
    .unwrap();
    let mut graph = WorkflowGraph::new(workflow.workflow_id.to_string(), 1).unwrap();
    graph
        .add_node(
            WorkflowNode::new(
                "node-a".into(),
                workflow.workflow_id.to_string(),
                1,
                WorkflowNodeType::Agent,
                serde_json::json!({"prompt": "bounded"}),
            )
            .unwrap(),
        )
        .unwrap();
    graph
        .add_node(
            WorkflowNode::new(
                "node-b".into(),
                workflow.workflow_id.to_string(),
                1,
                WorkflowNodeType::Approval,
                serde_json::json!({}),
            )
            .unwrap(),
        )
        .unwrap();
    let mut edge = WorkflowEdge::new("edge-a-b", "node-a", "node-b");
    edge.workflow_id = workflow.workflow_id.to_string();
    graph.add_edge(edge).unwrap();
    (workflow, graph)
}

#[tokio::test]
// @spec:AC-960
async fn workflow_definition_roundtrips_through_sqlite() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let repo = SqliteWorkflowRepository::new(storage.pool().clone());
    let (workflow, graph) = definition();
    seed_project(&storage, workflow.project_id).await;

    repo.save_definition(&workflow, &graph, None).await.unwrap();
    let loaded = repo
        .load_definition(
            &workflow.project_id,
            &WorkflowId::from_uuid(workflow.workflow_id),
            1,
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(loaded.0.workflow_id, workflow.workflow_id);
    assert_eq!(loaded.0.project_id, workflow.project_id);
    assert_eq!(loaded.1.nodes.len(), 2);
    assert_eq!(loaded.1.edges.len(), 1);
}

#[tokio::test]
// @spec:AC-961
async fn workflow_read_isolated_by_project_and_invalid_graph_is_atomic() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let repo = SqliteWorkflowRepository::new(storage.pool().clone());
    let (workflow, mut graph) = definition();
    seed_project(&storage, workflow.project_id).await;

    let foreign_project = ProjectId::new();
    assert!(repo
        .load_definition(
            &foreign_project,
            &WorkflowId::from_uuid(workflow.workflow_id),
            1
        )
        .await
        .unwrap()
        .is_none());

    let mut reverse_edge = WorkflowEdge::new("edge-b-a", "node-b", "node-a");
    reverse_edge.workflow_id = workflow.workflow_id.to_string();
    graph.add_edge(reverse_edge).unwrap();
    assert!(matches!(
        repo.save_definition(&workflow, &graph, None).await,
        Err(WorkflowPersistenceError::InvalidGraph(_))
    ));
    assert!(repo
        .load_definition(
            &workflow.project_id,
            &WorkflowId::from_uuid(workflow.workflow_id),
            1
        )
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
// @spec:AC-962
async fn workflow_update_requires_expected_version() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let repo = SqliteWorkflowRepository::new(storage.pool().clone());
    let (mut workflow, mut graph) = definition();
    seed_project(&storage, workflow.project_id).await;
    repo.save_definition(&workflow, &graph, None).await.unwrap();

    workflow.set_version(2).unwrap();
    graph.workflow_version = 2;
    for node in graph.nodes.values_mut() {
        node.workflow_version = 2;
    }
    assert!(repo
        .save_definition(&workflow, &graph, Some(1))
        .await
        .is_ok());
    assert_eq!(
        repo.save_definition(&workflow, &graph, Some(1)).await,
        Err(WorkflowPersistenceError::ConcurrencyConflict)
    );
}
