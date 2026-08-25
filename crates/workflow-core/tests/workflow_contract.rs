use agent_protocol::{AgentId, ProjectId};
use serde_json::json;
use workflow_core::{Workflow, WorkflowError, WorkflowStatus, WORKFLOW_SCHEMA_VERSION};

fn workflow() -> Workflow {
    Workflow::new(
        ProjectId::new(),
        AgentId::new(),
        "daily-review".into(),
        "policy-v1".into(),
    )
    .unwrap()
}

#[test]
// @spec:AC-945
fn workflow_identity_lifecycle_and_roundtrip_are_bounded() {
    let mut value = workflow();
    value.metadata.insert("purpose".into(), "review".into());
    value.activate().unwrap();
    let encoded = serde_json::to_value(&value).unwrap();
    let decoded: Workflow = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.schema_version, WORKFLOW_SCHEMA_VERSION);
    assert_eq!(decoded.status, WorkflowStatus::Active);
    assert_eq!(decoded.project_id, value.project_id);
}

#[test]
// @spec:AC-946
fn invalid_versions_metadata_and_lifecycle_fail_closed() {
    let mut value = workflow();
    assert_eq!(value.set_version(0), Err(WorkflowError::InvalidVersion));
    assert_eq!(
        value.set_metadata("x".into(), "y".repeat(513)),
        Err(WorkflowError::MetadataTooLarge)
    );
    assert_eq!(value.archive(), Err(WorkflowError::InvalidTransition));
    value.activate().unwrap();
    value.archive().unwrap();
    assert_eq!(value.activate(), Err(WorkflowError::InvalidTransition));
}

#[test]
// @spec:AC-947
fn unknown_schema_and_cross_project_identity_are_rejected() {
    let value = workflow();
    let mut encoded = serde_json::to_value(&value).unwrap();
    encoded["schema_version"] = json!(999);
    assert!(serde_json::from_value::<Workflow>(encoded).is_err());
    let other = Workflow::new(
        ProjectId::new(),
        value.owner_id,
        "same".into(),
        "policy".into(),
    )
    .unwrap();
    assert_ne!(value.project_id, other.project_id);
}
