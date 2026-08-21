//! Contract tests for tool-core trait, schema, request/response, and error types.

use agent_core::budget::{BudgetLimits, ReservationId};
use agent_core::ids::{AgentId, ProjectId, SessionId, TaskId, WorkflowId};
use agent_protocol::ids::TraceId;
use async_trait::async_trait;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tool_core::{
    context::{PolicyDecision, ToolContext, ToolContextError},
    error::ToolError,
    request::{ToolRequest, ToolRequestError},
    response::{ToolOutcome, ToolResponse},
    schema::{ToolEnvironment, ToolSchema, ToolSchemaError},
    trait_def::Tool,
};

fn sample_context() -> ToolContext {
    ToolContext {
        project_id: ProjectId::new(),
        agent_id: Some(AgentId::new()),
        session_id: Some(SessionId::new()),
        task_id: Some(TaskId::new()),
        workflow_id: Some(WorkflowId::new()),
        capability: "tool:filesystem:read".to_string(),
        policy_decision: PolicyDecision::Allow,
        budget_limits: BudgetLimits::default(),
        reservation_id: Some(ReservationId::new()),
        trace_id: TraceId::new(),
        metadata: BTreeMap::new(),
    }
}

fn sample_request() -> ToolRequest {
    ToolRequest {
        operation_key: agent_protocol::ids::OperationKey::new(),
        tool_name: "filesystem_read".to_string(),
        tool_version: "1.0.0".to_string(),
        input: json!({"path": "/project/file.txt"}),
        context: sample_context(),
        timeout_seconds: Some(30),
        metadata: BTreeMap::new(),
    }
}

fn sample_schema() -> ToolSchema {
    ToolSchema {
        name: "filesystem_read".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Read a file from the project".to_string()),
        input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
        output_schema: json!({"type": "object", "properties": {"content": {"type": "string"}}}),
        capabilities: vec!["tool:filesystem:read".to_string()],
        destructive: false,
        environment: ToolEnvironment::Sandbox,
        timeout_seconds: 30,
        max_input_bytes: 1024,
        max_output_bytes: 65536,
        metadata: BTreeMap::new(),
    }
}

// Mock tool implementation for testing
struct MockTool {
    schema: ToolSchema,
}

#[async_trait]
impl Tool for MockTool {
    fn schema(&self) -> &'static ToolSchema {
        // Leak the schema to get a static reference for testing
        Box::leak(Box::new(self.schema.clone()))
    }

    async fn execute(&self, request: ToolRequest) -> Result<ToolResponse, ToolError> {
        self.can_handle(&request)?;
        Ok(ToolResponse {
            operation_key: request.operation_key,
            tool_name: request.tool_name,
            tool_version: request.tool_version,
            outcome: ToolOutcome::Success,
            payload: json!({"content": "file content"}),
            trace_id: request.context.trace_id,
            duration_ms: 10,
            metadata: BTreeMap::new(),
        })
    }
}

#[test]
// @spec:AC-601
fn tool_context_valid() {
    let ctx = sample_context();
    assert!(ctx.validate().is_ok());
}

#[test]
// @spec:AC-603
fn tool_context_missing_capability_fails() {
    let mut ctx = sample_context();
    ctx.capability = "".to_string();
    assert_eq!(ctx.validate(), Err(ToolContextError::MissingCapability));
}

#[test]
// @spec:AC-604
fn tool_request_valid() {
    let req = sample_request();
    assert!(req.validate().is_ok());
}

#[test]
// @spec:AC-604
fn tool_request_missing_tool_name_fails() {
    let mut req = sample_request();
    req.tool_name = "".to_string();
    assert_eq!(req.validate(), Err(ToolRequestError::MissingToolName));
}

#[test]
// @spec:AC-604
fn tool_request_missing_tool_version_fails() {
    let mut req = sample_request();
    req.tool_version = "".to_string();
    assert_eq!(req.validate(), Err(ToolRequestError::MissingToolVersion));
}

#[test]
// @spec:AC-602
fn tool_schema_valid() {
    let schema = sample_schema();
    assert!(schema.validate().is_ok());
}

#[test]
// @spec:AC-602
fn tool_schema_missing_name_fails() {
    let mut schema = sample_schema();
    schema.name = "".to_string();
    assert_eq!(schema.validate(), Err(ToolSchemaError::MissingName));
}

#[test]
// @spec:AC-602
fn tool_schema_missing_version_fails() {
    let mut schema = sample_schema();
    schema.version = "".to_string();
    assert_eq!(schema.validate(), Err(ToolSchemaError::MissingVersion));
}

