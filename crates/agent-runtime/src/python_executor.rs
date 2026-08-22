//! Python tool executor: the single execution path from a registered tool
//! invocation to the worker protocol.
//!
//! Every dispatch passes through registry resolution, the permission
//! evaluator and the worker lifecycle — there is no ad hoc subprocess.
//! Results from the worker are untrusted bounded data: outputs are size
//! checked, never interpreted, and errors map deterministically to tool
//! outcomes. The executor requires no Python runtime: the transport is a
//! trait, so tests run against an in-memory fixture worker.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use agent_protocol::capability::{Action, Capability, Resource};
use agent_protocol::envelope::TerminalResult;
use agent_protocol::ids::RequestId;
use agent_protocol::json_rpc::{encode_frame, FrameDecoder, JsonRpcMessage};
use agent_protocol::worker::{WorkerContext, WorkerMessage};
use async_trait::async_trait;
use tool_core::registry::{RegistryError, ToolLookupRequest, ToolRegistry};
use tool_core::{
    error_response, PermissionDecision, PermissionEvaluator, PermissionRequest, ToolEffect,
    ToolEnvironment, ToolError, ToolExecutionWindow, ToolOutcome, ToolRequest, ToolResponse,
};
use uuid::Uuid;

use crate::python_lifecycle::PythonLifecycle;

/// Hard ceiling for worker outputs regardless of tool schema declarations.
pub const MAX_EXECUTOR_OUTPUT_BYTES: usize = 65_536;
/// Default bounded execution window for a python tool dispatch.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Bounded number of frames scanned while waiting for the correlated reply.
const MAX_FRAMES_PER_DISPATCH: usize = 16;

/// Bounded duplex frame transport to a worker session.
#[async_trait]
pub trait WorkerTransport: Send {
    /// Sends one encoded frame; a closed channel fails closed.
    async fn send_frame(&mut self, frame: &[u8]) -> Result<(), WorkerTransportError>;
    /// Receives the next raw frame; `Ok(None)` is a clean channel close.
    async fn recv_frame(&mut self) -> Result<Option<Vec<u8>>, WorkerTransportError>;
}

/// Transport failure with bounded, redacted causes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkerTransportError {
    #[error("worker channel is closed")]
    Closed,
    #[error("worker channel failed")]
    Failed,
}

/// Executor configuration with bounded defaults.
#[derive(Debug, Clone)]
pub struct PythonExecutorConfig {
    pub request_timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for PythonExecutorConfig {
    fn default() -> Self {
        Self {
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_output_bytes: MAX_EXECUTOR_OUTPUT_BYTES,
        }
    }
}

/// Executes registered python tools through the worker protocol.
pub struct PythonExecutor {
    registry: Arc<ToolRegistry>,
    evaluator: PermissionEvaluator,
    config: PythonExecutorConfig,
    next_rpc_id: AtomicU64,
}

impl PythonExecutor {
    pub fn new(
        registry: Arc<ToolRegistry>,
        evaluator: PermissionEvaluator,
        config: PythonExecutorConfig,
    ) -> Self {
        Self {
            registry,
            evaluator,
            config,
            next_rpc_id: AtomicU64::new(1),
        }
    }

