use agent_protocol::ids::{ProjectId, RunId, TraceId};
use serde_json::json;
use test_support::evaluation::*;
use test_support::ids::{project_id, run_id, trace_id};

fn metric_schema() -> MetricSchema {
    MetricSchema::new(
        "metrics-v1",
        vec![
            MetricDefinition::new(
                MetricName::Success,
                MetricValueKind::Boolean,
                MetricDirection::Exact,
                true,
                None,
                None,
            ),
            MetricDefinition::new(
                MetricName::TestsPassing,
                MetricValueKind::Count,
                MetricDirection::HigherIsBetter,
                true,
                Some(0.0),
                None,
            ),
            MetricDefinition::new(
                MetricName::LatencyMs,
                MetricValueKind::DurationMs,
                MetricDirection::LowerIsBetter,
                true,
                Some(0.0),
                Some(60_000.0),
            ),
        ],
    )
    .unwrap()
}

fn base_spec() -> EvaluationCaseSpec {
    EvaluationCaseSpec {
        schema_version: EVALUATION_CASE_SCHEMA_VERSION,
        case_id: "case-rust-bug".into(),
        project_id: project_id(1),
        run_id: run_id(2),
        trace_id: trace_id(3),
        scenario_id: "rust-bug".into(),
        task_contract_digest: "task-digest-1".into(),
        fixture: FixtureDescriptor::new("fixture-001", "v1", "fixture-digest-1", 42, true).unwrap(),
        scorer: ScorerDescriptor::new("deterministic-scorer", "v1", "scorer-digest-1").unwrap(),
        metric_schema: metric_schema(),
        authority: Some(EvaluationAuthority::VirtualOnly),
        allowed_effects: vec![EvaluationEffect::VirtualToolCall],
        expected_terminal: Some(EvaluationTerminal::Pass),
        holdout: Some(
            HoldoutMarker::new(
                HoldoutPartition::Training,
                "suite-v1",
                "partition-v1",
                "case-rust-bug",
            )
            .unwrap(),
        ),
        policy_revision: "eval-policy-v1".into(),
        schema_revision: "hank-eval-v1".into(),
        model_class: "reasoning-small".into(),
        idempotency_key: "eval-case-001".into(),
        cancellation: CancellationPolicy::new(true, 30_000, 16_384).unwrap(),
        budget: EvaluationBudget::new(32, 100_000, 1_000_000).unwrap(),
        artifact_requirements: vec![ArtifactRequirement::new(
            ArtifactKind::Result,
            "artifact-digest-1",
        )
        .unwrap()],
    }
}

fn valid_case() -> EvaluationCase {
    EvaluationCase::new(base_spec()).unwrap()
}

fn evidence(status: EvaluationEvidenceStatus) -> EvaluationEvidence {
    EvaluationEvidence::new(
        "sha-1",
        "tree-1",
        "policy-digest-1",
        "schema-digest-1",
        "fixture-digest-1",
        "environment-digest-1",
        vec!["artifact-digest-1".into()],
        status,
    )
    .unwrap()
}

fn valid_report(case: &EvaluationCase) -> BaselineReport {
    BaselineReport::from_case(
        case,
        EvaluationTerminal::Pass,
        vec![
            MetricObservation::boolean(MetricName::Success, true),
            MetricObservation::count(MetricName::TestsPassing, 3),
            MetricObservation::duration_ms(MetricName::LatencyMs, 42),
        ],
        evidence(EvaluationEvidenceStatus::Pass),
    )
    .unwrap()
}

// @spec:AC-1435
#[test]
fn valid_case_is_versioned_scoped_deterministic_and_roundtrips() {
    let case = valid_case();
    assert_eq!(case.schema_version, EVALUATION_CASE_SCHEMA_VERSION);
    assert_eq!(case.project_id, project_id(1));
    assert_eq!(case.run_id, run_id(2));
    assert_eq!(case.trace_id, trace_id(3));
    assert_eq!(case.fingerprint(), valid_case().fingerprint());

    let encoded = serde_json::to_value(&case).unwrap();
    let decoded: EvaluationCase = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    assert!(!encoded.to_string().contains("prompt"));
    assert!(!encoded.to_string().contains("chain_of_thought"));
}