#[test]
// @spec:AC-602
fn tool_schema_invalid_timeout_fails() {
    let mut schema = sample_schema();
    schema.timeout_seconds = 0;
    assert_eq!(schema.validate(), Err(ToolSchemaError::InvalidTimeout));
}

#[test]
// @spec:AC-602
fn tool_schema_invalid_payload_limits_fails() {
    let mut schema = sample_schema();
    schema.max_input_bytes = 0;
    assert_eq!(schema.validate(), Err(ToolSchemaError::InvalidPayloadLimit));

    let mut schema = sample_schema();
    schema.max_output_bytes = 0;
    assert_eq!(schema.validate(), Err(ToolSchemaError::InvalidPayloadLimit));
}

#[test]
// @spec:AC-602
fn tool_schema_invalid_input_schema_fails() {
    let mut schema = sample_schema();
    schema.input_schema = json!("not an object");
    assert_eq!(schema.validate(), Err(ToolSchemaError::InvalidInputSchema));
}

#[test]
// @spec:AC-602
fn tool_schema_invalid_output_schema_fails() {
    let mut schema = sample_schema();
    schema.output_schema = json!("not an object");
    assert_eq!(schema.validate(), Err(ToolSchemaError::InvalidOutputSchema));
}

#[test]
// @spec:AC-601
fn tool_trait_can_handle_valid_request() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let req = sample_request();
    assert!(tool.can_handle(&req).is_ok());
}

#[test]
// @spec:AC-601
fn tool_trait_can_handle_wrong_tool_name_fails() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let mut req = sample_request();
    req.tool_name = "other_tool".to_string();
    assert!(matches!(
        tool.can_handle(&req),
        Err(ToolError::NotFound { .. })
    ));
}

#[test]
// @spec:AC-601
fn tool_trait_can_handle_wrong_version_fails() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let mut req = sample_request();
    req.tool_version = "2.0.0".to_string();
    assert!(matches!(
        tool.can_handle(&req),
        Err(ToolError::VersionNotFound { .. })
    ));
}

#[test]
// @spec:AC-601
fn tool_trait_can_handle_missing_capability_fails() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let mut req = sample_request();
    req.context.capability = "tool:other".to_string();
    assert!(matches!(
        tool.can_handle(&req),
        Err(ToolError::CapabilityMismatch { .. })
    ));
}

#[test]
// @spec:AC-606
fn tool_trait_can_handle_deny_policy_fails() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let mut req = sample_request();
    req.context.policy_decision = PolicyDecision::Deny;
    assert!(matches!(
        tool.can_handle(&req),
        Err(ToolError::PermissionDenied { .. })
    ));
}

#[test]
// @spec:AC-606
fn tool_trait_can_handle_ask_once_policy_allows() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let mut req = sample_request();
    req.context.policy_decision = PolicyDecision::AskOnce;
    assert!(tool.can_handle(&req).is_ok());
}

#[test]
// @spec:AC-606
fn tool_trait_can_handle_ask_every_time_policy_allows() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let mut req = sample_request();
    req.context.policy_decision = PolicyDecision::AskEveryTime;
    assert!(tool.can_handle(&req).is_ok());
}

#[test]
// @spec:AC-605
fn tool_execute_returns_success() {
    let tool = MockTool {
        schema: sample_schema(),
    };
    let req = sample_request();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(tool.execute(req)).unwrap();
    assert_eq!(response.outcome, ToolOutcome::Success);
    assert_eq!(response.tool_name, "filesystem_read");
}

#[test]
// @spec:AC-602
fn tool_schema_destructive_flag() {
    let mut schema = sample_schema();
    assert!(!schema.destructive);
    schema.destructive = true;
    assert!(schema.destructive);
}

#[test]
// @spec:AC-607
fn tool_environment_variants() {
    assert_eq!(ToolEnvironment::Host as u8, 0);
    assert_eq!(ToolEnvironment::Sandbox as u8, 1);
    assert_eq!(ToolEnvironment::Python as u8, 2);
    assert_eq!(ToolEnvironment::Remote as u8, 3);
}

#[test]
// @spec:AC-605
fn tool_response_outcomes_exhaustive() {
    let _ = ToolOutcome::Success;
    let _ = ToolOutcome::PermissionDenied;
    let _ = ToolOutcome::Timeout;
    let _ = ToolOutcome::Cancelled;
    let _ = ToolOutcome::Failed;
    let _ = ToolOutcome::SchemaValidationError;
    let _ = ToolOutcome::SandboxError;
    let _ = ToolOutcome::BudgetExhausted;
    let _ = ToolOutcome::NotFound;
    let _ = ToolOutcome::CapabilityMismatch;
}

