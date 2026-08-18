//! Entidades Workflow e DAG de domínio.

use crate::ids::{NodeId, ProjectId, RunId, TraceId, WorkflowId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Tipo de nó do workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Agent,
    Tool,
    Python,
    Condition,
    Parallel,
    Delay,
    Approval,
    SubWorkflow,
}

/// Estado do nó
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
    Blocked,
}

/// Nó do workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: NodeId,
    pub workflow_id: WorkflowId,
    pub node_type: NodeType,
    pub name: String,
    pub config: serde_json::Value,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub dependencies: HashSet<NodeId>,
    pub capability: crate::capability::Capability,
    pub status: NodeStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timeout_seconds: Option<u64>,
    pub checkpoint: Option<serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Estado do workflow run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Pending,
    Running,
    Paused,
    Succeeded,
    Failed,
    Cancelled,
    NeedsReview,
}

/// Run de workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: RunId,
    pub workflow_id: WorkflowId,
    pub project_id: ProjectId,
    pub trace_id: TraceId,
    pub status: WorkflowRunStatus,
    pub current_nodes: HashSet<NodeId>,
    pub completed_nodes: HashSet<NodeId>,
    pub failed_nodes: HashSet<NodeId>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<DateTime<Utc>>,
}

/// Workflow de domínio (DAG versionado)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub nodes: HashMap<NodeId, WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub condition: Option<String>,
}

impl Workflow {
    pub fn new(project_id: ProjectId, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: WorkflowId::new(),
            project_id,
            name,
            description: None,
            version: "1.0.0".into(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.nodes.insert(node.id, node);
        self.updated_at = Utc::now();
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, condition: Option<String>) {
        self.edges.push(WorkflowEdge {
            from,
            to,
            condition,
        });
        self.updated_at = Utc::now();
    }

    pub fn validate_dag(&self) -> Result<(), crate::error::DomainError> {
        // Verifica ciclos usando Kahn's algorithm
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

        for node_id in self.nodes.keys() {
            in_degree.insert(*node_id, 0);
            adj.insert(*node_id, Vec::new());
        }

        for edge in &self.edges {
            *in_degree.get_mut(&edge.to).unwrap() += 1;
            adj.get_mut(&edge.from).unwrap().push(edge.to);
        }

        let mut queue: Vec<NodeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&k, _)| k)
            .collect();

        let mut visited = 0;
        while let Some(node) = queue.pop() {
            visited += 1;
            for &neighbor in &adj[&node] {
                let deg = in_degree.get_mut(&neighbor).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(neighbor);
                }
            }
        }

        if visited != self.nodes.len() {
            return Err(crate::error::DomainError::InvariantViolation(
                "Workflow contains cycles".into(),
            ));
        }

        Ok(())
    }
}
