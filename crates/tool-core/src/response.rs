//! Tool execution response.

use agent_protocol::ids::{OperationKey, TraceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Operation key from the request.
    pub operation_key: OperationKey,
    /// Tool name.
    pub tool_name: String,
    /// Tool version.
    pub tool_version: String,
    /// Execution outcome.
    pub outcome: ToolOutcome,
    /// Output payload (success) or error detail (failure).
    pub payload: serde_json::Value,
    /// Trace identifier.
    pub trace_id: TraceId,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Metadata for observability.
    pub metadata: BTreeMap<String, String>,
}

/// Outcome of tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Success,
    PermissionDenied,
    Timeout,
    Cancelled,
    Failed,
    SchemaValidationError,
    SandboxError,
    BudgetExhausted,
    NotFound,
    CapabilityMismatch,
}