    /// Single execution path: registry -> permission -> lifecycle -> worker.
    ///
    /// Always returns a bounded `ToolResponse`; denials and failures are
    /// terminal outcomes, never panics and never ad hoc execution.
    pub async fn invoke(
        &self,
        lifecycle: &mut PythonLifecycle,
        transport: &mut dyn WorkerTransport,
        request: ToolRequest,
    ) -> ToolResponse {
        let started = std::time::Instant::now();
        let duration = |started: std::time::Instant| {
            started.elapsed().as_millis().min(u64::MAX as u128) as u64
        };

        if let Err(error) = request.validate() {
            return error_response(
                &request,
                ToolOutcome::Failed,
                error.to_string(),
                duration(started),
            );
        }

        // 1. Registry gate: only registered, active, capability-matching
        //    python tools can dispatch.
        let lookup = ToolLookupRequest::new(
            request.tool_name.clone(),
            request.tool_version.clone(),
            request.context.project_id,
            Some(request.context.capability.clone()),
            request.context.trace_id,
        );
        let tool = match self.registry.resolve(&lookup) {
            Ok(tool) => tool,
            Err(error) => {
                let outcome = match error {
                    RegistryError::NotFound { .. } | RegistryError::NotActive { .. } => {
                        ToolOutcome::NotFound
                    }
                    RegistryError::CapabilityMismatch => ToolOutcome::CapabilityMismatch,
                    _ => ToolOutcome::Failed,
                };
                let detail = bounded_detail(&error.to_string());
                return error_response(&request, outcome, detail, duration(started));
            }
        };
        if tool.environment() != ToolEnvironment::Python {
            return error_response(
                &request,
                ToolOutcome::Failed,
                "tool is not registered for the python environment",
                duration(started),
            );
        }
        if let Err(error) = tool.can_handle(&request) {
            let detail = bounded_detail(&error.to_string());
            let outcome = match error {
                ToolError::PermissionDenied { .. } => ToolOutcome::PermissionDenied,
                ToolError::CapabilityMismatch { .. } => ToolOutcome::CapabilityMismatch,
                ToolError::NotFound { .. } | ToolError::VersionNotFound { .. } => {
                    ToolOutcome::NotFound
                }
                _ => ToolOutcome::Failed,
            };
            return error_response(&request, outcome, detail, duration(started));
        }
        let schema = tool.schema();

        // 2. Permission gate: python execution is a mutating effect, so
        //    missing approvals deny before any dispatch.
        let permission = PermissionRequest {
            project_id: Some(request.context.project_id),
            tool_name: request.tool_name.clone(),
            tool_version: request.tool_version.clone(),
            capability: request.context.capability.clone(),
            effect: ToolEffect::Execute,
            policy: request.context.policy_decision,
            budget_available: request.context.budget_limits.max_tokens > 0
                && request.context.budget_limits.max_wall_time_seconds > 0,
            confirmation_approved: false,
        };
        match self.evaluator.evaluate(&permission) {
            PermissionDecision::Allowed { .. } => {}
            PermissionDecision::NeedsConfirmation { .. } => {
                return error_response(
                    &request,
                    ToolOutcome::PermissionDenied,
                    "python execution requires an approval artifact",
                    duration(started),
                );
            }
            PermissionDecision::Denied { reason } => {
                return error_response(
                    &request,
                    ToolOutcome::PermissionDenied,
                    reason.to_string(),
                    duration(started),
                );
            }
        }

        // 3. Input bound from the tool schema.
        let input_bytes = serde_json::to_vec(&request.input)
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX);
        if input_bytes == 0 || input_bytes > schema.max_input_bytes {
            return error_response(
                &request,
                ToolOutcome::Failed,
                "input exceeds the tool input bound",
                duration(started),
            );
        }

        // 4. Lifecycle gate: budget reservation, dedupe and readiness.
        let operation_key = request.operation_key.to_string();
        let budget = request.context.budget_limits.max_tokens;
        if let Err(error) = lifecycle.begin_request(&operation_key, budget) {
            let (outcome, detail) = match error {
                crate::python_lifecycle::LifecycleError::DuplicateOperation(_) => (
                    ToolOutcome::Failed,
                    "operation key already consumed; effect is not repeated",
                ),
                crate::python_lifecycle::LifecycleError::InvalidTransition { state, .. } => {
                    (ToolOutcome::SandboxError, lifecycle_state_detail(state))
                }
                _ => (
                    ToolOutcome::SandboxError,
                    "worker lifecycle rejected the request",
                ),
            };
            return error_response(&request, outcome, detail, duration(started));
        }

        // 5. Bounded execution window with cancellation.
        let timeout = request
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(self.config.request_timeout)
            .min(self.config.request_timeout);
        let window = match ToolExecutionWindow::new(timeout) {
            Ok(window) => window,
            Err(_) => {
                let _ = lifecycle.complete_request(&operation_key);
                return error_response(
                    &request,
                    ToolOutcome::Failed,
                    "invalid timeout window",
                    duration(started),
                );
            }
        };
        if window.cancellation().is_cancelled() {
            let _ = lifecycle.cancel_request(&operation_key).await;
            return error_response(
                &request,
                ToolOutcome::Cancelled,
                "cancelled before dispatch",
                duration(started),
            );
        }

        // 6. Worker protocol dispatch over JSON-RPC framing.
        let worker_request = WorkerMessage::Request {
            schema_version: agent_protocol::worker::WORKER_PROTOCOL_SCHEMA_VERSION,
            request_id: RequestId::new(),
            context: WorkerContext {
                project_id: request.context.project_id,
                session_id: request.context.session_id.unwrap_or_default(),
                task_id: request.context.task_id,
                trace_id: request.context.trace_id,
            },
            capability: Capability::new(Resource::Tool, Action::Execute)
                .with_scope(request.context.capability.clone()),
            payload: request.input.clone(),
        };
        let response = self
            .dispatch(
                lifecycle,
                transport,
                &operation_key,
                &window,
                worker_request,
            )
            .await;
        let response = match response {
            Ok(response) => response,
            Err(dispatch) => {
                return error_response(
                    &request,
                    dispatch.outcome,
                    dispatch.detail,
                    duration(started),
                );
            }
        };

