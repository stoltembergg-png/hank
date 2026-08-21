//! Tool trait definition - the core contract for all executable tools.

use crate::error::ToolError;
use crate::request::ToolRequest;
use crate::response::{ToolOutcome, ToolResponse};
use crate::schema::ToolSchema;
use async_trait::async_trait;
use serde_json::Value;

/// Core trait that all tools must implement.
///
/// Tools are async, provider-agnostic, and receive explicit context
/// including project identity, capability, policy decision, budget, and trace.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool's schema (name, version, input/output schemas, etc.).
    fn schema(&self) -> &'static ToolSchema;

    /// Executes the tool with the given request.
    ///
    /// The request contains the input payload, execution context, and operation key.
    /// Returns a response with outcome, payload, and observability metadata.
    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError>;

    /// Validates that the tool can handle the given request.
    /// Called before execution to fail fast on schema/capability mismatches.
    fn can_handle(&self, request: &ToolRequest) -> Result<(), ToolError> {
        let schema = self.schema();
        if request.tool_name != schema.name {
            return Err(ToolError::NotFound {
                name: request.tool_name.clone(),
            });
        }
        if request.tool_version != schema.version {
            return Err(ToolError::VersionNotFound {
                name: request.tool_name.clone(),
                version: request.tool_version.clone(),
            });
        }
        // Validate capability
        if !schema.capabilities.contains(&request.context.capability) {
            return Err(ToolError::CapabilityMismatch {
                name: schema.name.clone(),
                capability: request.context.capability.clone(),
            });
        }
        // Check policy decision
        if request.context.policy_decision == crate::context::PolicyDecision::Deny {
            return Err(ToolError::PermissionDenied {
                decision: crate::context::PolicyDecision::Deny,
            });
        }
        Ok(())
    }

    /// Returns the tool's declared capabilities.
    fn capabilities(&self) -> &[String] {
        &self.schema().capabilities
    }

    /// Returns whether the tool is destructive.
    fn is_destructive(&self) -> bool {
        self.schema().destructive
    }

    /// Returns the required execution environment.
    fn environment(&self) -> crate::schema::ToolEnvironment {
        self.schema().environment
    }
}

/// Blanket implementation for boxed tools.
#[async_trait]
impl<T: Tool + ?Sized> Tool for Box<T> {
    fn schema(&self) -> &'static ToolSchema {
        (**self).schema()
    }

    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        (**self).execute(request).await
    }

    fn can_handle(&self, request: &ToolRequest) -> Result<(), ToolError> {
        (**self).can_handle(request)
    }

    fn capabilities(&self) -> &[String] {
        (**self).capabilities()
    }

    fn is_destructive(&self) -> bool {
        (**self).is_destructive()
    }

    fn environment(&self) -> crate::schema::ToolEnvironment {
        (**self).environment()
    }
}

/// Helper to create a successful response.
pub fn success_response(request: &ToolRequest, payload: Value, duration_ms: u64) -> ToolResponse {
    ToolResponse {
        operation_key: request.operation_key,
        tool_name: request.tool_name.clone(),
        tool_version: request.tool_version.clone(),
        outcome: ToolOutcome::Success,
        payload,
        trace_id: request.context.trace_id,
        duration_ms,
        metadata: BTreeMap::new(),
    }
}

/// Helper to create an error response.
pub fn error_response(
    request: &ToolRequest,
    outcome: ToolOutcome,
    error: impl Into<String>,
    duration_ms: u64,
) -> ToolResponse {
    ToolResponse {
        operation_key: request.operation_key,
        tool_name: request.tool_name.clone(),
        tool_version: request.tool_version.clone(),
        outcome,
        payload: serde_json::json!({ "error": error.into() }),
        trace_id: request.context.trace_id,
        duration_ms,
        metadata: BTreeMap::new(),
    }
}

use std::collections::BTreeMap;
