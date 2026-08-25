use agent_protocol::{AgentId, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub mod approval;
pub mod condition;
pub mod delay;
pub mod execution;
pub mod parallel;

pub use execution::{ExecutionError, NodeRunState, RetryEnvelope, RunState, WorkflowRun};

pub const WORKFLOW_SCHEMA_VERSION: u32 = 1;
const MAX_NAME_BYTES: usize = 128;
const MAX_POLICY_BYTES: usize = 128;
const MAX_METADATA_ENTRIES: usize = 32;
const MAX_METADATA_VALUE_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Draft,
    Active,
    Paused,
    Archived,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowError {
    #[error("workflow identity is invalid")]
    InvalidIdentity,
    #[error("workflow name is invalid or oversized")]
    InvalidName,
    #[error("workflow policy reference is invalid or oversized")]
    InvalidPolicy,
    #[error("workflow version is invalid")]
    InvalidVersion,
    #[error("workflow metadata is invalid or oversized")]
    MetadataTooLarge,
    #[error("workflow lifecycle transition is invalid")]
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub workflow_id: uuid::Uuid,
    pub project_id: ProjectId,
    pub owner_id: AgentId,
    pub name: String,
    pub version: u32,
    pub status: WorkflowStatus,
    pub policy_ref: String,
    pub metadata: BTreeMap<String, String>,
}

fn deserialize_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = u32::deserialize(deserializer)?;
    if version == WORKFLOW_SCHEMA_VERSION {
        Ok(version)
    } else {
        Err(serde::de::Error::custom("unknown workflow schema version"))
    }
}

impl Workflow {
    pub fn new(
        project_id: ProjectId,
        owner_id: AgentId,
        name: String,
        policy_ref: String,
    ) -> Result<Self, WorkflowError> {
        if name.trim().is_empty() || name.len() > MAX_NAME_BYTES {
            return Err(WorkflowError::InvalidName);
        }
        if policy_ref.trim().is_empty() || policy_ref.len() > MAX_POLICY_BYTES {
            return Err(WorkflowError::InvalidPolicy);
        }
        Ok(Self {
            schema_version: WORKFLOW_SCHEMA_VERSION,
            workflow_id: uuid::Uuid::new_v4(),
            project_id,
            owner_id,
            name,
            version: 1,
            status: WorkflowStatus::Draft,
            policy_ref,
            metadata: BTreeMap::new(),
        })
    }
    pub fn set_version(&mut self, version: u32) -> Result<(), WorkflowError> {
        if version == 0 || version < self.version {
            return Err(WorkflowError::InvalidVersion);
        }
        self.version = version;
        Ok(())
    }
    pub fn set_metadata(&mut self, key: String, value: String) -> Result<(), WorkflowError> {
        if key.trim().is_empty()
            || value.len() > MAX_METADATA_VALUE_BYTES
            || (!self.metadata.contains_key(&key) && self.metadata.len() >= MAX_METADATA_ENTRIES)
        {
            return Err(WorkflowError::MetadataTooLarge);
        }
        self.metadata.insert(key, value);
        Ok(())
    }
    pub fn activate(&mut self) -> Result<(), WorkflowError> {
        if !matches!(self.status, WorkflowStatus::Draft | WorkflowStatus::Paused) {
            return Err(WorkflowError::InvalidTransition);
        }
        self.status = WorkflowStatus::Active;
        Ok(())
    }
    pub fn pause(&mut self) -> Result<(), WorkflowError> {
        if self.status != WorkflowStatus::Active {
            return Err(WorkflowError::InvalidTransition);
        }
        self.status = WorkflowStatus::Paused;
        Ok(())
    }
    pub fn archive(&mut self) -> Result<(), WorkflowError> {
        if matches!(
            self.status,
            WorkflowStatus::Archived | WorkflowStatus::Blocked
        ) {
            return Err(WorkflowError::InvalidTransition);
        }
        if self.status != WorkflowStatus::Active && self.status != WorkflowStatus::Paused {
            return Err(WorkflowError::InvalidTransition);
        }
        self.status = WorkflowStatus::Archived;
        Ok(())
    }
    pub fn block(&mut self) {
        self.status = WorkflowStatus::Blocked;
    }
}

