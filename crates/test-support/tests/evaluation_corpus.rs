use std::collections::BTreeSet;

use tempfile::tempdir;
use test_support::evaluation::{
    EvaluationContractError, EvaluationEvidenceStatus, EvaluationTerminal, MetricName,
};
use test_support::evaluation_corpus::{
    core_evaluation_corpus, CoreEvaluationFixture, CORE_EVALUATION_CORPUS_SCHEMA_VERSION,
};
use test_support::fixtures::FixtureWorkspace;

fn find_case<'a>(
    corpus: &'a [CoreEvaluationFixture],
    scenario_id: &str,
) -> &'a CoreEvaluationFixture {
    corpus
        .iter()
        .find(|fixture| fixture.case.scenario_id == scenario_id)
        .unwrap_or_else(|| panic!("missing scenario {scenario_id}"))
}

// @spec:AC-1441
#[test]
fn core_corpus_contains_the_six_versioned_scenarios() {
    let corpus = core_evaluation_corpus().unwrap();

    assert_eq!(CORE_EVALUATION_CORPUS_SCHEMA_VERSION, 1);
    assert_eq!(corpus.len(), 6);
    assert_eq!(
        corpus
            .iter()
            .map(|fixture| fixture.case.scenario_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "rust_bug",
            "ci_failure",
            "architecture_violation",
            "vulnerable_dependency",
            "unsafe_operation",
            "interrupted_task",
        ]
    );

    let case_ids = corpus
        .iter()
        .map(|fixture| fixture.case.case_id.as_str())
        .collect::<BTreeSet<_>>();
    let fixture_ids = corpus
        .iter()
        .map(|fixture| fixture.fixture.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(case_ids.len(), corpus.len());
    assert_eq!(fixture_ids.len(), corpus.len());
}

// @spec:AC-1442
#[test]
fn every_core_case_has_bounded_terminal_artifacts_and_metrics() {
    let corpus = core_evaluation_corpus().unwrap();
    let required_metrics = [
        MetricName::Success,
        MetricName::TestsPassing,
        MetricName::ToolCalls,
        MetricName::FailedToolCalls,
        MetricName::Retries,
        MetricName::Tokens,
        MetricName::Cost,
        MetricName::EvidenceQuality,
    ];

    for fixture in &corpus {
        fixture.case.validate().unwrap();
        assert!(!fixture.case.artifact_requirements.is_empty());
        assert_eq!(fixture.case.expected_terminal, fixture.baseline.terminal);
        assert!(fixture.baseline.validate_against(&fixture.case).is_ok());

        let metric_names = fixture
            .baseline
            .metrics
            .iter()
            .map(|metric| metric.name)
            .collect::<BTreeSet<_>>();
        for required_metric in required_metrics {
            assert!(metric_names.contains(&required_metric));
        }
    }
}

// @spec:AC-1443
#[test]
fn corpus_fixture_materialization_matches_the_case_digest() {
    let corpus = core_evaluation_corpus().unwrap();
    let directory = tempdir().unwrap();
    let workspace = FixtureWorkspace::create(directory.path().join("core-corpus")).unwrap();

    for fixture in &corpus {
        let digest = fixture.materialize(&workspace).unwrap();
        assert_eq!(digest, fixture.case.fixture.fixture_digest);
        assert_eq!(
            workspace.read(&fixture.fixture.id).unwrap(),
            fixture.fixture
        );
    }
}

// @spec:AC-1444
#[test]
fn core_corpus_is_virtual_only_and_has_no_network_or_secret_fixture() {
    let corpus = core_evaluation_corpus().unwrap();

    for fixture in &corpus {
        assert_eq!(
            fixture.case.authority,
            test_support::evaluation::EvaluationAuthority::VirtualOnly
        );
        assert!(fixture
            .case
            .allowed_effects
            .iter()
            .all(|effect| *effect != test_support::evaluation::EvaluationEffect::ExternalWrite));
        let payload = fixture.fixture.payload.to_ascii_lowercase();
        assert!(!payload.contains("http://"));
        assert!(!payload.contains("https://"));
        assert!(!payload.contains("secret"));
        assert!(!payload.contains("api_key"));
    }
}

// @spec:AC-1445
#[test]
fn unsafe_and_interrupted_cases_keep_explicit_non_success_terminals() {
    let corpus = core_evaluation_corpus().unwrap();
    let unsafe_operation = find_case(&corpus, "unsafe_operation");
    let interrupted_task = find_case(&corpus, "interrupted_task");

    assert_eq!(
        unsafe_operation.case.expected_terminal,
        EvaluationTerminal::Blocked
    );
    assert_eq!(
        unsafe_operation.baseline.evidence.status,
        EvaluationEvidenceStatus::Blocked
    );
    assert_eq!(
        interrupted_task.case.expected_terminal,
        EvaluationTerminal::Cancelled
    );
    assert_eq!(
        interrupted_task.baseline.evidence.status,
        EvaluationEvidenceStatus::NoProof
    );
    assert!(!unsafe_operation.baseline.can_activate());
    assert!(!interrupted_task.baseline.can_activate());
}

// @spec:AC-1446
#[test]
fn stale_evidence_and_replayed_corpus_digests_fail_closed() {
    let corpus = core_evaluation_corpus().unwrap();
    let rust_bug = find_case(&corpus, "rust_bug");
    let replay = core_evaluation_corpus().unwrap();
    let replayed = find_case(&replay, "rust_bug");

    assert_eq!(rust_bug.case.fingerprint(), replayed.case.fingerprint());
    assert_eq!(
        rust_bug.baseline.report_digest,
        replayed.baseline.report_digest
    );

    let mut stale = rust_bug.baseline.clone();
    stale.evidence.fixture_digest = "fixture-digest-stale".into();
    assert!(matches!(
        stale.validate_against(&rust_bug.case),
        Err(EvaluationContractError::EvidenceMismatch)
    ));
}
