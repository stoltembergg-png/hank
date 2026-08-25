//! Bounded workflow boundary for the existing optional Python executor.

use crate::python_executor::{PythonExecutor, PythonExecutorConfig, WorkerTransport};
use crate::python_lifecycle::PythonLifecycle;
use provider_core::CancellationToken;
use std::sync::Arc;
use tool_core::registry::ToolRegistry;
use tool_core::{error_response, ToolOutcome, ToolRequest, ToolResponse};

#[derive(Debug, Clone)]
pub struct PythonNodeRequest {
    pub request: ToolRequest,
    pub cancellation: CancellationToken,
}

pub struct PythonNodeAdapter {
    executor: PythonExecutor,
}

impl PythonNodeAdapter {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            executor: PythonExecutor::new(
                registry,
                tool_core::PermissionEvaluator::new(),
                PythonExecutorConfig::default(),
            ),
        }
    }

    pub async fn execute(
        &self,
        node: PythonNodeRequest,
        lifecycle: &mut PythonLifecycle,
        transport: &mut dyn WorkerTransport,
    ) -> ToolResponse {
        if node.cancellation.is_cancelled() {
            return error_response(
                &node.request,
                ToolOutcome::Cancelled,
                "cancelled before python dispatch",
                0,
            );
        }
        let response = self
            .executor
            .invoke(lifecycle, transport, node.request.clone())
            .await;
        if node.cancellation.is_cancelled() && response.outcome == ToolOutcome::Success {
            return error_response(
                &node.request,
                ToolOutcome::Cancelled,
                "cancelled after python dispatch",
                response.duration_ms,
            );
        }
        response
    }
}
