//! Declarative Python tool registration; never executes Python code.

use crate::registry::{ToolOrigin, ToolRegistrationRequest, ToolScope};
use crate::{Tool, ToolError, ToolSchema};
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PythonRegistrationError {
    #[error("python registration schema is invalid")]
    InvalidSchema,
    #[error("python registration requires a Python environment")]
    InvalidEnvironment,
    #[error("python registration requires project scope")]
    InvalidScope,
    #[error("python worker identity is invalid")]
    InvalidWorker,
    #[error("python registration origin is unauthorized")]
    UnauthorizedOrigin,
}

/// Declarative metadata from a Python worker. It is not an executable handler.
pub struct PythonToolRegistration {
    schema: ToolSchema,
    worker_id: String,
    project_id: ProjectId,
    trace_id: TraceId,
    origin: ToolOrigin,
}

impl PythonToolRegistration {
    pub fn new(
        schema: ToolSchema,
        worker_id: impl Into<String>,
        project_id: ProjectId,
        trace_id: TraceId,
    ) -> Self {
        Self {
            schema,
            worker_id: worker_id.into(),
            project_id,
            trace_id,
            origin: ToolOrigin::Project(project_id),
        }
    }

    pub fn schema_mut(&mut self) -> &mut ToolSchema {
        &mut self.schema
    }

    pub fn with_origin(mut self, origin: ToolOrigin) -> Self {
        self.origin = origin;
        self
    }

    pub fn into_request(self) -> Result<ToolRegistrationRequest, PythonRegistrationError> {
        self.schema
            .validate()
            .map_err(|_| PythonRegistrationError::InvalidSchema)?;
        if self.schema.environment != crate::ToolEnvironment::Python {
            return Err(PythonRegistrationError::InvalidEnvironment);
        }
        if self.worker_id.is_empty()
            || self.worker_id.len() > 128
            || self.worker_id.chars().any(char::is_control)
        {
            return Err(PythonRegistrationError::InvalidWorker);
        }
        if !matches!(self.origin, ToolOrigin::Project(project) if project == self.project_id) {
            return Err(PythonRegistrationError::UnauthorizedOrigin);
        }
        let schema = Box::leak(Box::new(self.schema));
        let tool = Arc::new(PythonRegisteredTool {
            schema,
            worker_id: self.worker_id,
        });
        Ok(ToolRegistrationRequest::new(
            tool,
            self.origin,
            ToolScope::Project(self.project_id),
            self.trace_id,
        ))
    }
}

struct PythonRegisteredTool {
    schema: &'static ToolSchema,
    worker_id: String,
}

#[async_trait]
impl Tool for PythonRegisteredTool {
    fn schema(&self) -> &'static ToolSchema {
        self.schema
    }

    async fn execute(
        &self,
        _request: crate::ToolRequest,
    ) -> Result<crate::ToolResponse, ToolError> {
        let _ = &self.worker_id;
        Err(ToolError::PermissionDenied {
            decision: crate::PolicyDecision::Deny,
        })
    }
}
