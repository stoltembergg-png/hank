use agent_core::budget::BudgetLimits;
use agent_core::ids::ProjectId;
use agent_protocol::ids::{OperationKey, TraceId};
use agent_runtime::tool_node::{ToolNodeAdapter, ToolNodeError, ToolNodeRequest};
use async_trait::async_trait;
use provider_core::CancellationToken;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tool_core::registry::{ToolOrigin, ToolRegistrationRequest, ToolRegistry, ToolScope};
use tool_core::{
    PermissionEvaluator, PermissionRequest, PolicyDecision, Tool, ToolContext, ToolEffect,
    ToolEnvironment, ToolError, ToolOutcome, ToolRequest, ToolResponse, ToolSchema,
};

struct MockTool {
    calls: Arc<AtomicUsize>,
    schema: ToolSchema,
}
impl MockTool {
    fn new(calls: Arc<AtomicUsize>) -> Arc<Self> {
        Arc::new(Self {
            calls,
            schema: ToolSchema {
                name: "read_tool".into(),
                version: "1.0.0".into(),
                description: None,
                input_schema: json!({"type":"object","properties":{"value":{"type":"string"}},"required":["value"],"additionalProperties":false}),
                output_schema: json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}),
                capabilities: vec!["tool:test:read".into()],
                destructive: false,
                environment: ToolEnvironment::Sandbox,
                timeout_seconds: 2,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                metadata: BTreeMap::new(),
            },
        })
    }
}
#[async_trait]
impl Tool for MockTool {
    fn schema(&self) -> &'static ToolSchema {
        Box::leak(Box::new(self.schema.clone()))
    }
    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ToolResponse {
            operation_key: request.operation_key,
            tool_name: request.tool_name,
            tool_version: request.tool_version,
            outcome: ToolOutcome::Success,
            payload: json!({"ok":true}),
            trace_id: request.context.trace_id,
            duration_ms: 1,
            metadata: BTreeMap::new(),
        })
    }
}

fn fixture() -> (
    ProjectId,
    ToolRequest,
    PermissionRequest,
    Arc<AtomicUsize>,
    Arc<ToolRegistry>,
    Arc<PermissionEvaluator>,
) {
    let project = ProjectId::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let tool = MockTool::new(calls.clone());
    let registry = Arc::new(ToolRegistry::new());
    registry
        .register(ToolRegistrationRequest::new(
            tool,
            ToolOrigin::Builtin,
            ToolScope::Global,
            TraceId::new(),
        ))
        .unwrap();
    let operation_key = OperationKey::new();
    let trace_id = TraceId::new();
    let context = ToolContext {
        project_id: project,
        agent_id: None,
        session_id: None,
        task_id: None,
        workflow_id: None,
        capability: "tool:test:read".into(),
        policy_decision: PolicyDecision::Allow,
        budget_limits: BudgetLimits::default(),
        reservation_id: None,
        trace_id,
        metadata: BTreeMap::new(),
    };
    let request = ToolRequest {
        operation_key,
        tool_name: "read_tool".into(),
        tool_version: "1.0.0".into(),
        input: json!({"value":"hello"}),
        context,
        timeout_seconds: Some(1),
        metadata: BTreeMap::new(),
    };
    let permission = PermissionRequest {
        project_id: Some(project),
        tool_name: "read_tool".into(),
        tool_version: "1.0.0".into(),
        capability: "tool:test:read".into(),
        effect: ToolEffect::Read,
        policy: PolicyDecision::Allow,
        budget_available: true,
        confirmation_approved: false,
    };
    (
        project,
        request,
        permission,
        calls,
        registry,
        Arc::new(PermissionEvaluator::new()),
    )
}

// @spec:AC-976
#[tokio::test]
async fn gates_registry_schema_and_permission_before_handler() {
    let (_project, request, permission, calls, registry, evaluator) = fixture();
    let adapter = ToolNodeAdapter::new(registry, evaluator);
    let result = adapter
        .execute(ToolNodeRequest {
            request: request.clone(),
            permission: permission.clone(),
            cancellation: CancellationToken::new(),
        })
        .await
        .unwrap();
    assert_eq!(result.outcome, ToolOutcome::Success);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let mut denied = permission.clone();
    denied.policy = PolicyDecision::Deny;
    let mut denied_request = request.clone();
    denied_request.operation_key = OperationKey::new();
    assert!(matches!(
        adapter
            .execute(ToolNodeRequest {
                request: denied_request,
                permission: denied,
                cancellation: CancellationToken::new()
            })
            .await,
        Err(ToolNodeError::PermissionDenied)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let mut oversized = request;
    oversized.operation_key = OperationKey::new();
    oversized.input = json!({ "value": "x".repeat(2_000) });
    assert!(matches!(
        adapter
            .execute(ToolNodeRequest {
                request: oversized,
                permission,
                cancellation: CancellationToken::new()
            })
            .await,
        Err(ToolNodeError::SchemaInvalid)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

// @spec:AC-977
#[tokio::test]
async fn cancelled_request_fails_without_handler_execution() {
    let (_project, request, permission, calls, registry, evaluator) = fixture();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let adapter = ToolNodeAdapter::new(registry, evaluator);
    assert!(matches!(
        adapter
            .execute(ToolNodeRequest {
                request: request.clone(),
                permission: permission.clone(),
                cancellation
            })
            .await,
        Err(ToolNodeError::Cancelled)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

// @spec:AC-978
#[tokio::test]
async fn duplicate_operation_returns_cached_response_without_second_call() {
    let (_project, request, permission, calls, registry, evaluator) = fixture();
    let adapter = ToolNodeAdapter::new(registry, evaluator);
    let cancellation = CancellationToken::new();
    let first = adapter
        .execute(ToolNodeRequest {
            request: request.clone(),
            permission: permission.clone(),
            cancellation: cancellation.clone(),
        })
        .await
        .unwrap();
    let second = adapter
        .execute(ToolNodeRequest {
            request,
            permission,
            cancellation,
        })
        .await
        .unwrap();
    assert_eq!(first.payload, second.payload);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
