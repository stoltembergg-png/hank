//! Bounded invocation ancestry graph; no scheduler or execution semantics.

use agent_protocol::InvocationRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub const MAX_INVOCATION_GRAPH_NODES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationNodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvocationGraphError {
    #[error("invocation request is invalid")]
    InvalidRequest,
    #[error("invocation parent is missing")]
    MissingParent,
    #[error("invocation graph scope mismatch")]
    ScopeMismatch,
    #[error("invocation node is duplicate")]
    Duplicate,
    #[error("invocation graph node limit reached")]
    NodeLimit,
}

#[derive(Debug, Clone)]
struct InvocationNode {
    request: InvocationRequest,
    parent: Option<uuid::Uuid>,
    status: InvocationNodeStatus,
}

#[derive(Debug, Default)]
pub struct InvocationGraph {
    nodes: HashMap<uuid::Uuid, InvocationNode>,
}

impl InvocationGraph {
    pub fn register(
        &mut self,
        request: InvocationRequest,
        parent: Option<uuid::Uuid>,
    ) -> Result<(), InvocationGraphError> {
        request
            .validate()
            .map_err(|_| InvocationGraphError::InvalidRequest)?;
        if self.nodes.contains_key(&request.invocation_id) {
            return Err(InvocationGraphError::Duplicate);
        }
        if self.nodes.len() >= MAX_INVOCATION_GRAPH_NODES {
            return Err(InvocationGraphError::NodeLimit);
        }
        if let Some(parent_id) = parent {
            let parent_node = self
                .nodes
                .get(&parent_id)
                .ok_or(InvocationGraphError::MissingParent)?;
            if parent_node.request.project_id != request.project_id {
                return Err(InvocationGraphError::ScopeMismatch);
            }
        }
        self.nodes.insert(
            request.invocation_id,
            InvocationNode {
                status: status_from_protocol(request.status),
                request,
                parent,
            },
        );
        Ok(())
    }

    pub fn status(&self, id: uuid::Uuid) -> Option<InvocationNodeStatus> {
        self.nodes.get(&id).map(|node| node.status)
    }

    pub fn cancel(&mut self, id: uuid::Uuid) -> bool {
        let Some(node) = self.nodes.get_mut(&id) else {
            return false;
        };
        if node.status == InvocationNodeStatus::Cancelled {
            return false;
        }
        node.status = InvocationNodeStatus::Cancelled;
        true
    }

    pub fn parent(&self, id: uuid::Uuid) -> Option<Option<uuid::Uuid>> {
        self.nodes.get(&id).map(|node| node.parent)
    }
}

fn status_from_protocol(status: agent_protocol::InvocationStatus) -> InvocationNodeStatus {
    match status {
        agent_protocol::InvocationStatus::Pending => InvocationNodeStatus::Pending,
        agent_protocol::InvocationStatus::Running => InvocationNodeStatus::Running,
        agent_protocol::InvocationStatus::Completed => InvocationNodeStatus::Completed,
        agent_protocol::InvocationStatus::Failed => InvocationNodeStatus::Failed,
        agent_protocol::InvocationStatus::Cancelled => InvocationNodeStatus::Cancelled,
    }
}
