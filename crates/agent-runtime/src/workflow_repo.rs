use agent_protocol::ids::{AgentId, ProjectId, WorkflowId};
use sqlx::{Pool, Row, Sqlite};
use thiserror::Error;
use workflow_core::{
    CancelPolicy, RetryPolicy, Workflow, WorkflowEdge, WorkflowGraph, WorkflowNode,
    WorkflowNodeType, WorkflowStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowPersistenceError {
    #[error("workflow definition is invalid: {0}")]
    InvalidGraph(String),
    #[error("workflow persistence serialization failed: {0}")]
    Serialization(String),
    #[error("workflow persistence query failed: {0}")]
    Query(String),
    #[error("workflow version concurrency conflict")]
    ConcurrencyConflict,
    #[error("workflow definition not found")]
    NotFound,
}

#[derive(Clone)]
pub struct SqliteWorkflowRepository {
    pool: Pool<Sqlite>,
}

impl SqliteWorkflowRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn save_definition(
        &self,
        workflow: &Workflow,
        graph: &WorkflowGraph,
        expected_version: Option<u32>,
    ) -> Result<(), WorkflowPersistenceError> {
        graph
            .validate()
            .map_err(|error| WorkflowPersistenceError::InvalidGraph(error.to_string()))?;
        if graph.workflow_id != workflow.workflow_id.to_string()
            || graph.workflow_version != workflow.version
        {
            return Err(WorkflowPersistenceError::InvalidGraph(
                "workflow and graph identity/version differ".into(),
            ));
        }
        if let Some(expected) = expected_version {
            if workflow.version != expected.saturating_add(1) {
                return Err(WorkflowPersistenceError::ConcurrencyConflict);
            }
        }

        let metadata = serde_json::to_string(&workflow.metadata)
            .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;

        if let Some(expected) = expected_version {
            let row = sqlx::query(
                "SELECT COUNT(*) AS count FROM workflow_definitions WHERE workflow_id = ? AND project_id = ? AND version = ?",
            )
            .bind(workflow_key(workflow.workflow_id))
            .bind(workflow.project_id.to_string())
            .bind(i64::from(expected))
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;
            let count: i64 = row.get("count");
            if count != 1 {
                return Err(WorkflowPersistenceError::ConcurrencyConflict);
            }
        }

        sqlx::query(
            "INSERT INTO workflow_definitions (workflow_id, project_id, owner_id, version, schema_version, name, status, policy_ref, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(workflow_key(workflow.workflow_id))
        .bind(workflow.project_id.to_string())
        .bind(workflow.owner_id.to_string())
        .bind(i64::from(workflow.version))
        .bind(i64::from(workflow.schema_version))
        .bind(&workflow.name)
        .bind(status_name(workflow.status))
        .bind(&workflow.policy_ref)
        .bind(metadata)
        .execute(&mut *transaction)
        .await
        .map_err(|error| {
            if error.as_database_error().is_some_and(|db| db.is_unique_violation()) {
                WorkflowPersistenceError::ConcurrencyConflict
            } else {
                WorkflowPersistenceError::Query(error.to_string())
            }
        })?;

        for node in graph.nodes.values() {
            sqlx::query(
                "INSERT INTO workflow_nodes (workflow_id, workflow_version, node_id, schema_version, node_type, input_schema, output_schema, timeout_ms, retry_max_attempts, cancel_policy, required_capabilities) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(workflow_key(workflow.workflow_id))
            .bind(i64::from(workflow.version))
            .bind(&node.node_id)
            .bind(i64::from(node.schema_version))
            .bind(node_type_name(node.node_type))
            .bind(json_text(&node.input_schema)?)
            .bind(json_text(&node.output_schema)?)
            .bind(i64::try_from(node.timeout_ms).map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?)
            .bind(i64::from(node.retry.max_attempts))
            .bind(cancel_policy_name(node.cancel))
            .bind(json_text(&node.required_capabilities)?)
            .execute(&mut *transaction)
            .await
            .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;
        }

        for edge in &graph.edges {
            sqlx::query(
                "INSERT INTO workflow_edges (workflow_id, workflow_version, edge_id, source_node, source_port, target_node, target_port, condition, ordering) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(workflow_key(workflow.workflow_id))
            .bind(i64::from(workflow.version))
            .bind(&edge.edge_id)
            .bind(&edge.source_node)
            .bind(&edge.source_port)
            .bind(&edge.target_node)
            .bind(&edge.target_port)
            .bind(&edge.condition)
            .bind(i64::from(edge.ordering))
            .execute(&mut *transaction)
            .await
            .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))
    }

    pub async fn load_definition(
        &self,
        project_id: &ProjectId,
        workflow_id: &WorkflowId,
        version: u32,
    ) -> Result<Option<(Workflow, WorkflowGraph)>, WorkflowPersistenceError> {
        let row = sqlx::query(
            "SELECT workflow_id, project_id, owner_id, version, schema_version, name, status, policy_ref, metadata FROM workflow_definitions WHERE workflow_id = ? AND project_id = ? AND version = ?",
        )
        .bind(workflow_id.to_string())
        .bind(project_id.to_string())
        .bind(i64::from(version))
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;
        let Some(row) = row else {
            return Ok(None);
        };

        let stored_workflow_id: WorkflowId = parse_id(row.get::<String, _>("workflow_id"))?;
        let stored_project_id: ProjectId = parse_id(row.get::<String, _>("project_id"))?;
        let owner_id: AgentId = parse_id(row.get::<String, _>("owner_id"))?;
        let stored_version = u32::try_from(row.get::<i64, _>("version"))
            .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?;
        let schema_version = u32::try_from(row.get::<i64, _>("schema_version"))
            .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?;
        let status = parse_status(row.get::<String, _>("status"))?;
        let metadata = serde_json::from_str(row.get::<String, _>("metadata").as_str())
            .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?;
        let workflow = Workflow {
            schema_version,
            workflow_id: stored_workflow_id.as_uuid(),
            project_id: stored_project_id,
            owner_id,
            name: row.get("name"),
            version: stored_version,
            status,
            policy_ref: row.get("policy_ref"),
            metadata,
        };
        let mut graph = WorkflowGraph::new(workflow.workflow_id.to_string(), workflow.version)
            .map_err(|error| WorkflowPersistenceError::InvalidGraph(error.to_string()))?;

        let node_rows = sqlx::query(
            "SELECT node_id, schema_version, node_type, input_schema, output_schema, timeout_ms, retry_max_attempts, cancel_policy, required_capabilities FROM workflow_nodes WHERE workflow_id = ? AND workflow_version = ? ORDER BY node_id",
        )
        .bind(workflow_key(workflow.workflow_id))
        .bind(i64::from(version))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;
        for row in node_rows {
            let node = WorkflowNode {
                schema_version: u32::try_from(row.get::<i64, _>("schema_version"))
                    .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?,
                node_id: row.get("node_id"),
                workflow_id: workflow.workflow_id.to_string(),
                workflow_version: version,
                node_type: parse_node_type(row.get("node_type"))?,
                input_schema: parse_json(row.get("input_schema"))?,
                output_schema: parse_json(row.get("output_schema"))?,
                timeout_ms: u64::try_from(row.get::<i64, _>("timeout_ms"))
                    .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?,
                retry: RetryPolicy {
                    max_attempts: u8::try_from(row.get::<i64, _>("retry_max_attempts")).map_err(
                        |error| WorkflowPersistenceError::Serialization(error.to_string()),
                    )?,
                },
                cancel: parse_cancel_policy(row.get("cancel_policy"))?,
                required_capabilities: serde_json::from_str(
                    row.get::<String, _>("required_capabilities").as_str(),
                )
                .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?,
            };
            graph
                .add_node(node)
                .map_err(|error| WorkflowPersistenceError::InvalidGraph(error.to_string()))?;
        }

        let edge_rows = sqlx::query(
            "SELECT edge_id, source_node, source_port, target_node, target_port, condition, ordering FROM workflow_edges WHERE workflow_id = ? AND workflow_version = ? ORDER BY ordering, edge_id",
        )
        .bind(workflow_key(workflow.workflow_id))
        .bind(i64::from(version))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| WorkflowPersistenceError::Query(error.to_string()))?;
        for row in edge_rows {
            let edge = WorkflowEdge {
                edge_id: row.get("edge_id"),
                workflow_id: workflow.workflow_id.to_string(),
                source_node: row.get("source_node"),
                source_port: row.get("source_port"),
                target_node: row.get("target_node"),
                target_port: row.get("target_port"),
                condition: row.get("condition"),
                ordering: u32::try_from(row.get::<i64, _>("ordering"))
                    .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))?,
            };
            graph
                .add_edge(edge)
                .map_err(|error| WorkflowPersistenceError::InvalidGraph(error.to_string()))?;
        }
        graph
            .validate()
            .map_err(|error| WorkflowPersistenceError::InvalidGraph(error.to_string()))?;
        Ok(Some((workflow, graph)))
    }
}

