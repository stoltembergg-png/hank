//! Typed workflow editor bridge for the desktop shell.
//!
//! The bridge accepts only the bounded editor DTO.  Project ownership,
//! graph validation, version checks and SQLite persistence remain in the
//! runtime repository and workflow core; no raw SQL or arbitrary payloads
//! cross the Tauri boundary.

use agent_core::project::{ProjectRepository, ProjectStatus};
use agent_protocol::ids::{AgentId, ProjectId, WorkflowId};
use agent_runtime::workflow_repo::{SqliteWorkflowRepository, WorkflowPersistenceError};
use agent_runtime::{project_repo::SqliteProjectRepository, SqliteStorage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use workflow_core::{
    CancelPolicy, RetryPolicy, Workflow, WorkflowEdge, WorkflowGraph, WorkflowNode,
    WorkflowNodeType,
};

const MAX_EDITOR_NODES: usize = 12;
const MAX_EDITOR_EDGES: usize = 24;
const MAX_EDITOR_LABEL_BYTES: usize = 256;
const MAX_EDITOR_ID_BYTES: usize = 128;
const WORKFLOW_POLICY_REF: &str = "workflow.default";
const WORKFLOW_NAME: &str = "Desktop workflow";
const LOCAL_OWNER_UUID: &str = "00000000-0000-4000-8000-000000000001";

#[derive(Clone)]
pub struct WorkflowBridgeState {
    projects: Arc<SqliteProjectRepository>,
    workflows: Arc<SqliteWorkflowRepository>,
}

impl WorkflowBridgeState {
    pub fn new(
        projects: Arc<SqliteProjectRepository>,
        workflows: Arc<SqliteWorkflowRepository>,
    ) -> Self {
        Self { projects, workflows }
    }
}

pub fn bridge_state(storage: &SqliteStorage) -> WorkflowBridgeState {
    WorkflowBridgeState::new(
        Arc::new(SqliteProjectRepository::new(storage.pool().clone())),
        Arc::new(SqliteWorkflowRepository::new(storage.pool().clone())),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBridgeErrorCode {
    InvalidInput,
    Unauthorized,
    NotFound,
    Conflict,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowBridgeError {
    pub code: WorkflowBridgeErrorCode,
    pub message: String,
}

impl WorkflowBridgeError {
    fn new(code: WorkflowBridgeErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }

    fn reason(&self) -> &'static str {
        match self.code {
            WorkflowBridgeErrorCode::InvalidInput => "invalid_workflow",
            WorkflowBridgeErrorCode::Unauthorized => "unauthorized_project",
            WorkflowBridgeErrorCode::NotFound | WorkflowBridgeErrorCode::Conflict => {
                "stale_version"
            }
            WorkflowBridgeErrorCode::Internal => "workflow_unavailable",
        }
    }
}

impl std::fmt::Display for WorkflowBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WorkflowBridgeError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowNodeInput {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowEdgeInput {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowDraftInput {
    pub project_id: String,
    pub workflow_id: String,
    pub nodes: Vec<WorkflowNodeInput>,
    pub edges: Vec<WorkflowEdgeInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowCommandInput {
    pub project_id: String,
    pub workflow_id: String,
    pub expected_version: u32,
    pub draft: WorkflowDraftInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GetWorkflowInput {
    pub project_id: String,
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowValidationOutput {
    pub valid: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSaveOutput {
    pub version: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowNodeOutput {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowEdgeOutput {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSnapshotOutput {
    pub project_id: String,
    pub workflow_id: String,
    pub version: u32,
    pub nodes: Vec<WorkflowNodeOutput>,
    pub edges: Vec<WorkflowEdgeOutput>,
}

fn parse_project_id(value: &str) -> Result<ProjectId, WorkflowBridgeError> {
    value.parse::<ProjectId>().map_err(|_| {
        WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid project id",
        )
    })
}

fn parse_workflow_id(value: &str) -> Result<WorkflowId, WorkflowBridgeError> {
    value.parse::<WorkflowId>().map_err(|_| {
        WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid workflow id",
        )
    })
}

fn node_type(kind: &str) -> Result<WorkflowNodeType, WorkflowBridgeError> {
    match kind {
        "agent" => Ok(WorkflowNodeType::Agent),
        "condition" => Ok(WorkflowNodeType::Condition),
        "approval" => Ok(WorkflowNodeType::Approval),
        "tool" => Ok(WorkflowNodeType::Tool),
        _ => Err(WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "unsupported workflow node type",
        )),
    }
}

fn validate_identifier(value: &str) -> Result<(), WorkflowBridgeError> {
    if value.trim().is_empty()
        || value.len() > MAX_EDITOR_ID_BYTES
        || value.chars().any(char::is_control)
        || value.contains('/')
        || value.contains('\\')
        || value.contains("..")
    {
        return Err(WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid workflow identity",
        ));
    }
    Ok(())
}

fn validate_label(value: &str) -> Result<(), WorkflowBridgeError> {
    if value.trim().is_empty()
        || value.len() > MAX_EDITOR_LABEL_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid workflow label",
        ));
    }
    Ok(())
}

fn local_owner() -> AgentId {
    AgentId::parse(&format!("agent-{LOCAL_OWNER_UUID}")).expect("static desktop owner id")
}

async fn project_scope(
    state: &WorkflowBridgeState,
    value: &str,
) -> Result<ProjectId, WorkflowBridgeError> {
    let project_id = parse_project_id(value)?;
    let project = state
        .projects
        .get_by_id(&project_id)
        .await
        .map_err(|_| {
            WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::Internal,
                "could not load project",
            )
        })?
        .ok_or_else(|| {
            WorkflowBridgeError::new(WorkflowBridgeErrorCode::NotFound, "project not found")
        })?;
    if project.status != ProjectStatus::Active {
        return Err(WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::Unauthorized,
            "project is not active",
        ));
    }
    Ok(project_id)
}

fn graph_from_input(
    workflow_id: &WorkflowId,
    version: u32,
    draft: &WorkflowDraftInput,
) -> Result<WorkflowGraph, WorkflowBridgeError> {
    if draft.nodes.len() > MAX_EDITOR_NODES || draft.edges.len() > MAX_EDITOR_EDGES {
        return Err(WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "workflow graph exceeds editor bounds",
        ));
    }
    // workflow-core stores the graph identity as the raw UUID while the
    // transport contract exposes the typed `wf-<uuid>` representation.
    let workflow_key = workflow_id.as_uuid().to_string();
    let mut graph = WorkflowGraph::new(workflow_key.clone(), version).map_err(|_| {
        WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid workflow graph",
        )
    })?;
    for input in &draft.nodes {
        validate_identifier(&input.id)?;
        validate_label(&input.label)?;
        let node = WorkflowNode {
            schema_version: workflow_core::WORKFLOW_NODE_SCHEMA_VERSION,
            node_id: input.id.clone(),
            workflow_id: workflow_key.clone(),
            workflow_version: version,
            node_type: node_type(&input.kind)?,
            input_schema: serde_json::json!({ "label": input.label }),
            output_schema: serde_json::json!({}),
            timeout_ms: 30_000,
            retry: RetryPolicy { max_attempts: 1 },
            cancel: CancelPolicy::Cooperative,
            required_capabilities: Vec::new(),
        };
        graph.add_node(node).map_err(|_| {
            WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::InvalidInput,
                "invalid workflow node",
            )
        })?;
    }
    for (index, input) in draft.edges.iter().enumerate() {
        validate_identifier(&input.source)?;
        validate_identifier(&input.target)?;
        let mut edge = WorkflowEdge::new(format!("edge-{index}"), &input.source, &input.target);
        edge.workflow_id = workflow_key.clone();
        edge.ordering = u32::try_from(index).map_err(|_| {
            WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::InvalidInput,
                "invalid workflow edge",
            )
        })?;
        graph.add_edge(edge).map_err(|_| {
            WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::InvalidInput,
                "invalid workflow edge",
            )
        })?;
    }
    graph.validate().map_err(|_| {
        WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid workflow graph",
        )
    })?;
    Ok(graph)
}