        // 7. Bounded, untrusted result mapping.
        let terminal = match &response {
            WorkerMessage::Response { result, .. } => *result,
            _ => TerminalResult::Failed,
        };
        match terminal {
            TerminalResult::Succeeded => {
                let value = match &response {
                    WorkerMessage::Response {
                        value: Some(value), ..
                    } => value.clone(),
                    _ => serde_json::Value::Null,
                };
                let output_bound = schema.max_output_bytes.min(self.config.max_output_bytes);
                let output_bytes = serde_json::to_vec(&value)
                    .map(|bytes| bytes.len())
                    .unwrap_or(usize::MAX);
                if output_bytes > output_bound {
                    let _ = lifecycle.complete_request(&operation_key);
                    return error_response(
                        &request,
                        ToolOutcome::Failed,
                        "worker output exceeds the bounded size",
                        duration(started),
                    );
                }
                let _ = lifecycle.complete_request(&operation_key);
                let mut tool_response =
                    tool_core::success_response(&request, value, duration(started));
                tool_response
                    .metadata
                    .insert("worker_environment".to_string(), "python".to_string());
                tool_response
            }
            TerminalResult::Cancelled => {
                let _ = lifecycle.cancel_request(&operation_key).await;
                error_response(
                    &request,
                    ToolOutcome::Cancelled,
                    "worker reported cancellation",
                    duration(started),
                )
            }
            TerminalResult::TimedOut => {
                let _ = lifecycle.timeout_request(&operation_key).await;
                error_response(
                    &request,
                    ToolOutcome::Timeout,
                    "worker exceeded the execution window",
                    duration(started),
                )
            }
            TerminalResult::Rejected
            | TerminalResult::Failed
            | TerminalResult::NotSupported
            | TerminalResult::Blocked => {
                let detail = match &response {
                    WorkerMessage::Response {
                        error: Some(error), ..
                    } => bounded_detail(&error.detail),
                    _ => "worker rejected the request".to_string(),
                };
                let _ = lifecycle.complete_request(&operation_key);
                error_response(&request, ToolOutcome::Failed, detail, duration(started))
            }
        }
    }

