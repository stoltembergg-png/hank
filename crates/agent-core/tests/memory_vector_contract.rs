use agent_core::{MemoryKind, ProjectId, VectorError, VectorIndex, VectorQuery, VectorRecord};

fn record(project: ProjectId, id: &str, vector: Vec<f32>) -> VectorRecord {
    VectorRecord {
        id: id.into(),
        project_id: project,
        agent_id: None,
        kind: MemoryKind::Fact,
        model: "mock-embed".into(),
        model_version: "1".into(),
        vector,
        content_ref: format!("memory:{id}"),
        active: true,
    }
}

fn query(project: ProjectId, vector: Vec<f32>) -> VectorQuery {
    VectorQuery {
        project_id: project,
        agent_id: None,
        model: "mock-embed".into(),
        model_version: "1".into(),
        vector,
        k: 5,
        max_bytes: 4096,
        trace_id: "trace-vector".into(),
    }
}

// @spec:AC-761
#[test]
fn vector_query_is_scoped_ranked_and_dimension_checked() {
    let project = ProjectId::new();
    let mut index = VectorIndex::default();
    index.upsert(record(project, "m1", vec![1.0, 0.0])).unwrap();
    index.upsert(record(project, "m2", vec![0.9, 0.1])).unwrap();
    index
        .upsert(record(ProjectId::new(), "m3", vec![1.0, 0.0]))
        .unwrap();
    let results = index.query(&query(project, vec![1.0, 0.0])).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "m1");
    assert!(matches!(
        index.query(&query(project, vec![1.0])),
        Err(VectorError::DimensionMismatch)
    ));
}

// @spec:AC-762
// @spec:AC-779
#[test]
fn upsert_is_idempotent_and_archive_delete_remove_active_index() {
    let project = ProjectId::new();
    let mut index = VectorIndex::default();
    index.upsert(record(project, "m1", vec![1.0, 0.0])).unwrap();
    index.upsert(record(project, "m1", vec![0.0, 1.0])).unwrap();
    assert_eq!(
        index.query(&query(project, vec![0.0, 1.0])).unwrap()[0].id,
        "m1"
    );
    index.archive(&project, "m1").unwrap();
    assert!(index
        .query(&query(project, vec![0.0, 1.0]))
        .unwrap()
        .is_empty());

    let foreign = ProjectId::new();
    index
        .upsert(record(foreign, "foreign", vec![1.0, 0.0]))
        .unwrap();
    assert!(matches!(
        index.archive(&project, "foreign"),
        Err(VectorError::ProjectScope)
    ));
    assert_eq!(
        index.query(&query(foreign, vec![1.0, 0.0])).unwrap()[0].id,
        "foreign"
    );
}

// @spec:AC-763
#[test]
fn k_and_bytes_limits_fail_closed_or_truncate_whole_records() {
    let project = ProjectId::new();
    let mut index = VectorIndex::default();
    index.upsert(record(project, "m1", vec![1.0, 0.0])).unwrap();
    let mut q = query(project, vec![1.0, 0.0]);
    q.k = 0;
    assert!(matches!(index.query(&q), Err(VectorError::InvalidQuery)));
    q.k = 1;
    q.max_bytes = 1;
    assert!(index.query(&q).unwrap().is_empty());
}

// @spec:AC-764
#[test]
fn rebuild_failure_rolls_back_previous_index() {
    let project = ProjectId::new();
    let mut index = VectorIndex::default();
    index.upsert(record(project, "m1", vec![1.0, 0.0])).unwrap();
    let result = index.rebuild(vec![record(project, "bad", vec![1.0])]);
    assert!(result.is_err());
    assert_eq!(
        index.query(&query(project, vec![1.0, 0.0])).unwrap()[0].id,
        "m1"
    );
}
