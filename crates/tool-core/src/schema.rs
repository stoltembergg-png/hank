//! Tool schema definition and validation.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tool schema describing input, output, capabilities, and constraints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSchema {
    /// Tool name (unique within registry).
    pub name: String,
    /// Schema version.
    pub version: String,
    /// Human-readable description (non-trusted).
    pub description: Option<String>,
    /// Input JSON schema.
    #[schemars(with = "serde_json::Value")]
    pub input_schema: serde_json::Value,
    /// Output JSON schema.
    #[schemars(with = "serde_json::Value")]
    pub output_schema: serde_json::Value,
    /// Declared capabilities this tool provides/requires.
    pub capabilities: Vec<String>,
    /// Whether the tool has destructive side effects.
    pub destructive: bool,
    /// Required execution environment.
    pub environment: ToolEnvironment,
    /// Default timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum input payload size in bytes.
    pub max_input_bytes: usize,
    /// Maximum output payload size in bytes.
    pub max_output_bytes: usize,
    /// Additional metadata.
    pub metadata: BTreeMap<String, String>,
}

/// Execution environment requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolEnvironment {
    /// Runs in the host process (no sandbox).
    Host,
    /// Runs in a sandboxed process.
    Sandbox,
    /// Runs in a Python worker.
    Python,
    /// Runs in a remote/external process.
    Remote,
}

impl ToolSchema {
    /// Validates the schema itself.
    pub fn validate(&self) -> Result<(), ToolSchemaError> {
        if self.name.trim().is_empty() {
            return Err(ToolSchemaError::MissingName);
        }
        if self.version.trim().is_empty() {
            return Err(ToolSchemaError::MissingVersion);
        }
        if self.timeout_seconds == 0 {
            return Err(ToolSchemaError::InvalidTimeout);
        }
        if self.max_input_bytes == 0 || self.max_output_bytes == 0 {
            return Err(ToolSchemaError::InvalidPayloadLimit);
        }
        // Validate JSON schemas are objects
        if !self.input_schema.is_object() {
            return Err(ToolSchemaError::InvalidInputSchema);
        }
        if !self.output_schema.is_object() {
            return Err(ToolSchemaError::InvalidOutputSchema);
        }
        Ok(())
    }
}

/// Errors during schema validation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ToolSchemaError {
    #[error("schema name is required")]
    MissingName,
    #[error("schema version is required")]
    MissingVersion,
    #[error("timeout must be > 0")]
    InvalidTimeout,
    #[error("payload limits must be > 0")]
    InvalidPayloadLimit,
    #[error("input_schema must be a JSON object")]
    InvalidInputSchema,
    #[error("output_schema must be a JSON object")]
    InvalidOutputSchema,
}