async fn prepare(
    state: &WorkflowBridgeState,
    input: &WorkflowCommandInput,
) -> Result<(Workflow, WorkflowGraph, Option<u32>), WorkflowBridgeError> {
    let project_id = project_scope(state, &input.project_id).await?;
    if input.draft.project_id != input.project_id || input.draft.workflow_id != input.workflow_id {
        return Err(WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::Unauthorized,
            "workflow scope does not match project",
        ));
    }
    let workflow_id = parse_workflow_id(&input.workflow_id)?;
    let (mut workflow, expected) = if input.expected_version == 0 {
        if state
            .workflows
            .load_latest_definition(&project_id, &workflow_id)
            .await
            .map_err(map_persistence_error)?
            .is_some()
        {
            return Err(WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::Conflict,
                "workflow version is stale",
            ));
        }
        let mut workflow = Workflow::new(
            project_id,
            local_owner(),
            WORKFLOW_NAME.to_string(),
            WORKFLOW_POLICY_REF.to_string(),
        )
        .map_err(|_| {
            WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::Internal,
                "could not create workflow",
            )
        })?;
        workflow.workflow_id = workflow_id.as_uuid();
        (workflow, None)
    } else {
        let (workflow, _) = state
            .workflows
            .load_definition(&project_id, &workflow_id, input.expected_version)
            .await
            .map_err(map_persistence_error)?
            .ok_or_else(|| {
                WorkflowBridgeError::new(
                    WorkflowBridgeErrorCode::Conflict,
                    "workflow version is stale",
                )
            })?;
        (workflow, Some(input.expected_version))
    };
    let next_version = input.expected_version.saturating_add(1);
    workflow.set_version(next_version).map_err(|_| {
        WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::InvalidInput,
            "invalid workflow version",
        )
    })?;
    let graph = graph_from_input(&workflow_id, next_version, &input.draft)?;
    Ok((workflow, graph, expected))
}

