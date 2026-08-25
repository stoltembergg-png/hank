use agent_protocol::{AgentId, ProjectId, WorkflowId};
use agent_runtime::migrations::run_migrations;
use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::workflow_repo::SqliteWorkflowRepository;
use agent_runtime::workflow_scheduler::{WorkflowRunRequest, WorkflowSchedulerError};
use workflow_core::{Workflow, WorkflowGraph, WorkflowNode, WorkflowNodeType};

async fn seeded() -> (SqliteStorage, Workflow, WorkflowGraph) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project = ProjectId::new();
    let owner = AgentId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Workflow Project', 'active', ?, '2026-01-01', '2026-01-01', '{}')").bind(project.to_string()).bind(owner.to_string()).execute(storage.pool()).await.unwrap();
    let mut workflow =
        Workflow::new(project, owner, "scheduled".into(), "policy-default".into()).unwrap();
    workflow.activate().unwrap();
    let mut graph = WorkflowGraph::new(workflow.workflow_id.to_string(), 1).unwrap();
    graph
        .add_node(
            WorkflowNode::new(
                "node-a".into(),
                workflow.workflow_id.to_string(),
                1,
                WorkflowNodeType::Approval,
                serde_json::json!({}),
            )
            .unwrap(),
        )
        .unwrap();
    (storage, workflow, graph)
}

async fn prepare(
    repo: &SqliteWorkflowRepository,
    workflow: &Workflow,
    owner: &AgentId,
    version: u32,
) -> Result<WorkflowRunRequest, WorkflowSchedulerError> {
    WorkflowRunRequest::prepare(
        repo,
        &workflow.project_id,
        owner,
        "job-a",
        "run-a",
        &WorkflowId::from_uuid(workflow.workflow_id),
        version,
    )
    .await
}

// @spec:AC-1251
#[tokio::test]
async fn active_version_resolves_with_stable_idempotency() {
    let (storage, workflow, graph) = seeded().await;
    let repo = SqliteWorkflowRepository::new(storage.pool().clone());
    repo.save_definition(&workflow, &graph, None).await.unwrap();
    let request = prepare(&repo, &workflow, &workflow.owner_id, 1)
        .await
        .unwrap();
    assert_eq!(request.workflow_version, 1);
    assert_eq!(
        request.idempotency_key,
        format!("scheduler:workflow:{}:run-a:1", workflow.project_id)
    );
    let retry = prepare(&repo, &workflow, &workflow.owner_id, 1)
        .await
        .unwrap();
    assert_eq!(request.idempotency_key, retry.idempotency_key);
}

// @spec:AC-1252
#[tokio::test]
async fn archived_missing_and_owner_mismatch_fail_closed() {
    let (storage, mut workflow, graph) = seeded().await;
    let repo = SqliteWorkflowRepository::new(storage.pool().clone());
    repo.save_definition(&workflow, &graph, None).await.unwrap();
    assert!(matches!(
        prepare(&repo, &workflow, &AgentId::new(), 1).await,
        Err(WorkflowSchedulerError::OwnerMismatch)
    ));
    assert!(matches!(
        WorkflowRunRequest::prepare(
            &repo,
            &workflow.project_id,
            &workflow.owner_id,
            "job-a",
            "run-a",
            &WorkflowId::new(),
            1
        )
        .await,
        Err(WorkflowSchedulerError::NotFound)
    ));
    workflow.archive().unwrap();
    workflow.set_version(2).unwrap();
    let mut archived_graph = graph;
    archived_graph.workflow_version = 2;
    for node in archived_graph.nodes.values_mut() {
        node.workflow_version = 2;
    }
    repo.save_definition(&workflow, &archived_graph, Some(1))
        .await
        .unwrap();
    assert!(matches!(
        prepare(&repo, &workflow, &workflow.owner_id, 2).await,
        Err(WorkflowSchedulerError::NotActive)
    ));
}

// @spec:AC-1253
#[tokio::test]
async fn request_contains_no_capability_grant() {
    let (storage, workflow, graph) = seeded().await;
    let repo = SqliteWorkflowRepository::new(storage.pool().clone());
    repo.save_definition(&workflow, &graph, None).await.unwrap();
    let request = prepare(&repo, &workflow, &workflow.owner_id, 1)
        .await
        .unwrap();
    assert_eq!(request.policy_ref, "policy-default");
    assert!(!request.idempotency_key.contains("capability"));
}
