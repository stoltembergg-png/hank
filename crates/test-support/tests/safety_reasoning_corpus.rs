use std::collections::BTreeSet;

use tempfile::tempdir;
use test_support::evaluation::{
    EvaluationContractError, EvaluationEvidenceStatus, EvaluationTerminal, MetricName, MetricValue,
};
use test_support::fixtures::FixtureWorkspace;
use test_support::safety_reasoning_corpus::{
    safety_reasoning_evaluation_corpus, SafetyReasoningEvaluationFixture,
    SafetyReasoningFailureMode, SAFETY_REASONING_EVALUATION_CORPUS_SCHEMA_VERSION,
};

fn find_case<'a>(
    corpus: &'a [SafetyReasoningEvaluationFixture],
    scenario_id: &str,
) -> &'a SafetyReasoningEvaluationFixture {
    corpus
        .iter()
        .find(|fixture| fixture.case.scenario_id == scenario_id)
        .unwrap_or_else(|| panic!("missing scenario {scenario_id}"))
}

fn count_metric(fixture: &SafetyReasoningEvaluationFixture, name: MetricName) -> u64 {
    fixture
        .baseline
        .metrics
        .iter()
        .find(|metric| metric.name == name)
        .and_then(|metric| match metric.value {
            MetricValue::Count(value) => Some(value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing count metric {name:?}"))
}

// @spec:AC-1447
#[test]
fn safety_reasoning_corpus_contains_the_six_versioned_scenarios() {
    let corpus = safety_reasoning_evaluation_corpus().unwrap();

    assert_eq!(SAFETY_REASONING_EVALUATION_CORPUS_SCHEMA_VERSION, 1);
    assert_eq!(corpus.len(), 6);
    assert_eq!(
        corpus
            .iter()
            .map(|fixture| fixture.case.scenario_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "failure_memory",
            "skill_selection",
            "fabricated_evidence",
            "delegation",
            "budget",
            "tool_misuse",
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
    let report_digests = corpus
        .iter()
        .map(|fixture| fixture.baseline.report_digest.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(case_ids.len(), corpus.len());
    assert_eq!(fixture_ids.len(), corpus.len());
    assert_eq!(report_digests.len(), corpus.len());
}

// @spec:AC-1448
#[test]
fn every_safety_case_is_observably_fail_closed() {
    let corpus = safety_reasoning_evaluation_corpus().unwrap();

    for fixture in &corpus {
        fixture.case.validate().unwrap();
        fixture.baseline.validate_against(&fixture.case).unwrap();
        assert_ne!(fixture.case.expected_terminal, EvaluationTerminal::Pass);
        assert_eq!(fixture.case.expected_terminal, fixture.baseline.terminal);
        assert!(matches!(
            fixture.baseline.evidence.status,
            EvaluationEvidenceStatus::Blocked | EvaluationEvidenceStatus::NoProof
        ));
        assert!(!fixture.baseline.can_activate());
        assert!(matches!(
            fixture
                .baseline
                .metrics
                .iter()
                .find(|metric| metric.name == MetricName::Success)
                .map(|metric| &metric.value),
            Some(MetricValue::Boolean(false))
        ));
    }
}

// @spec:AC-1449
#[test]
fn fabricated_evidence_is_detected_and_cannot_activate() {
    let corpus = safety_reasoning_evaluation_corpus().unwrap();
    let fabricated = find_case(&corpus, "fabricated_evidence");

    assert_eq!(
        fabricated.failure_mode,
        SafetyReasoningFailureMode::FabricatedEvidence
    );
    assert!(count_metric(fabricated, MetricName::EvidenceConflicts) > 0);
    assert_eq!(
        fabricated.baseline.evidence.status,
        EvaluationEvidenceStatus::NoProof
    );
    assert!(!fabricated.baseline.can_activate());

    let mut stale = fabricated.baseline.clone();
    stale.evidence.fixture_digest = "stale-fabricated-evidence".into();
    assert!(matches!(
        stale.validate_against(&fabricated.case),
        Err(EvaluationContractError::EvidenceMismatch)
    ));
}

// @spec:AC-1450
#[test]
fn cross_project_shadow_write_and_policy_bypass_are_explicitly_denied() {
    let corpus = safety_reasoning_evaluation_corpus().unwrap();
    let delegation = find_case(&corpus, "delegation");
    let skill_selection = find_case(&corpus, "skill_selection");
    let tool_misuse = find_case(&corpus, "tool_misuse");

    assert_eq!(
        delegation.failure_mode,
        SafetyReasoningFailureMode::CrossProjectDelegation
    );
    assert_eq!(
        skill_selection.failure_mode,
        SafetyReasoningFailureMode::SkillPolicyBypass
    );
    assert_eq!(
        tool_misuse.failure_mode,
        SafetyReasoningFailureMode::ToolMisuse
    );

    for fixture in [delegation, skill_selection, tool_misuse] {
        assert!(fixture
            .case
            .allowed_effects
            .iter()
            .all(|effect| *effect != test_support::evaluation::EvaluationEffect::ExternalWrite));
    }
    assert!(count_metric(delegation, MetricName::PolicyViolations) > 0);
    assert!(count_metric(skill_selection, MetricName::PolicyViolations) > 0);
    assert!(count_metric(tool_misuse, MetricName::ExternalSideEffectAttempts) > 0);
}

// @spec:AC-1451
#[test]
fn safety_fixtures_materialize_deterministically_and_stay_offline() {
    let corpus = safety_reasoning_evaluation_corpus().unwrap();
    let directory = tempdir().unwrap();
    let workspace = FixtureWorkspace::create(directory.path().join("safety-corpus")).unwrap();

    for fixture in &corpus {
        let digest = fixture.materialize(&workspace).unwrap();
        assert_eq!(digest, fixture.case.fixture.fixture_digest);
        assert_eq!(
            workspace.read(&fixture.fixture.id).unwrap(),
            fixture.fixture
        );
        let payload = fixture.fixture.payload.to_ascii_lowercase();
        assert!(!payload.contains("http://"));
        assert!(!payload.contains("https://"));
        assert!(!payload.contains("api_key"));
        assert!(!payload.contains("-----begin"));
        assert!(!payload.contains("authorization:"));
    }
}

// @spec:AC-1452
#[test]
fn replay_is_stable_and_unsafe_fixture_paths_fail_closed() {
    let corpus = safety_reasoning_evaluation_corpus().unwrap();
    let replay = safety_reasoning_evaluation_corpus().unwrap();

    for (first, second) in corpus.iter().zip(&replay) {
        assert_eq!(first.case.fingerprint(), second.case.fingerprint());
        assert_eq!(first.baseline.report_digest, second.baseline.report_digest);
    }

    let mut escaped = corpus[0].clone();
    escaped.fixture.id = "../shadow-write".into();
    escaped.case.fixture.fixture_id = "../shadow-write".into();
    let directory = tempdir().unwrap();
    let workspace = FixtureWorkspace::create(directory.path().join("safety-corpus")).unwrap();
    assert!(escaped.materialize(&workspace).is_err());
    assert!(!directory.path().join("shadow-write.json").exists());
}
