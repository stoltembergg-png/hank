//! Tool execution errors.

use agent_core::ids::ProjectId;
use thiserror::Error;

/// Errors returned by tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {name}")]
    NotFound { name: String },

    #[error("tool {name} version {version} not found")]
    VersionNotFound { name: String, version: String },

    #[error("tool {name} is not active")]
    NotActive { name: String },

    #[error("tool {name} requires capability {capability}")]
    CapabilityMismatch { name: String, capability: String },

    #[error("permission denied: {decision:?}")]
    PermissionDenied {
        decision: super::context::PolicyDecision,
    },

    #[error("project {0} not authorized for tool {1}")]
    ProjectUnauthorized(ProjectId, String),

    #[error("budget exhausted: {0}")]
    BudgetExhausted(String),

    #[error("timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("cancelled")]
    Cancelled,

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("sandbox error: {0}")]
    Sandbox(String),

    #[error("internal error: {0}")]
    Internal(String),
}
