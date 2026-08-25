//! Provider-agnostic agent invocation contract; it never executes transport.

use crate::{AgentId, ProjectId, SessionId, TraceId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const INVOCATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_INVOCATION_TASK_BYTES: usize = 4 * 1024;
pub const MAX_INVOCATION_CONTEXT_REFS: usize = 32;
pub const MAX_INVOCATION_DEPTH: u16 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvocationError {
    #[error("invocation identity is invalid")]
    InvalidIdentity,
    #[error("invocation task is invalid")]
    InvalidTask,
    #[error("invocation budget is invalid")]
    InvalidBudget,
    #[error("invocation context is invalid")]
    InvalidContext,
    #[error("invocation depth limit reached")]
    DepthLimit,
    #[error("invocation context contains duplicate references")]
    DuplicateContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationRequest {
    pub schema_version: u32,
    pub invocation_id: uuid::Uuid,
    pub project_id: ProjectId,
    pub group_id: uuid::Uuid,
    pub session_id: SessionId,
    pub caller_id: AgentId,
    pub callee_id: AgentId,
    pub trace_id: TraceId,
    pub task: String,
    pub context_refs: Vec<String>,
    pub max_tokens: u64,
    pub depth: u16,
    pub status: InvocationStatus,
}

impl InvocationRequest {
    pub fn validate(&self) -> Result<(), InvocationError> {
        if self.schema_version != INVOCATION_SCHEMA_VERSION
            || self.invocation_id.is_nil()
            || self.project_id.to_string().is_empty()
            || self.group_id.is_nil()
            || self.caller_id == self.callee_id
            || self.trace_id.as_uuid().is_nil()
        {
            return Err(InvocationError::InvalidIdentity);
        }
        if self.task.trim().is_empty() || self.task.len() > MAX_INVOCATION_TASK_BYTES {
            return Err(InvocationError::InvalidTask);
        }
        if self.max_tokens == 0 {
            return Err(InvocationError::InvalidBudget);
        }
        if self.depth > MAX_INVOCATION_DEPTH {
            return Err(InvocationError::DepthLimit);
        }
        if self.context_refs.len() > MAX_INVOCATION_CONTEXT_REFS {
            return Err(InvocationError::InvalidContext);
        }
        let mut seen = HashSet::new();
        if self.context_refs.iter().any(|reference| {
            reference.len() > 256
                || !reference.starts_with("project://")
                || reference.contains("..")
                || !seen.insert(reference)
        }) {
            return Err(InvocationError::InvalidContext);
        }
        Ok(())
    }
}
