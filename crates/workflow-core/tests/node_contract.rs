use agent_protocol::capability::{Action, Capability, Resource};
use serde_json::json;
use workflow_core::{
    WorkflowNode, WorkflowNodeError, WorkflowNodeType, WORKFLOW_NODE_SCHEMA_VERSION,
};

fn node(kind: WorkflowNodeType) -> WorkflowNode {
    WorkflowNode::new(
        "node-1".into(),
        "workflow-1".into(),
        3,
        kind,
        json!({"input": "bounded"}),
    )
    .unwrap()
}

#[test]
// @spec:AC-949 @spec:AC-951
fn all_initial_node_types_are_explicit_and_serializable() {
    let kinds = [
        WorkflowNodeType::Agent,
        WorkflowNodeType::Tool,
        WorkflowNodeType::Python,
        WorkflowNodeType::Condition,
        WorkflowNodeType::Parallel,
        WorkflowNodeType::Delay,
        WorkflowNodeType::Approval,
        WorkflowNodeType::SubWorkflow,
    ];

    for kind in kinds {
        let mut value = node(kind);
        value.required_capabilities = vec![Capability::new(Resource::Workflow, Action::Read)];
        let encoded = serde_json::to_value(&value).unwrap();
        assert_eq!(encoded["schema_version"], WORKFLOW_NODE_SCHEMA_VERSION);
        assert_eq!(encoded["type"], serde_json::to_value(kind).unwrap());
        assert!(serde_json::from_value::<WorkflowNode>(encoded).is_ok());
    }
}

#[test]
// @spec:AC-950
fn required_fields_and_bounds_fail_closed() {
    let mut value = node(WorkflowNodeType::Agent);
    value.node_id.clear();
    assert_eq!(value.validate(), Err(WorkflowNodeError::InvalidIdentity));

    let mut oversized = node(WorkflowNodeType::Tool);
    oversized.input_schema = json!("x".repeat(20_000));
    assert_eq!(
        oversized.validate(),
        Err(WorkflowNodeError::PayloadTooLarge)
    );

    let unknown = json!({
        "schema_version": 1,
        "node_id": "node-1",
        "workflow_id": "workflow-1",
        "workflow_version": 1,
        "type": "unknown_node",
        "input_schema": {},
        "output_schema": {},
        "timeout_ms": 1000,
        "retry": {"max_attempts": 1},
        "cancel": "cooperative",
        "required_capabilities": []
    });
    assert!(serde_json::from_value::<WorkflowNode>(unknown).is_err());
}

#[test]
// @spec:AC-951
fn schema_version_and_execution_policies_are_explicit() {
    let value = node(WorkflowNodeType::Condition);
    assert_eq!(value.schema_version, WORKFLOW_NODE_SCHEMA_VERSION);
    assert_eq!(value.workflow_version, 3);
    assert_eq!(value.timeout_ms, 30_000);
    assert_eq!(value.retry.max_attempts, 1);
    assert_eq!(value.cancel, workflow_core::CancelPolicy::Cooperative);
    assert!(value.required_capabilities.is_empty());
}
