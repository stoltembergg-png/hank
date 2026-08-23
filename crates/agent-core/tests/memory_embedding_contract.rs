use agent_core::{
    EmbeddingError, EmbeddingInput, EmbeddingPolicy, EmbeddingRequest, MockEmbeddingProvider,
    ProjectId,
};

fn request() -> EmbeddingRequest {
    EmbeddingRequest {
        project_id: Some(ProjectId::new()),
        trace_id: "trace-embed".into(),
        model: "mock-embed".into(),
        model_version: "1".into(),
        dimensions: 8,
        inputs: vec![EmbeddingInput {
            reference: "memory:m1".into(),
        }],
        budget_available: true,
        cancelled: false,
    }
}

// @spec:AC-757
#[test]
fn mock_embedding_returns_explicit_dimension_and_identity() {
    let response = MockEmbeddingProvider::embed(&request(), &EmbeddingPolicy::default()).unwrap();
    assert_eq!(response.model, "mock-embed");
    assert_eq!(response.model_version, "1");
    assert_eq!(response.vectors.len(), 1);
    assert_eq!(response.vectors[0].len(), 8);
}

// @spec:AC-758
#[test]
fn invalid_model_dimension_batch_project_or_budget_fails_closed() {
    let mut invalid = request();
    invalid.project_id = None;
    assert!(matches!(
        MockEmbeddingProvider::embed(&invalid, &EmbeddingPolicy::default()),
        Err(EmbeddingError::MissingProject)
    ));
    let mut mismatch = request();
    mismatch.dimensions = 0;
    assert!(matches!(
        MockEmbeddingProvider::embed(&mismatch, &EmbeddingPolicy::default()),
        Err(EmbeddingError::InvalidDimensions)
    ));
    let mut no_budget = request();
    no_budget.budget_available = false;
    assert!(matches!(
        MockEmbeddingProvider::embed(&no_budget, &EmbeddingPolicy::default()),
        Err(EmbeddingError::BudgetUnavailable)
    ));
}

// @spec:AC-759
#[test]
fn cancellation_closes_trace_without_vector_result() {
    let mut cancelled = request();
    cancelled.cancelled = true;
    assert!(matches!(
        MockEmbeddingProvider::embed(&cancelled, &EmbeddingPolicy::default()),
        Err(EmbeddingError::Cancelled)
    ));
}

// @spec:AC-760
#[test]
fn batch_and_reference_limits_are_bounded_without_raw_content() {
    let mut request = request();
    request.inputs = (0..129)
        .map(|i| EmbeddingInput {
            reference: format!("memory:{i}"),
        })
        .collect();
    assert!(matches!(
        MockEmbeddingProvider::embed(&request, &EmbeddingPolicy::default()),
        Err(EmbeddingError::BatchTooLarge)
    ));
}