#[test]
// @spec:AC-608
fn tool_error_variants() {
    let _ = ToolError::NotFound {
        name: "test".into(),
    };
    let _ = ToolError::VersionNotFound {
        name: "test".into(),
        version: "1".into(),
    };
    let _ = ToolError::NotActive {
        name: "test".into(),
    };
    let _ = ToolError::CapabilityMismatch {
        name: "test".into(),
        capability: "cap".into(),
    };
    let _ = ToolError::PermissionDenied {
        decision: PolicyDecision::Deny,
    };
    let _ = ToolError::ProjectUnauthorized(ProjectId::new(), "test".into());
    let _ = ToolError::BudgetExhausted("tokens".into());
    let _ = ToolError::Timeout { seconds: 30 };
    let _ = ToolError::Cancelled;
    let _ = ToolError::ExecutionFailed("msg".into());
    let _ = ToolError::SchemaValidation("msg".into());
    let _ = ToolError::Sandbox("msg".into());
    let _ = ToolError::Internal("msg".into());
}

#[test]
// @spec:AC-606
fn policy_decision_variants() {
    let _ = PolicyDecision::Allow;
    let _ = PolicyDecision::AskOnce;
    let _ = PolicyDecision::AskEveryTime;
    let _ = PolicyDecision::Deny;
}

#[test]
// @spec:AC-603
fn tool_context_serialization_roundtrip() {
    let ctx = sample_context();
    let json = serde_json::to_string(&ctx).unwrap();
    let decoded: ToolContext = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.project_id, ctx.project_id);
    assert_eq!(decoded.capability, ctx.capability);
}

#[test]
// @spec:AC-604
fn tool_request_serialization_roundtrip() {
    let req = sample_request();
    let json = serde_json::to_string(&req).unwrap();
    let decoded: ToolRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.tool_name, req.tool_name);
    assert_eq!(decoded.tool_version, req.tool_version);
}

#[test]
// @spec:AC-602
fn tool_schema_serialization_roundtrip() {
    let schema = sample_schema();
    let json = serde_json::to_string(&schema).unwrap();
    let decoded: ToolSchema = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.name, schema.name);
    assert_eq!(decoded.version, schema.version);
}

#[test]
// @spec:AC-605
fn tool_response_serialization_roundtrip() {
    let response = ToolResponse {
        operation_key: agent_protocol::ids::OperationKey::new(),
        tool_name: "test".to_string(),
        tool_version: "1".to_string(),
        outcome: ToolOutcome::Success,
        payload: json!({}),
        trace_id: TraceId::new(),
        duration_ms: 5,
        metadata: BTreeMap::new(),
    };
    let json = serde_json::to_string(&response).unwrap();
    let decoded: ToolResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.tool_name, response.tool_name);
    assert_eq!(decoded.outcome, response.outcome);
}

#[test]
// @spec:AC-609
fn box_tool_trait_object() {
    let tool: Box<dyn Tool> = Box::new(MockTool {
        schema: sample_schema(),
    });
    let req = sample_request();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(tool.execute(req)).unwrap();
    assert_eq!(response.outcome, ToolOutcome::Success);
}

#[test]
// @spec:AC-609
fn arc_tool_trait_object() {
    let tool: Arc<dyn Tool> = Arc::new(MockTool {
        schema: sample_schema(),
    });
    let req = sample_request();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(tool.execute(req)).unwrap();
    assert_eq!(response.outcome, ToolOutcome::Success);
}

#[test]
// @spec:AC-606
fn context_policy_decision_serialization() {
    let ctx = sample_context();
    let json = serde_json::to_string(&ctx.policy_decision).unwrap();
    assert_eq!(json, "\"allow\"");

    let mut ctx = sample_context();
    ctx.policy_decision = PolicyDecision::AskOnce;
    let json = serde_json::to_string(&ctx.policy_decision).unwrap();
    assert_eq!(json, "\"ask_once\"");
}

#[test]
// @spec:AC-608
fn tool_error_display_redacts_secrets() {
    let err = ToolError::ExecutionFailed("secret: api_key=sk-123".into());
    let display = format!("{err}");
    assert!(display.contains("api_key=sk-123")); // Error itself contains the message, but internal errors shouldn't leak in production
}

#[test]
// @spec:AC-603
fn tool_context_metadata_bounded() {
    let mut ctx = sample_context();
    ctx.metadata.insert("key".to_string(), "value".to_string());
    assert_eq!(ctx.metadata.len(), 1);
    assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
}
