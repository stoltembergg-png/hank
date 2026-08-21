use agent_core::ids::{AgentId, ProjectId, SessionId};
use agent_runtime::usage::{
    UsageAggregator, UsageConfidence, UsageError, UsageEvent, UsageOutcome, UsageRecordResult,
    UsageSource, USAGE_SCHEMA_VERSION,
};
use provider_core::{ModelId, ProviderId};

fn event(
    attempt_id: &str,
    project_id: ProjectId,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cost_micros: Option<u64>,
    source: UsageSource,
    confidence: UsageConfidence,
) -> UsageEvent {
    UsageEvent {
        schema_version: USAGE_SCHEMA_VERSION,
        attempt_id: attempt_id.into(),
        execution_id: "exec-1".into(),
        project_id,
        agent_id: AgentId::new(),
        session_id: SessionId::new(),
        provider_id: Some(ProviderId::parse("provider-one").unwrap()),
        model_id: Some(ModelId::parse("model-one").unwrap()),
        input_tokens,
        output_tokens,
        cost_micros,
        currency: cost_micros.map(|_| "USD".into()),
        source,
        confidence,
        outcome: UsageOutcome::Completed,
        terminal: true,
    }
}

#[test]
fn provider_reported_usage_aggregates_into_ui_read_model() {
    let project_id = ProjectId::new();
    let event = event(
        "attempt-1",
        project_id,
        Some(10),
        Some(5),
        Some(42),
        UsageSource::ProviderReported,
        UsageConfidence::Exact,
    );
    let agent_id = event.agent_id;
    let session_id = event.session_id;
    let mut aggregator = UsageAggregator::new(8).unwrap();
    assert_eq!(
        aggregator.record(event).unwrap(),
        UsageRecordResult::Accepted
    );
    let model = aggregator
        .read_model(&project_id, &agent_id, &session_id)
        .unwrap();
    assert_eq!(model.input_tokens, Some(10));
    assert_eq!(model.output_tokens, Some(5));
    assert_eq!(model.cost_micros, Some(42));
    assert_eq!(model.sample_count, 1);
    assert_eq!(model.source, UsageSource::ProviderReported);
    assert_eq!(model.confidence, UsageConfidence::Exact);
}

#[test]
fn missing_usage_stays_optional_and_never_becomes_false_zero() {
    let project_id = ProjectId::new();
    let mut missing = event(
        "attempt-missing",
        project_id,
        None,
        None,
        None,
        UsageSource::Missing,
        UsageConfidence::Unavailable,
    );
    missing.provider_id = None;
    missing.model_id = None;
    let mut aggregator = UsageAggregator::new(8).unwrap();
    aggregator.record(missing.clone()).unwrap();
    let model = aggregator
        .read_model(&project_id, &missing.agent_id, &missing.session_id)
        .unwrap();
    assert_eq!(model.input_tokens, None);
    assert_eq!(model.output_tokens, None);
    assert_eq!(model.cost_micros, None);
    assert_eq!(model.missing_usage_count, 1);
    assert_eq!(model.confidence, UsageConfidence::Unavailable);
}

#[test]
fn duplicate_attempt_is_idempotent_and_does_not_double_count() {
    let project_id = ProjectId::new();
    let first = event(
        "attempt-dup",
        project_id,
        Some(3),
        Some(2),
        None,
        UsageSource::ProviderReported,
        UsageConfidence::Exact,
    );
    let mut duplicate = first.clone();
    duplicate.input_tokens = Some(99);
    let agent_id = first.agent_id;
    let session_id = first.session_id;
    let mut aggregator = UsageAggregator::new(8).unwrap();
    assert_eq!(
        aggregator.record(first).unwrap(),
        UsageRecordResult::Accepted
    );
    assert_eq!(
        aggregator.record(duplicate).unwrap(),
        UsageRecordResult::Duplicate
    );
    let model = aggregator
        .read_model(&project_id, &agent_id, &session_id)
        .unwrap();
    assert_eq!(model.input_tokens, Some(3));
    assert_eq!(model.sample_count, 1);
}

#[test]
fn retries_and_fallback_attempts_count_once_each_and_preserve_confidence_mix() {
    let project_id = ProjectId::new();
    let first = event(
        "attempt-primary",
        project_id,
        Some(4),
        Some(1),
        None,
        UsageSource::ProviderReported,
        UsageConfidence::Exact,
    );
    let mut second = event(
        "attempt-fallback",
        project_id,
        Some(5),
        Some(2),
        None,
        UsageSource::Estimated,
        UsageConfidence::Estimated,
    );
    second.agent_id = first.agent_id;
    second.session_id = first.session_id;
    let agent_id = first.agent_id;
    let session_id = first.session_id;
    let mut aggregator = UsageAggregator::new(8).unwrap();
    aggregator.record(first).unwrap();
    aggregator.record(second).unwrap();
    let model = aggregator
        .read_model(&project_id, &agent_id, &session_id)
        .unwrap();
    assert_eq!(model.input_tokens, Some(9));
    assert_eq!(model.output_tokens, Some(3));
    assert_eq!(model.sample_count, 2);
    assert_eq!(model.source, UsageSource::Mixed);
    assert_eq!(model.confidence, UsageConfidence::Mixed);
}

#[test]
fn overflow_and_nonterminal_events_fail_without_mutation() {
    let project_id = ProjectId::new();
    let mut aggregator = UsageAggregator::new(8).unwrap();
    let mut nonterminal = event(
        "attempt-open",
        project_id,
        Some(1),
        Some(1),
        None,
        UsageSource::ProviderReported,
        UsageConfidence::Exact,
    );
    nonterminal.terminal = false;
    assert_eq!(aggregator.record(nonterminal), Err(UsageError::NotTerminal));
    let mut overflow = event(
        "attempt-overflow",
        project_id,
        Some(u64::MAX),
        Some(1),
        None,
        UsageSource::ProviderReported,
        UsageConfidence::Exact,
    );
    overflow.agent_id = AgentId::new();
    assert_eq!(aggregator.record(overflow), Err(UsageError::Overflow));
    assert!(aggregator
        .read_model(&project_id, &AgentId::new(), &SessionId::new())
        .is_none());
}

#[test]
fn invalid_cost_scope_and_capacity_fail_closed_without_secret_payload() {
    let project_id = ProjectId::new();
    let mut invalid = event(
        "attempt-invalid",
        project_id,
        Some(1),
        Some(1),
        Some(10),
        UsageSource::ProviderReported,
        UsageConfidence::Exact,
    );
    invalid.currency = Some("USD!".into());
    assert_eq!(invalid.validate(), Err(UsageError::Invalid));
    assert!(!format!("{invalid:?}").contains("api_key"));
    let mut aggregator = UsageAggregator::new(1).unwrap();
    aggregator
        .record(event(
            "attempt-one",
            project_id,
            Some(1),
            Some(1),
            None,
            UsageSource::ProviderReported,
            UsageConfidence::Exact,
        ))
        .unwrap();
    assert_eq!(
        aggregator.record(event(
            "attempt-two",
            project_id,
            Some(1),
            Some(1),
            None,
            UsageSource::ProviderReported,
            UsageConfidence::Exact
        )),
        Err(UsageError::Capacity)
    );
}