    async fn dispatch(
        &self,
        lifecycle: &mut PythonLifecycle,
        transport: &mut dyn WorkerTransport,
        operation_key: &str,
        window: &ToolExecutionWindow,
        worker_request: WorkerMessage,
    ) -> Result<WorkerMessage, DispatchFailure> {
        let (request_id, expected_context) = match &worker_request {
            WorkerMessage::Request {
                request_id,
                context,
                ..
            } => (*request_id, *context),
            _ => {
                return Err(DispatchFailure::internal(
                    "dispatch requires a worker request",
                ))
            }
        };

        let mut params = serde_json::to_value(&worker_request)
            .map_err(|_| DispatchFailure::internal("request serialization failed"))?;
        let Some(object) = params.as_object_mut() else {
            return Err(DispatchFailure::internal("request serialization failed"));
        };
        object.remove("kind");

        let rpc_id = self.next_rpc_id.fetch_add(1, Ordering::SeqCst);
        let message = JsonRpcMessage::request(rpc_id, "request", params);
        let payload = serde_json::to_string(&message)
            .map_err(|_| DispatchFailure::internal("request serialization failed"))?;
        if let Err(error) = transport.send_frame(&encode_frame(&payload)).await {
            let _ = lifecycle.crash().await;
            let _ = lifecycle.complete_request(operation_key);
            return Err(DispatchFailure::closed(error));
        }

        let mut decoder = FrameDecoder::new();
        for _ in 0..MAX_FRAMES_PER_DISPATCH {
            let remaining = window.remaining();
            if remaining.is_zero() {
                let _ = lifecycle.timeout_request(operation_key).await;
                return Err(DispatchFailure {
                    outcome: ToolOutcome::Timeout,
                    detail: "execution window elapsed".to_string(),
                });
            }
            let frame = match tokio::time::timeout(remaining, transport.recv_frame()).await {
                Ok(Ok(Some(frame))) => frame,
                Ok(Ok(None)) => {
                    let _ = lifecycle.crash().await;
                    let _ = lifecycle.complete_request(operation_key);
                    return Err(DispatchFailure {
                        outcome: ToolOutcome::SandboxError,
                        detail: "worker channel closed during dispatch".to_string(),
                    });
                }
                Ok(Err(error)) => {
                    let _ = lifecycle.crash().await;
                    let _ = lifecycle.complete_request(operation_key);
                    return Err(DispatchFailure::closed(error));
                }
                Err(_) => {
                    let _ = lifecycle.timeout_request(operation_key).await;
                    return Err(DispatchFailure {
                        outcome: ToolOutcome::Timeout,
                        detail: "execution window elapsed".to_string(),
                    });
                }
            };
            if decoder.push(&frame).is_err() {
                let _ = lifecycle.crash().await;
                let _ = lifecycle.complete_request(operation_key);
                return Err(DispatchFailure {
                    outcome: ToolOutcome::SandboxError,
                    detail: "worker frame exceeded the bounded size".to_string(),
                });
            }
            while let Some(decoded) = decoder.pop_message() {
                let message = match decoded {
                    Ok(message) => message,
                    Err(_) => continue,
                };
                let reply_id = match &message {
                    JsonRpcMessage::Response { id, .. } => Some(*id),
                    JsonRpcMessage::Error { id, .. } => Some(*id),
                    _ => None,
                };
                if reply_id != Some(rpc_id) {
                    continue;
                }
                let _ = lifecycle.complete_request(operation_key);
                if window.cancellation().is_cancelled() {
                    let _ = lifecycle.cancel_request(operation_key).await;
                    return Err(DispatchFailure {
                        outcome: ToolOutcome::Cancelled,
                        detail: "cancelled while awaiting the worker".to_string(),
                    });
                }
                return match message {
                    JsonRpcMessage::Response { result, .. } => {
                        let response: WorkerMessage =
                            serde_json::from_value(result).map_err(|_| {
                                DispatchFailure::internal("worker reply is not a protocol message")
                            })?;
                        response.validate().map_err(|_| {
                            DispatchFailure::internal("worker reply violates the protocol")
                        })?;
                        match &response {
                            WorkerMessage::Response {
                                request_id: got,
                                context,
                                ..
                            } if *got == request_id && *context == expected_context => Ok(response),
                            _ => Err(DispatchFailure::internal(
                                "worker reply does not match the presented request",
                            )),
                        }
                    }
                    JsonRpcMessage::Error { error, .. } => Err(DispatchFailure {
                        outcome: ToolOutcome::Failed,
                        detail: bounded_detail(&error.message),
                    }),
                    _ => Err(DispatchFailure::internal("unexpected worker message")),
                };
            }
        }
        let _ = lifecycle.crash().await;
        let _ = lifecycle.complete_request(operation_key);
        Err(DispatchFailure {
            outcome: ToolOutcome::SandboxError,
            detail: "worker flooded the channel without a correlated reply".to_string(),
        })
    }
}

struct DispatchFailure {
    outcome: ToolOutcome,
    detail: String,
}

impl DispatchFailure {
    fn internal(detail: &str) -> Self {
        Self {
            outcome: ToolOutcome::Failed,
            detail: detail.to_string(),
        }
    }

    fn closed(_: WorkerTransportError) -> Self {
        Self {
            outcome: ToolOutcome::SandboxError,
            detail: "worker channel failed".to_string(),
        }
    }
}

fn lifecycle_state_detail(state: crate::python_lifecycle::LifecycleState) -> &'static str {
    match state {
        crate::python_lifecycle::LifecycleState::Stopped => "worker is stopped",
        crate::python_lifecycle::LifecycleState::Starting => "worker is starting",
        crate::python_lifecycle::LifecycleState::Busy => "worker is busy",
        crate::python_lifecycle::LifecycleState::Crashed => "worker crashed",
        crate::python_lifecycle::LifecycleState::TimedOut => "worker timed out",
        crate::python_lifecycle::LifecycleState::Cancelled => "worker was cancelled",
        crate::python_lifecycle::LifecycleState::Ready => "worker is ready",
    }
}

fn bounded_detail(detail: &str) -> String {
    detail.chars().take(256).collect()
}

/// Generates a fresh protocol request id (exposed for fixture workers).
pub fn new_request_id() -> RequestId {
    RequestId::from_uuid(Uuid::new_v4())
}
