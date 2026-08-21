//! Tool request carrying input, context, and operation identity.

use agent_protocol::ids::OperationKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Request to execute a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    /// Unique operation key for idempotency.
    pub operation_key: OperationKey,
    /// Tool name.
    pub tool_name: String,
    /// Tool version.
    pub tool_version: String,
    /// Input arguments as JSON.
    pub input: serde_json::Value,
    /// Execution context.
    pub context: super::context::ToolContext,
    /// Optional timeout override in seconds.
    pub timeout_seconds: Option<u64>,
    /// Optional metadata for trace correlation.
    pub metadata: BTreeMap<String, String>,
}

impl ToolRequest {
    pub fn validate(&self) -> Result<(), ToolRequestError> {
        if self.tool_name.trim().is_empty() {
            return Err(ToolRequestError::MissingToolName);
        }
        if self.tool_version.trim().is_empty() {
            return Err(ToolRequestError::MissingToolVersion);
        }
        if self.operation_key.to_string().trim().is_empty() {
            return Err(ToolRequestError::MissingOperationKey);
        }
        self.context.validate()?;
        Ok(())
    }
}

/// Errors during request validation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ToolRequestError {
    #[error("tool_name is required")]
    MissingToolName,
    #[error("tool_version is required")]
    MissingToolVersion,
    #[error("operation_key is required")]
    MissingOperationKey,
    #[error(transparent)]
    Context(#[from] super::context::ToolContextError),
}