fn workflow_key(workflow_id: agent_protocol::Uuid) -> String {
    WorkflowId::from_uuid(workflow_id).to_string()
}

fn json_text<T: serde::Serialize>(value: &T) -> Result<String, WorkflowPersistenceError> {
    serde_json::to_string(value)
        .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))
}

fn parse_json(text: String) -> Result<serde_json::Value, WorkflowPersistenceError> {
    serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))
}

fn parse_id<T: std::str::FromStr<Err = agent_protocol::ids::IdParseError>>(
    text: String,
) -> Result<T, WorkflowPersistenceError> {
    T::from_str(&text).map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))
}

fn status_name(status: WorkflowStatus) -> &'static str {
    match status {
        WorkflowStatus::Draft => "draft",
        WorkflowStatus::Active => "active",
        WorkflowStatus::Paused => "paused",
        WorkflowStatus::Archived => "archived",
        WorkflowStatus::Blocked => "blocked",
    }
}
fn parse_status(text: String) -> Result<WorkflowStatus, WorkflowPersistenceError> {
    match text.as_str() {
        "draft" => Ok(WorkflowStatus::Draft),
        "active" => Ok(WorkflowStatus::Active),
        "paused" => Ok(WorkflowStatus::Paused),
        "archived" => Ok(WorkflowStatus::Archived),
        "blocked" => Ok(WorkflowStatus::Blocked),
        _ => Err(WorkflowPersistenceError::Serialization(
            "unknown workflow status".into(),
        )),
    }
}
fn node_type_name(kind: WorkflowNodeType) -> &'static str {
    match kind {
        WorkflowNodeType::Agent => "agent",
        WorkflowNodeType::Tool => "tool",
        WorkflowNodeType::Python => "python",
        WorkflowNodeType::Condition => "condition",
        WorkflowNodeType::Parallel => "parallel",
        WorkflowNodeType::Delay => "delay",
        WorkflowNodeType::Approval => "approval",
        WorkflowNodeType::SubWorkflow => "sub_workflow",
    }
}
fn parse_node_type(text: String) -> Result<WorkflowNodeType, WorkflowPersistenceError> {
    serde_json::from_value(serde_json::Value::String(text))
        .map_err(|error| WorkflowPersistenceError::Serialization(error.to_string()))
}
fn cancel_policy_name(policy: CancelPolicy) -> &'static str {
    match policy {
        CancelPolicy::Cooperative => "cooperative",
        CancelPolicy::Immediate => "immediate",
    }
}
fn parse_cancel_policy(text: String) -> Result<CancelPolicy, WorkflowPersistenceError> {
    match text.as_str() {
        "cooperative" => Ok(CancelPolicy::Cooperative),
        "immediate" => Ok(CancelPolicy::Immediate),
        _ => Err(WorkflowPersistenceError::Serialization(
            "unknown cancel policy".into(),
        )),
    }
}
