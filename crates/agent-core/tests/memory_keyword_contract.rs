use agent_core::{
    KeywordMemoryRecord, KeywordQuery, KeywordRetriever, MemoryKind, MemoryStatus, ProjectId,
};

fn record(project: ProjectId, id: &str, content: &str, importance: f32) -> KeywordMemoryRecord {
    KeywordMemoryRecord {
        id: id.into(),
        project_id: project,
        agent_id: None,
        kind: MemoryKind::Fact,
        status: MemoryStatus::Approved,
        content: content.into(),
        importance,
    }
}

fn query(project: ProjectId, terms: &str) -> KeywordQuery {
    KeywordQuery {
        project_id: project,
        agent_id: None,
        terms: terms.into(),
        max_results: 10,
        max_bytes: 4096,
        trace_id: "trace-1".into(),
    }
}

// @spec:AC-753
#[test]
fn keyword_retrieval_is_scoped_filtered_and_deterministic() {
    let project = ProjectId::new();
    let mut retriever = KeywordRetriever::default();
    retriever
        .insert(record(project, "m1", "Rust architecture", 0.7))
        .unwrap();
    retriever
        .insert(record(project, "m2", "Rust architecture details", 0.9))
        .unwrap();
    retriever
        .insert(record(ProjectId::new(), "m3", "Rust architecture", 1.0))
        .unwrap();
    let results = retriever
        .query(&query(project, " RUST   architecture "))
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "m2");
}

// @spec:AC-754
#[test]
fn archived_records_and_oversized_terms_are_blocked() {
    let project = ProjectId::new();
    let mut retriever = KeywordRetriever::default();
    let mut archived = record(project, "m1", "old fact", 1.0);
    archived.status = MemoryStatus::Archived;
    retriever.insert(archived).unwrap();
    assert!(retriever.query(&query(project, "old")).unwrap().is_empty());
    assert!(retriever.query(&query(project, &"x".repeat(4097))).is_err());
}

// @spec:AC-755
#[test]
fn duplicate_results_are_removed_and_budget_truncates_data() {
    let project = ProjectId::new();
    let mut retriever = KeywordRetriever::default();
    retriever
        .insert(record(project, "m1", "same text", 0.5))
        .unwrap();
    assert!(retriever
        .insert(record(project, "m1", "same text", 0.5))
        .is_err());
    let mut q = query(project, "same");
    q.max_bytes = 4;
    assert_eq!(retriever.query(&q).unwrap().len(), 0);
}

// @spec:AC-756
#[test]
fn missing_identity_or_trace_fails_closed() {
    let project = ProjectId::new();
    let retriever = KeywordRetriever::default();
    let mut q = query(project, "fact");
    q.trace_id.clear();
    assert!(retriever.query(&q).is_err());
}