pub const WORKFLOW_NODE_SCHEMA_VERSION: u32 = 1;
const MAX_NODE_ID_BYTES: usize = 128;
const MAX_NODE_PAYLOAD_BYTES: usize = 16 * 1024;
const MAX_NODE_CAPABILITIES: usize = 32;
const MAX_NODE_TIMEOUT_MS: u64 = 3_600_000;
const MAX_RETRY_ATTEMPTS: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeType {
    Agent,
    Tool,
    Python,
    Condition,
    Parallel,
    Delay,
    Approval,
    SubWorkflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelPolicy {
    Cooperative,
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowNodeError {
    #[error("workflow node identity is invalid")]
    InvalidIdentity,
    #[error("workflow node schema version is invalid")]
    InvalidSchemaVersion,
    #[error("workflow node payload is oversized or invalid")]
    PayloadTooLarge,
    #[error("workflow node timeout or retry policy is invalid")]
    InvalidPolicy,
    #[error("workflow node capability requirements are invalid")]
    InvalidCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowNode {
    #[serde(deserialize_with = "deserialize_node_schema_version")]
    pub schema_version: u32,
    pub node_id: String,
    pub workflow_id: String,
    pub workflow_version: u32,
    #[serde(rename = "type")]
    pub node_type: WorkflowNodeType,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub timeout_ms: u64,
    pub retry: RetryPolicy,
    pub cancel: CancelPolicy,
    pub required_capabilities: Vec<agent_protocol::capability::Capability>,
}

fn deserialize_node_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = u32::deserialize(deserializer)?;
    if version == WORKFLOW_NODE_SCHEMA_VERSION {
        Ok(version)
    } else {
        Err(serde::de::Error::custom(
            "unknown workflow node schema version",
        ))
    }
}

impl WorkflowNode {
    pub fn new(
        node_id: String,
        workflow_id: String,
        workflow_version: u32,
        node_type: WorkflowNodeType,
        input_schema: serde_json::Value,
    ) -> Result<Self, WorkflowNodeError> {
        let node = Self {
            schema_version: WORKFLOW_NODE_SCHEMA_VERSION,
            node_id,
            workflow_id,
            workflow_version,
            node_type,
            input_schema,
            output_schema: serde_json::json!({}),
            timeout_ms: 30_000,
            retry: RetryPolicy { max_attempts: 1 },
            cancel: CancelPolicy::Cooperative,
            required_capabilities: Vec::new(),
        };
        node.validate()?;
        Ok(node)
    }

    pub fn validate(&self) -> Result<(), WorkflowNodeError> {
        if self.schema_version != WORKFLOW_NODE_SCHEMA_VERSION
            || self.node_id.trim().is_empty()
            || self.node_id.len() > MAX_NODE_ID_BYTES
            || self.workflow_id.trim().is_empty()
            || self.workflow_id.len() > MAX_NODE_ID_BYTES
            || self.workflow_version == 0
        {
            return Err(WorkflowNodeError::InvalidIdentity);
        }
        let input_bytes = serde_json::to_vec(&self.input_schema)
            .map_err(|_| WorkflowNodeError::PayloadTooLarge)?
            .len();
        let output_bytes = serde_json::to_vec(&self.output_schema)
            .map_err(|_| WorkflowNodeError::PayloadTooLarge)?
            .len();
        if input_bytes > MAX_NODE_PAYLOAD_BYTES || output_bytes > MAX_NODE_PAYLOAD_BYTES {
            return Err(WorkflowNodeError::PayloadTooLarge);
        }
        if self.timeout_ms == 0
            || self.timeout_ms > MAX_NODE_TIMEOUT_MS
            || self.retry.max_attempts == 0
            || self.retry.max_attempts > MAX_RETRY_ATTEMPTS
        {
            return Err(WorkflowNodeError::InvalidPolicy);
        }
        if self.required_capabilities.len() > MAX_NODE_CAPABILITIES
            || self
                .required_capabilities
                .iter()
                .any(|capability| capability.scope.as_deref() == Some(""))
        {
            return Err(WorkflowNodeError::InvalidCapabilities);
        }
        Ok(())
    }
}

pub const WORKFLOW_GRAPH_MAX_NODES: usize = 512;
pub const WORKFLOW_GRAPH_MAX_EDGES: usize = 1024;
const MAX_EDGE_ID_BYTES: usize = 128;
const MAX_PORT_BYTES: usize = 64;
const MAX_CONDITION_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowEdge {
    pub edge_id: String,
    pub workflow_id: String,
    pub source_node: String,
    pub source_port: String,
    pub target_node: String,
    pub target_port: String,
    pub condition: Option<String>,
    pub ordering: u32,
}

impl WorkflowEdge {
    pub fn new(
        edge_id: impl Into<String>,
        source_node: impl Into<String>,
        target_node: impl Into<String>,
    ) -> Self {
        Self {
            edge_id: edge_id.into(),
            workflow_id: "workflow-1".into(),
            source_node: source_node.into(),
            source_port: "out".into(),
            target_node: target_node.into(),
            target_port: "in".into(),
            condition: None,
            ordering: 0,
        }
    }
    fn validate_shape(&self) -> Result<(), WorkflowEdgeError> {
        if self.edge_id.trim().is_empty()
            || self.edge_id.len() > MAX_EDGE_ID_BYTES
            || self.source_node.trim().is_empty()
            || self.target_node.trim().is_empty()
            || self.source_port.len() > MAX_PORT_BYTES
            || self.target_port.len() > MAX_PORT_BYTES
            || self
                .condition
                .as_ref()
                .is_some_and(|value| value.len() > MAX_CONDITION_BYTES)
        {
            return Err(WorkflowEdgeError::InvalidShape);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowEdgeError {
    #[error("workflow edge shape is invalid")]
    InvalidShape,
    #[error("workflow edge references an unknown node")]
    UnknownNode,
    #[error("workflow edge is a self-edge")]
    SelfEdge,
    #[error("workflow edge belongs to another workflow")]
    CrossWorkflow,
    #[error("workflow edge is duplicated")]
    DuplicateEdge,
    #[error("workflow graph has too many nodes or edges")]
    TooManyEdges,
    #[error("workflow graph contains a cycle")]
    Cycle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowGraph {
    pub workflow_id: String,
    pub workflow_version: u32,
    pub nodes: BTreeMap<String, WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

impl WorkflowGraph {
    pub fn new(workflow_id: String, workflow_version: u32) -> Result<Self, WorkflowEdgeError> {
        if workflow_id.trim().is_empty() || workflow_version == 0 {
            return Err(WorkflowEdgeError::InvalidShape);
        }
        Ok(Self {
            workflow_id,
            workflow_version,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
        })
    }
    pub fn add_node(&mut self, node: WorkflowNode) -> Result<(), WorkflowEdgeError> {
        node.validate()
            .map_err(|_| WorkflowEdgeError::InvalidShape)?;
        if node.workflow_id != self.workflow_id || node.workflow_version != self.workflow_version {
            return Err(WorkflowEdgeError::CrossWorkflow);
        }
        if self.nodes.len() >= WORKFLOW_GRAPH_MAX_NODES {
            return Err(WorkflowEdgeError::TooManyEdges);
        }
        if self.nodes.insert(node.node_id.clone(), node).is_some() {
            return Err(WorkflowEdgeError::DuplicateEdge);
        }
        Ok(())
    }
    pub fn add_edge(&mut self, edge: WorkflowEdge) -> Result<(), WorkflowEdgeError> {
        edge.validate_shape()?;
        if edge.workflow_id != self.workflow_id {
            return Err(WorkflowEdgeError::CrossWorkflow);
        }
        if edge.source_node == edge.target_node {
            return Err(WorkflowEdgeError::SelfEdge);
        }
        if !self.nodes.contains_key(&edge.source_node)
            || !self.nodes.contains_key(&edge.target_node)
        {
            return Err(WorkflowEdgeError::UnknownNode);
        }
        if self
            .edges
            .iter()
            .any(|existing| existing.edge_id == edge.edge_id)
        {
            return Err(WorkflowEdgeError::DuplicateEdge);
        }
        if self.edges.len() >= WORKFLOW_GRAPH_MAX_EDGES {
            return Err(WorkflowEdgeError::TooManyEdges);
        }
        self.edges.push(edge);
        Ok(())
    }
    pub fn validate(&self) -> Result<(), WorkflowEdgeError> {
        if self.nodes.len() > WORKFLOW_GRAPH_MAX_NODES
            || self.edges.len() > WORKFLOW_GRAPH_MAX_EDGES
        {
            return Err(WorkflowEdgeError::TooManyEdges);
        }
        let mut edge_ids = std::collections::BTreeSet::new();
        for edge in &self.edges {
            edge.validate_shape()?;
            if !edge_ids.insert(&edge.edge_id) {
                return Err(WorkflowEdgeError::DuplicateEdge);
            }
            if edge.workflow_id != self.workflow_id {
                return Err(WorkflowEdgeError::CrossWorkflow);
            }
            if edge.source_node == edge.target_node {
                return Err(WorkflowEdgeError::SelfEdge);
            }
            if !self.nodes.contains_key(&edge.source_node)
                || !self.nodes.contains_key(&edge.target_node)
            {
                return Err(WorkflowEdgeError::UnknownNode);
            }
        }
        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &self.edges {
            adjacency
                .entry(&edge.source_node)
                .or_default()
                .push(&edge.target_node);
        }
        let mut visiting = BTreeMap::new();
        let mut visited = BTreeMap::new();
        for node in self.nodes.keys() {
            if Self::has_cycle(node, &adjacency, &mut visiting, &mut visited) {
                return Err(WorkflowEdgeError::Cycle);
            }
        }
        Ok(())
    }
    fn has_cycle<'a>(
        node: &'a str,
        adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
        visiting: &mut BTreeMap<&'a str, bool>,
        visited: &mut BTreeMap<&'a str, bool>,
    ) -> bool {
        if visiting.get(node).copied().unwrap_or(false) {
            return true;
        }
        if visited.get(node).copied().unwrap_or(false) {
            return false;
        }
        visiting.insert(node, true);
        if let Some(targets) = adjacency.get(node) {
            if targets
                .iter()
                .any(|target| Self::has_cycle(target, adjacency, visiting, visited))
            {
                return true;
            }
        }
        visiting.remove(node);
        visited.insert(node, true);
        false
    }
}
