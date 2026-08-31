use agent_core::budget::{BudgetLimits, ReservationId};
use agent_core::ids::{AgentId, ProjectId};
use agent_protocol::ids::{OperationKey, TraceId};
use async_trait::async_trait;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tool_core::{
    PolicyDecision, Tool, ToolContext, ToolEnvironment, ToolError, ToolOutcome, ToolPluginAdapter,
    ToolRequest, ToolResponse, ToolSchema,
};

fn schema() -> &'static ToolSchema {
    Box::leak(Box::new(ToolSchema {
        name: "plugin_tool".into(),
        version: "1.0.0".into(),
        description: Some("fixture".into()),
        input_schema: json!({"type": "object"}),
        output_schema: json!({"type": "object"}),
        capabilities: vec!["plugin:read".into()],
        destructive: false,
        environment: ToolEnvironment::Sandbox,
        timeout_seconds: 10,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        metadata: BTreeMap::new(),
    }))
}

struct FixtureTool;

#[async_trait]
impl Tool for FixtureTool {
    fn schema(&self) -> &'static ToolSchema {
        schema()
    }
    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        self.can_handle(&request)?;
        Ok(ToolResponse {
            operation_key: request.operation_key,
            tool_name: request.tool_name,
            tool_version: request.tool_version,
            outcome: ToolOutcome::Success,
            payload: json!({"ok": true}),
            trace_id: request.context.trace_id,
            duration_ms: 1,
            metadata: BTreeMap::new(),
        })
    }
}

fn request(policy: PolicyDecision) -> ToolRequest {
    ToolRequest {
        operation_key: OperationKey::new(),
        tool_name: "plugin_tool".into(),
        tool_version: "1.0.0".into(),
        input: json!({}),
        context: ToolContext {
            project_id: ProjectId::new(),
            agent_id: Some(AgentId::new()),
            session_id: None,
            task_id: None,
            workflow_id: None,
            capability: "plugin:read".into(),
            policy_decision: policy,
            budget_limits: BudgetLimits::default(),
            reservation_id: Some(ReservationId::new()),
            trace_id: TraceId::new(),
            metadata: BTreeMap::new(),
        },
        timeout_seconds: Some(10),
        metadata: BTreeMap::new(),
    }
}

#[tokio::test]
// @spec:AC-1399
async fn approved_tool_plugin_delegates_bounded_call() {
    let adapter =
        ToolPluginAdapter::new(Arc::new(FixtureTool), "plugin-a", "digest-1", true).unwrap();
    let response = adapter
        .execute(request(PolicyDecision::Allow))
        .await
        .unwrap();
    assert_eq!(response.outcome, ToolOutcome::Success);
    assert_eq!(adapter.plugin_id(), "plugin-a");
}

#[tokio::test]
// @spec:AC-1400
async fn unapproved_or_denied_tool_plugin_fails_before_delegation() {
    let adapter =
        ToolPluginAdapter::new(Arc::new(FixtureTool), "plugin-a", "digest-1", false).unwrap();
    assert!(matches!(
        adapter.execute(request(PolicyDecision::Allow)).await,
        Err(ToolError::PermissionDenied { .. })
    ));
    let approved =
        ToolPluginAdapter::new(Arc::new(FixtureTool), "plugin-a", "digest-1", true).unwrap();
    assert!(matches!(
        approved.execute(request(PolicyDecision::Deny)).await,
        Err(ToolError::PermissionDenied { .. })
    ));
}
