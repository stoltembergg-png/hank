//! Provider-neutral ToolNode adapter over Tool Runtime boundaries.

use agent_protocol::ids::TraceId;
use provider_core::CancellationToken;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tool_core::registry::{RegistryError, ToolLookupRequest, ToolRegistry};
use tool_core::{
    PermissionDecision, PermissionEvaluator, PermissionRequest, ToolError, ToolOutcome,
    ToolRequest, ToolResponse,
};

const MAX_CACHE_ENTRIES: usize = 1024;

#[derive(Debug, Clone)]
pub struct ToolNodeRequest {
    pub request: ToolRequest,
    pub permission: PermissionRequest,
    pub cancellation: CancellationToken,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ToolNodeError {
    #[error("tool node request is invalid")]
    InvalidRequest,
    #[error("tool node operation was cancelled")]
    Cancelled,
    #[error("tool node permission denied")]
    PermissionDenied,
    #[error("tool node tool was not found")]
    NotFound,
    #[error("tool node capability mismatch")]
    CapabilityMismatch,
    #[error("tool node input schema is invalid")]
    SchemaInvalid,
    #[error("tool node timed out")]
    Timeout,
    #[error("tool node execution failed")]
    ExecutionFailed,
    #[error("tool node idempotency cache is full")]
    CacheFull,
}

pub struct ToolNodeAdapter {
    registry: Arc<ToolRegistry>,
    permissions: Arc<PermissionEvaluator>,
    cache: Mutex<BTreeMap<String, ToolResponse>>,
}

impl ToolNodeAdapter {
    pub fn new(registry: Arc<ToolRegistry>, permissions: Arc<PermissionEvaluator>) -> Self {
        Self {
            registry,
            permissions,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub async fn execute(&self, node: ToolNodeRequest) -> Result<ToolResponse, ToolNodeError> {
        validate_request(&node)?;
        if node.cancellation.is_cancelled() {
            return Err(ToolNodeError::Cancelled);
        }
        let operation_key = node.request.operation_key.to_string();
        if let Some(response) = self.cached(&operation_key)? {
            return Ok(response);
        }

        match self.permissions.evaluate(&node.permission) {
            PermissionDecision::Allowed { .. } => {}
            PermissionDecision::NeedsConfirmation { .. } | PermissionDecision::Denied { .. } => {
                return Err(ToolNodeError::PermissionDenied)
            }
        }
        let schema = self.resolve_schema(&node.request)?;
        schema
            .validate_input(
                &node.request.input,
                tool_core::SchemaValidationPolicy::strict(),
            )
            .map_err(|_| ToolNodeError::SchemaInvalid)?;
        let tool = self
            .registry
            .resolve(&ToolLookupRequest::new(
                node.request.tool_name.clone(),
                node.request.tool_version.clone(),
                node.request.context.project_id,
                Some(node.request.context.capability.clone()),
                node.request.context.trace_id,
            ))
            .map_err(map_registry_error)?;
        tool.can_handle(&node.request)
            .map_err(|_| ToolNodeError::PermissionDenied)?;
        let timeout_seconds = node
            .request
            .timeout_seconds
            .unwrap_or(schema.timeout_seconds);
        let response = tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            tool.execute(node.request.clone()),
        )
        .await
        .map_err(|_| ToolNodeError::Timeout)?
        .map_err(|_| ToolNodeError::ExecutionFailed)?;
        if node.cancellation.is_cancelled() {
            return Err(ToolNodeError::Cancelled);
        }
        if response.outcome != ToolOutcome::Success {
            return Err(ToolNodeError::ExecutionFailed);
        }
        self.store(operation_key, response.clone())?;
        Ok(response)
    }

    fn resolve_schema(
        &self,
        request: &ToolRequest,
    ) -> Result<&'static tool_core::ToolSchema, ToolNodeError> {
        let tool = self
            .registry
            .resolve(&ToolLookupRequest::new(
                request.tool_name.clone(),
                request.tool_version.clone(),
                request.context.project_id,
                Some(request.context.capability.clone()),
                request.context.trace_id,
            ))
            .map_err(map_registry_error)?;
        Ok(tool.schema())
    }

    fn cached(&self, key: &str) -> Result<Option<ToolResponse>, ToolNodeError> {
        self.cache
            .lock()
            .map_err(|_| ToolNodeError::CacheFull)
            .map(|cache| cache.get(key).cloned())
    }

    fn store(&self, key: String, response: ToolResponse) -> Result<(), ToolNodeError> {
        let mut cache = self.cache.lock().map_err(|_| ToolNodeError::CacheFull)?;
        if cache.len() >= MAX_CACHE_ENTRIES && !cache.contains_key(&key) {
            return Err(ToolNodeError::CacheFull);
        }
        cache.insert(key, response);
        Ok(())
    }
}

fn validate_request(node: &ToolNodeRequest) -> Result<(), ToolNodeError> {
    node.request
        .validate()
        .map_err(|_| ToolNodeError::InvalidRequest)?;
    node.permission
        .validate()
        .map_err(|_| ToolNodeError::PermissionDenied)?;
    if node.request.context.project_id
        != node
            .permission
            .project_id
            .ok_or(ToolNodeError::PermissionDenied)?
        || node.request.tool_name != node.permission.tool_name
        || node.request.tool_version != node.permission.tool_version
        || node.request.context.capability != node.permission.capability
    {
        return Err(ToolNodeError::PermissionDenied);
    }
    Ok(())
}

fn map_registry_error(error: RegistryError) -> ToolNodeError {
    match error {
        RegistryError::NotFound { .. } => ToolNodeError::NotFound,
        RegistryError::CapabilityMismatch => ToolNodeError::CapabilityMismatch,
        _ => ToolNodeError::NotFound,
    }
}

#[allow(dead_code)]
fn _trace(_: TraceId, _: ToolError) {}