fn map_persistence_error(error: WorkflowPersistenceError) -> WorkflowBridgeError {
    match error {
        WorkflowPersistenceError::InvalidGraph(_) | WorkflowPersistenceError::Serialization(_) => {
            WorkflowBridgeError::new(
                WorkflowBridgeErrorCode::InvalidInput,
                "invalid workflow definition",
            )
        }
        WorkflowPersistenceError::ConcurrencyConflict => WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::Conflict,
            "workflow version is stale",
        ),
        WorkflowPersistenceError::NotFound => {
            WorkflowBridgeError::new(WorkflowBridgeErrorCode::NotFound, "workflow not found")
        }
        WorkflowPersistenceError::Query(_) => WorkflowBridgeError::new(
            WorkflowBridgeErrorCode::Internal,
            "workflow persistence is unavailable",
        ),
    }
}

#[tauri::command]
pub async fn validate_workflow(
    state: State<'_, WorkflowBridgeState>,
    input: WorkflowCommandInput,
) -> Result<WorkflowValidationOutput, WorkflowBridgeError> {
    match prepare(&state, &input).await {
        Ok(_) => Ok(WorkflowValidationOutput {
            valid: true,
            reason: None,
        }),
        Err(error)
            if matches!(
                error.code,
                WorkflowBridgeErrorCode::InvalidInput
                    | WorkflowBridgeErrorCode::Unauthorized
                    | WorkflowBridgeErrorCode::Conflict
                    | WorkflowBridgeErrorCode::NotFound
            ) => Ok(WorkflowValidationOutput {
                valid: false,
                reason: Some(error.reason().to_string()),
            }),
        Err(error) => Err(error),
    }
}

#[tauri::command]
pub async fn save_workflow(
    state: State<'_, WorkflowBridgeState>,
    input: WorkflowCommandInput,
) -> Result<WorkflowSaveOutput, WorkflowBridgeError> {
    let (workflow, graph, expected) = prepare(&state, &input).await?;
    let version = workflow.version;
    state
        .workflows
        .save_definition(&workflow, &graph, expected)
        .await
        .map_err(map_persistence_error)?;
    Ok(WorkflowSaveOutput { version })
}

