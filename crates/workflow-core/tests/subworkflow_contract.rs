use std::collections::BTreeMap;
use workflow_core::subworkflow::{
    CompositionError, SubWorkflowCatalog, SubWorkflowPlan, SubWorkflowReference,
};

fn reference(project: &str, workflow: &str, version: &str) -> SubWorkflowReference {
    SubWorkflowReference::new(project, workflow, version).unwrap()
}

fn catalog() -> SubWorkflowCatalog {
    let mut catalog = SubWorkflowCatalog::new(4).unwrap();
    catalog
        .register(reference("project-a", "child", "1.2.0"))
        .unwrap();
    catalog
}

// @spec:AC-1021
#[test]
fn valid_reference_mapping_and_correlation_are_deterministic() {
    let catalog = catalog();
    let mut mapping = BTreeMap::new();
    mapping.insert("child_input".into(), "parent_output".into());
    let plan = SubWorkflowPlan::new(
        "project-a",
        "parent",
        "run-1",
        "node-sub",
        3,
        reference("project-a", "child", "1.2.0"),
        mapping,
        0,
        4,
        10,
        100,
    )
    .unwrap();
    let first = plan
        .resolve(
            &catalog,
            false,
            &BTreeMap::from([("parent_output".into(), serde_json::json!(7))]),
        )
        .unwrap();
    let second = plan
        .resolve(
            &catalog,
            false,
            &BTreeMap::from([("parent_output".into(), serde_json::json!(7))]),
        )
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.child_inputs["child_input"], serde_json::json!(7));
}

// @spec:AC-1022
#[test]
fn missing_mapping_scope_cycle_depth_and_budget_fail_closed() {
    let catalog = catalog();
    let missing = SubWorkflowPlan::new(
        "project-a",
        "parent",
        "run-1",
        "node",
        1,
        reference("project-a", "missing", "1.0.0"),
        BTreeMap::new(),
        0,
        4,
        1,
        10,
    )
    .unwrap();
    assert!(matches!(
        missing.resolve(&catalog, false, &BTreeMap::new()),
        Err(CompositionError::VersionNotFound)
    ));
    let cross = SubWorkflowPlan::new(
        "project-b",
        "parent",
        "run-1",
        "node",
        1,
        reference("project-a", "child", "1.2.0"),
        BTreeMap::new(),
        0,
        4,
        1,
        10,
    )
    .unwrap();
    assert!(matches!(
        cross.resolve(&catalog, false, &BTreeMap::new()),
        Err(CompositionError::CrossProjectDenied)
    ));
    let deep = SubWorkflowPlan::new(
        "project-a",
        "parent",
        "run-1",
        "node",
        1,
        reference("project-a", "child", "1.2.0"),
        BTreeMap::new(),
        4,
        4,
        1,
        10,
    )
    .unwrap();
    assert!(matches!(
        deep.resolve(&catalog, false, &BTreeMap::new()),
        Err(CompositionError::DepthLimit)
    ));
    let budget = SubWorkflowPlan::new(
        "project-a",
        "parent",
        "run-1",
        "node",
        1,
        reference("project-a", "child", "1.2.0"),
        BTreeMap::new(),
        0,
        4,
        11,
        10,
    )
    .unwrap();
    assert!(matches!(
        budget.resolve(&catalog, false, &BTreeMap::new()),
        Err(CompositionError::BudgetExceeded)
    ));
    let cycle = SubWorkflowPlan::new(
        "project-a",
        "child",
        "run-1",
        "node",
        1,
        reference("project-a", "child", "1.2.0"),
        BTreeMap::new(),
        0,
        4,
        1,
        10,
    )
    .unwrap();
    assert!(matches!(
        cycle.resolve(&catalog, false, &BTreeMap::new()),
        Err(CompositionError::CycleDetected)
    ));
}

// @spec:AC-1023
#[test]
fn cancellation_is_terminal_and_replanning_is_idempotent() {
    let catalog = catalog();
    let plan = SubWorkflowPlan::new(
        "project-a",
        "parent",
        "run-1",
        "node-sub",
        3,
        reference("project-a", "child", "1.2.0"),
        BTreeMap::new(),
        0,
        4,
        1,
        10,
    )
    .unwrap();
    let mut child = plan.resolve(&catalog, false, &BTreeMap::new()).unwrap();
    assert_eq!(
        child,
        plan.resolve(&catalog, false, &BTreeMap::new()).unwrap()
    );
    assert!(child.cancel());
    assert!(!child.cancel());
    assert!(child.is_cancelled());
}
