use agent_runtime::workflow_logs::{
    EventKind, LogError, RetentionClass, Severity, WorkflowLogEvent, WorkflowLogStore,
};
use std::collections::BTreeMap;

fn event(project: &str, run: &str, id: &str, timestamp_ms: u64) -> WorkflowLogEvent {
    WorkflowLogEvent::new(
        project,
        run,
        "node-1",
        id,
        EventKind::Transition,
        Severity::Info,
        RetentionClass::Short,
        timestamp_ms,
    )
    .unwrap()
}

// @spec:AC-1061
#[test]
fn allowlist_and_golden_redaction_remove_sensitive_values() {
    let store = WorkflowLogStore::new(8, 4096);
    let mut item = event("project-1", "run-1", "event-1", 1);
    let mut fields = BTreeMap::new();
    fields.insert("status".into(), "running".into());
    fields.insert("token".into(), "secret-token-value".into());
    fields.insert("url".into(), "https://example.invalid/page".into());
    fields.insert("path".into(), "/home/user/prompt.txt".into());
    fields.insert("message".into(), "page content".into());
    item.fields = fields;
    store.append(item).unwrap();
    let exported = store.export("project-1", "run-1", 8).unwrap();
    assert!(exported.contains("running"));
    assert!(!exported.contains("secret-token-value"));
    assert!(!exported.contains("https://"));
    assert!(!exported.contains("/home/"));
    assert!(!exported.contains("page content"));
}

// @spec:AC-1062
#[test]
fn correlation_order_duplicate_and_project_scope_are_fail_closed() {
    let store = WorkflowLogStore::new(8, 4096);
    store
        .append(event("project-1", "run-1", "event-1", 2))
        .unwrap();
    assert_eq!(
        store.append(event("project-1", "run-1", "event-1", 3)),
        Err(LogError::Duplicate)
    );
    assert_eq!(
        store.append(event("project-1", "run-1", "event-2", 1)),
        Err(LogError::OutOfOrder)
    );
    store
        .append(event("project-2", "run-1", "event-3", 1))
        .unwrap();
    assert_eq!(store.query("project-1", "run-1", 8).unwrap().len(), 1);
}

// @spec:AC-1063
#[test]
fn retention_export_and_metrics_are_bounded() {
    let store = WorkflowLogStore::new(2, 120);
    for (id, time) in [("event-1", 1), ("event-2", 2), ("event-3", 3)] {
        store.append(event("project-1", "run-1", id, time)).unwrap();
    }
    assert_eq!(store.query("project-1", "run-1", 2).unwrap().len(), 2);
    assert!(store.export("project-1", "run-1", 2).unwrap().len() <= 120);
    assert_eq!(store.metrics().dropped, 1);
}