fn node_kind(node: WorkflowNodeType) -> &'static str {
    match node {
        WorkflowNodeType::Agent => "agent",
        WorkflowNodeType::Condition => "condition",
        WorkflowNodeType::Approval => "approval",
        WorkflowNodeType::Tool => "tool",
        WorkflowNodeType::Python => "python",
        WorkflowNodeType::Parallel => "parallel",
        WorkflowNodeType::Delay => "delay",
        WorkflowNodeType::SubWorkflow => "sub_workflow",
    }
}

fn snapshot(
    project_id: &ProjectId,
    workflow: &Workflow,
    graph: &WorkflowGraph,
) -> WorkflowSnapshotOutput {
    let nodes = graph
        .nodes
        .values()
        .map(|node| WorkflowNodeOutput {
            id: node.node_id.clone(),
            kind: node_kind(node.node_type).to_string(),
            label: node
                .input_schema
                .get("label")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&node.node_id)
                .to_string(),
        })
        .collect();
    let edges = graph
        .edges
        .iter()
        .map(|edge| WorkflowEdgeOutput {
            source: edge.source_node.clone(),
            target: edge.target_node.clone(),
        })
        .collect();
    WorkflowSnapshotOutput {
        project_id: project_id.to_string(),
        workflow_id: WorkflowId::from_uuid(workflow.workflow_id).to_string(),
        version: workflow.version,
        nodes,
        edges,
    }
}

#[tauri::command]
pub async fn get_workflow(
    state: State<'_, WorkflowBridgeState>,
    input: GetWorkflowInput,
) -> Result<Option<WorkflowSnapshotOutput>, WorkflowBridgeError> {
    let project_id = project_scope(&state, &input.project_id).await?;
    let workflow_id = parse_workflow_id(&input.workflow_id)?;
    state
        .workflows
        .load_latest_definition(&project_id, &workflow_id)
        .await
        .map_err(map_persistence_error)
        .map(|definition| definition.map(|(workflow, graph)| snapshot(&project_id, &workflow, &graph)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::project::{Project, ProjectRepository};
    use agent_runtime::migrations::run_migrations;

    fn command(project_id: &str, expected_version: u32) -> WorkflowCommandInput {
        WorkflowCommandInput {
            project_id: project_id.to_string(),
            workflow_id: "wf-00000000-0000-4000-8000-000000000001".to_string(),
            expected_version,
            draft: WorkflowDraftInput {
                project_id: project_id.to_string(),
                workflow_id: "wf-00000000-0000-4000-8000-000000000001".to_string(),
                nodes: vec![WorkflowNodeInput {
                    id: "agent-1".to_string(),
                    kind: "agent".to_string(),
                    label: "Release agent".to_string(),
                }],
                edges: Vec::new(),
            },
        }
    }

    #[test]
    fn graph_mapping_rejects_unknown_kinds_and_preserves_bounds() {
        let workflow_id = "wf-00000000-0000-4000-8000-000000000001"
            .parse::<WorkflowId>()
            .unwrap();
        let mut invalid = command("proj-00000000-0000-4000-8000-000000000001", 0);
        invalid.draft.nodes[0].kind = "shell".to_string();
        assert_eq!(
            graph_from_input(&workflow_id, 1, &invalid.draft)
                .unwrap_err()
                .message,
            "unsupported workflow node type"
        );
    }

    #[tokio::test]
    async fn save_and_load_are_project_scoped_and_versioned() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let projects = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let workflows = Arc::new(SqliteWorkflowRepository::new(storage.pool().clone()));
        let project = Project::create("workflow-project", "owner", None).unwrap();
        projects.save(&project).await.unwrap();
        let state = WorkflowBridgeState::new(projects, workflows);

        let input = command(&project.id.to_string(), 0);
        let (workflow, graph, expected) = prepare(&state, &input).await.unwrap();
        state
            .workflows
            .save_definition(&workflow, &graph, expected)
            .await
            .unwrap();
        let workflow_id = input.workflow_id.parse::<WorkflowId>().unwrap();
        let loaded = state
            .workflows
            .load_latest_definition(&project.id, &workflow_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.0.version, 1);
        assert_eq!(loaded.1.nodes.len(), 1);
        assert_eq!(loaded.0.project_id, project.id);

        let stale = prepare(&state, &input).await.unwrap_err();
        assert_eq!(stale.code, WorkflowBridgeErrorCode::Conflict);
    }
}