// @spec:AC-1436
#[test]
fn missing_authority_terminal_or_holdout_metadata_fails_closed() {
    let mut missing_authority = base_spec();
    missing_authority.authority = None;
    assert!(matches!(
        EvaluationCase::new(missing_authority),
        Err(EvaluationContractError::MissingAuthority)
    ));

    let mut missing_terminal = base_spec();
    missing_terminal.expected_terminal = None;
    assert!(matches!(
        EvaluationCase::new(missing_terminal),
        Err(EvaluationContractError::MissingExpectedTerminal)
    ));

    let mut missing_holdout = base_spec();
    missing_holdout.holdout = None;
    assert!(matches!(
        EvaluationCase::new(missing_holdout),
        Err(EvaluationContractError::MissingHoldout)
    ));
}

// @spec:AC-1437
#[test]
fn unknown_metric_and_secret_fields_are_rejected_by_wire_contract() {
    let case = valid_case();

    let mut unknown_metric = serde_json::to_value(&case).unwrap();
    unknown_metric["metric_schema"]["metrics"][0]["name"] = json!("unknown_metric");
    assert!(serde_json::from_value::<EvaluationCase>(unknown_metric).is_err());

    let mut secret_field = serde_json::to_value(&case).unwrap();
    secret_field["api_key"] = json!("not-allowed");
    assert!(serde_json::from_value::<EvaluationCase>(secret_field).is_err());

    let mut unsupported_schema = serde_json::to_value(&case).unwrap();
    unsupported_schema["schema_version"] = json!(2);
    assert!(serde_json::from_value::<EvaluationCase>(unsupported_schema).is_err());
}

// @spec:AC-1438
#[test]
fn unsafe_fixture_and_external_effect_are_not_valid_evaluation_authority() {
    let mut unsafe_fixture = base_spec();
    unsafe_fixture.fixture.deterministic = false;
    assert!(matches!(
        EvaluationCase::new(unsafe_fixture),
        Err(EvaluationContractError::UnsafeFixture)
    ));

    let mut unsafe_wire = serde_json::to_value(valid_case()).unwrap();
    unsafe_wire["fixture"]["deterministic"] = json!(false);
    assert!(serde_json::from_value::<EvaluationCase>(unsafe_wire).is_err());

    let mut external_effect = base_spec();
    external_effect
        .allowed_effects
        .push(EvaluationEffect::ExternalWrite);
    assert!(matches!(
        EvaluationCase::new(external_effect),
        Err(EvaluationContractError::UnsafeEffect)
    ));
}

// @spec:AC-1439
#[test]
fn baseline_report_requires_exact_case_identity_and_declared_metrics() {
    let case = valid_case();
    let report = valid_report(&case);
    assert_eq!(report.terminal, EvaluationTerminal::Pass);
    assert_eq!(report.report_digest, valid_report(&case).report_digest);
    assert!(!report.can_activate());
    assert!(report.validate_against(&case).is_ok());
    let encoded = serde_json::to_value(&report).unwrap();
    assert_eq!(
        serde_json::from_value::<BaselineReport>(encoded.clone())
            .unwrap()
            .report_digest,
        report.report_digest
    );
    let mut stale_wire = encoded;
    stale_wire["report_digest"] = json!("stale");
    assert!(serde_json::from_value::<BaselineReport>(stale_wire).is_err());

    let mut foreign = report.clone();
    foreign.project_id = project_id(99);
    assert!(matches!(
        foreign.validate_against(&case),
        Err(EvaluationContractError::IdentityMismatch)
    ));

    let mut missing_metric = report;
    missing_metric.metrics.pop();
    assert!(matches!(
        missing_metric.validate_against(&case),
        Err(EvaluationContractError::MissingMetric)
    ));
}

// @spec:AC-1440
#[test]
fn cancelled_and_no_proof_reports_remain_bounded_and_non_promotable() {
    let case = valid_case();
    for terminal in [EvaluationTerminal::Cancelled, EvaluationTerminal::NoProof] {
        let report = BaselineReport::from_case(
            &case,
            terminal,
            vec![
                MetricObservation::boolean(MetricName::Success, false),
                MetricObservation::count(MetricName::TestsPassing, 0),
                MetricObservation::duration_ms(MetricName::LatencyMs, 0),
            ],
            evidence(EvaluationEvidenceStatus::NoProof),
        )
        .unwrap();
        assert_eq!(report.terminal, terminal);
        assert!(!report.can_activate());
        assert!(report.validate_against(&case).is_ok());
    }
}

#[allow(dead_code)]
fn _typed_ids_are_not_stringly_typed(
    project: ProjectId,
    run: RunId,
    trace: TraceId,
) -> (ProjectId, RunId, TraceId) {
    (project, run, trace)
}
