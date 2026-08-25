use agent_core::budget::BudgetLimits;
use agent_core::ids::ProjectId;
use agent_protocol::ids::{OperationKey, TraceId};
use agent_runtime::python_executor::{WorkerTransport, WorkerTransportError};
use agent_runtime::python_lifecycle::{
    LifecycleState, PythonLifecycle, PythonLifecycleConfig, WorkerIdentity,
};
use agent_runtime::python_node::{PythonNodeAdapter, PythonNodeRequest};
use async_trait::async_trait;
use provider_core::CancellationToken;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tool_core::registry::ToolRegistry;
use tool_core::{PolicyDecision, ToolContext, ToolOutcome, ToolRequest};

struct ClosedTransport;
#[async_trait]
impl WorkerTransport for ClosedTransport {
    async fn send_frame(&mut self, _frame: &[u8]) -> Result<(), WorkerTransportError> {
        Err(WorkerTransportError::Closed)
    }
    async fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, WorkerTransportError> {
        Ok(None)
    }
}

fn request() -> ToolRequest {
    ToolRequest {
        operation_key: OperationKey::new(),
        tool_name: "python.demo".into(),
        tool_version: "1.0.0".into(),
        input: serde_json::json!({"value": true}),
        context: ToolContext {
            project_id: ProjectId::new(),
            agent_id: None,
            session_id: None,
            task_id: None,
            workflow_id: None,
            capability: "python:execute".into(),
            policy_decision: PolicyDecision::Allow,
            budget_limits: BudgetLimits::default(),
            reservation_id: None,
            trace_id: TraceId::new(),
            metadata: BTreeMap::new(),
        },
        timeout_seconds: Some(1),
        metadata: BTreeMap::new(),
    }
}

fn lifecycle(request: &ToolRequest) -> PythonLifecycle {
    PythonLifecycle::new(
        PythonLifecycleConfig {
            command: PathBuf::from("python-not-required"),
            args: vec![],
            startup_timeout: Duration::from_secs(1),
            request_timeout: Duration::from_secs(1),
            max_restarts: 0,
            restart_backoff: Duration::ZERO,
        },
        WorkerIdentity {
            project_id: request.context.project_id.to_string(),
            session_id: "session".into(),
            task_id: "task".into(),
            trace_id: request.context.trace_id.to_string(),
        },
    )
    .unwrap()
}

fn adapter() -> PythonNodeAdapter {
    PythonNodeAdapter::new(Arc::new(ToolRegistry::new()))
}

// @spec:AC-982
#[tokio::test]
async fn python_node_keeps_execution_inside_existing_executor_boundary() {
    let req = request();
    let mut life = lifecycle(&req);
    let mut transport = ClosedTransport;
    let result = adapter()
        .execute(
            PythonNodeRequest {
                request: req.clone(),
                cancellation: CancellationToken::new(),
            },
            &mut life,
            &mut transport,
        )
        .await;
    assert_eq!(result.outcome, ToolOutcome::NotFound);
    assert_eq!(result.operation_key, req.operation_key);
    assert_eq!(life.state(), LifecycleState::Stopped);
}

// @spec:AC-983
#[tokio::test]
async fn cancelled_python_node_fails_before_lifecycle_or_worker_dispatch() {
    let req = request();
    let mut life = lifecycle(&req);
    let mut transport = ClosedTransport;
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = adapter()
        .execute(
            PythonNodeRequest {
                request: req.clone(),
                cancellation,
            },
            &mut life,
            &mut transport,
        )
        .await;
    assert_eq!(result.outcome, ToolOutcome::Cancelled);
    assert_eq!(result.operation_key, req.operation_key);
    assert_eq!(life.state(), LifecycleState::Stopped);
}

// @spec:AC-984
#[tokio::test]
async fn invalid_python_node_preserves_trace_and_operation_correlation() {
    let req = request();
    let trace = req.context.trace_id;
    let key = req.operation_key;
    let mut life = lifecycle(&req);
    let mut transport = ClosedTransport;
    let result = adapter()
        .execute(
            PythonNodeRequest {
                request: req,
                cancellation: CancellationToken::new(),
            },
            &mut life,
            &mut transport,
        )
        .await;
    assert_eq!(result.trace_id, trace);
    assert_eq!(result.operation_key, key);
}
