//! Tool execution context carrying identity, capability, policy, budget, and trace.

use agent_core::budget::{BudgetLimits, ReservationId};
use agent_core::ids::{AgentId, ProjectId, SessionId, TaskId, WorkflowId};
use agent_protocol::ids::TraceId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Execution context supplied to every tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    /// Owning project identity.
    pub project_id: ProjectId,
    /// Optional agent identity.
    pub agent_id: Option<AgentId>,
    /// Optional session identity.
    pub session_id: Option<SessionId>,
    /// Optional task identity.
    pub task_id: Option<TaskId>,
    /// Optional workflow identity.
    pub workflow_id: Option<WorkflowId>,
    /// Declared capability required for this call.
    pub capability: String,
    /// Policy decision from permission evaluator.
    pub policy_decision: PolicyDecision,
    /// Budget limits for this invocation.
    pub budget_limits: BudgetLimits,
    /// Optional active reservation ID.
    pub reservation_id: Option<ReservationId>,
    /// Distributed trace identifier.
    pub trace_id: TraceId,
    /// Metadata (non-sensitive) for observability.
    pub metadata: BTreeMap<String, String>,
}

/// Permission evaluation outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Execution allowed without further confirmation.
    Allow,
    /// Human confirmation required once per scope.
    AskOnce,
    /// Human confirmation required for every invocation.
    AskEveryTime,
    /// Execution denied.
    Deny,
}

impl ToolContext {
    /// Validates that required fields are present.
    pub fn validate(&self) -> Result<(), ToolContextError> {
        if self.capability.trim().is_empty() {
            return Err(ToolContextError::MissingCapability);
        }
        if self.trace_id.to_string().trim().is_empty() {
            return Err(ToolContextError::MissingTraceId);
        }
        Ok(())
    }
}

/// Errors during context validation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ToolContextError {
    #[error("capability is required")]
    MissingCapability,
    #[error("trace_id is required")]
    MissingTraceId,
}
