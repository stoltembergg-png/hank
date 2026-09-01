use tempfile::tempdir;
use test_support::benchmark_comparison::{
    BenchmarkComparison, BenchmarkComparisonError, BenchmarkComparisonPolicy,
    BenchmarkComparisonStatus, IndependentReviewArtifact, IndependentReviewDisposition,
};
use test_support::evaluation::{
    BaselineReport, EvaluationEvidence, EvaluationEvidenceStatus, EvaluationTerminal, MetricName,
    MetricObservation, MetricValue,
};
use test_support::evaluation_corpus::{core_evaluation_corpus, CoreEvaluationFixture};
use test_support::evaluation_runner::{
    NativeEvaluationEnvironment, NativeEvaluationRun, NativeEvaluationRunner,
};
use test_support::fixtures::FixtureWorkspace;

const BASELINE_ID: &str = "skill-baseline-v1";
const CANDIDATE_ID: &str = "skill-candidate-v2";

fn baseline_run() -> (NativeEvaluationRun, Vec<CoreEvaluationFixture>) {
    let corpus = core_evaluation_corpus().unwrap();
    let environment =
        NativeEvaluationEnvironment::from_evidence(&corpus[0].baseline.evidence).unwrap();
    let directory = tempdir().unwrap();
    let workspace = FixtureWorkspace::create(directory.path().join("runner")).unwrap();
    let run = NativeEvaluationRunner::default()
        .run(&corpus, &environment, &workspace)
        .unwrap();
    (run, corpus)
}

fn candidate_run(
    baseline: &NativeEvaluationRun,
    corpus: &[CoreEvaluationFixture],
    mutate: impl Fn(&str, &mut Vec<MetricObservation>, &mut EvaluationTerminal),
) -> NativeEvaluationRun {
    let environment = NativeEvaluationEnvironment::new(
        "sha-candidate-v2",
        "tree-candidate-v2",
        baseline.environment.policy_digest.clone(),
        baseline.environment.schema_digest.clone(),
        baseline.environment.environment_digest.clone(),
    )
    .unwrap();

    let reports = corpus
        .iter()
        .map(|entry| {
            let mut metrics = entry.baseline.metrics.clone();
            let mut terminal = entry.baseline.terminal;
            mutate(&entry.case.case_id, &mut metrics, &mut terminal);
            let evidence = EvaluationEvidence::new(
                environment.head_sha.clone(),
                environment.tree_sha.clone(),
                environment.policy_digest.clone(),
                environment.schema_digest.clone(),
                entry.case.fixture.fixture_digest.clone(),
                environment.environment_digest.clone(),
                entry.baseline.evidence.artifact_digests.clone(),
                match terminal {
                    EvaluationTerminal::Pass => EvaluationEvidenceStatus::Pass,
                    EvaluationTerminal::Fail => EvaluationEvidenceStatus::Fail,
                    EvaluationTerminal::Blocked => EvaluationEvidenceStatus::Blocked,
                    EvaluationTerminal::Cancelled | EvaluationTerminal::NoProof => {
                        EvaluationEvidenceStatus::NoProof
                    }
                },
            )
            .unwrap();
            BaselineReport::from_case(&entry.case, terminal, metrics, evidence).unwrap()
        })
        .collect();

    NativeEvaluationRun::from_reports(
        baseline.suite_id.clone(),
        baseline.corpus_schema_version,
        environment,
        reports,
    )
    .unwrap()
}

fn review(
    baseline: &NativeEvaluationRun,
    candidate: &NativeEvaluationRun,
) -> IndependentReviewArtifact {
    IndependentReviewArtifact::new(
        "reviewer-independent-v1",
        "reviewer-tool-v1",
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline.run_digest,
        &candidate.run_digest,
        IndependentReviewDisposition::Reviewed,
    )
    .unwrap()
}

fn set_count(metrics: &mut [MetricObservation], name: MetricName, value: u64) {
    let metric = metrics
        .iter_mut()
        .find(|metric| metric.name == name)
        .unwrap();
    metric.value = match metric.value {
        MetricValue::Count(_) => MetricValue::Count(value),
        MetricValue::DurationMs(_) => MetricValue::DurationMs(value),
        _ => panic!("metric is not numeric"),
    };
}

fn set_success(metrics: &mut [MetricObservation], value: bool) {
    let metric = metrics
        .iter_mut()
        .find(|metric| metric.name == MetricName::Success)
        .unwrap();
    metric.value = MetricValue::Boolean(value);
}

fn set_terminal(metrics: &mut [MetricObservation], terminal: EvaluationTerminal) {
    let metric = metrics
        .iter_mut()
        .find(|metric| metric.name == MetricName::TerminalState)
        .unwrap();
    metric.value = MetricValue::Category(
        match terminal {
            EvaluationTerminal::Pass => "pass",
            EvaluationTerminal::Fail => "fail",
            EvaluationTerminal::Blocked => "blocked",
            EvaluationTerminal::Cancelled => "cancelled",
            EvaluationTerminal::NoProof => "no_proof",
        }
        .into(),
    );
}

// @spec:AC-1491
#[test]
fn comparable_baseline_and_candidate_produce_training_holdout_deltas() {
    let (baseline, corpus) = baseline_run();
    let candidate = candidate_run(&baseline, &corpus, |_case_id, _metrics, _terminal| {});
    let review = review(&baseline, &candidate);

    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        Some(&review),
    )
    .unwrap();

    assert_eq!(report.status, BenchmarkComparisonStatus::Pass);
    assert_eq!(report.training.case_count, 4);
    assert_eq!(report.holdout.case_count, 2);
    assert_eq!(report.regressions.len(), 0);
    assert_eq!(report.training.deltas.len(), 68);
    assert_eq!(report.holdout.deltas.len(), 34);
    assert!(!report.report_digest.is_empty());
}

// @spec:AC-1492
#[test]
fn holdout_regression_blocks_even_when_training_improves() {
    let (baseline, corpus) = baseline_run();
    let candidate = candidate_run(&baseline, &corpus, |case_id, metrics, terminal| {
        if case_id == "core-rust_bug" {
            set_count(metrics, MetricName::FailedToolCalls, 0);
        }
        if case_id == "core-unsafe_operation" {
            *terminal = EvaluationTerminal::Pass;
            set_success(metrics, true);
            set_terminal(metrics, EvaluationTerminal::Pass);
        }
    });
    let review = review(&baseline, &candidate);

    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        Some(&review),
    )
    .unwrap();

    assert_eq!(report.status, BenchmarkComparisonStatus::Regression);
    assert!(report.holdout.regression_count > 0);
    assert!(report
        .regressions
        .iter()
        .any(|delta| delta.case_id == "core-unsafe_operation"));
}

// @spec:AC-1492
#[test]
fn training_success_regression_within_threshold_is_allowed() {
    let (baseline, corpus) = baseline_run();
    let candidate = candidate_run(&baseline, &corpus, |case_id, metrics, terminal| {
        if case_id == "core-rust_bug" {
            *terminal = EvaluationTerminal::Fail;
            set_success(metrics, false);
            set_terminal(metrics, EvaluationTerminal::Fail);
        }
    });
    let review = review(&baseline, &candidate);
    let policy =
        BenchmarkComparisonPolicy::new("benchmark-policy-success-v1", 1, 0, 0.0, 0, 0, 0).unwrap();

    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &policy,
        Some(&review),
    )
    .unwrap();

    assert_eq!(report.status, BenchmarkComparisonStatus::Pass);
    assert_eq!(report.training.regression_count, 0);
    assert_eq!(report.holdout.regression_count, 0);
}

// @spec:AC-1492
#[test]
fn declared_tool_and_resource_metrics_are_compared() {
    let (baseline, corpus) = baseline_run();
    let candidate = candidate_run(&baseline, &corpus, |case_id, metrics, _terminal| {
        if case_id == "core-rust_bug" {
            set_count(metrics, MetricName::Tokens, 513);
            set_count(metrics, MetricName::ExternalSideEffectAttempts, 1);
        }
    });
    let review = review(&baseline, &candidate);

    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        Some(&review),
    )
    .unwrap();

    assert_eq!(report.status, BenchmarkComparisonStatus::Regression);
    assert!(report
        .regressions
        .iter()
        .any(|delta| { delta.case_id == "core-rust_bug" && delta.metric == MetricName::Tokens }));
    assert!(report.regressions.iter().any(|delta| {
        delta.case_id == "core-rust_bug" && delta.metric == MetricName::ExternalSideEffectAttempts
    }));
}

// @spec:AC-1492
#[test]
fn configured_cost_threshold_is_applied_without_hiding_other_deltas() {
    let (baseline, corpus) = baseline_run();
    let candidate = candidate_run(&baseline, &corpus, |case_id, metrics, _terminal| {
        if case_id == "core-rust_bug" {
            set_count(metrics, MetricName::Cost, 1_050);
        }
    });
    let review = review(&baseline, &candidate);

    let permissive =
        BenchmarkComparisonPolicy::new("benchmark-policy-cost-v1", 0, 0, 0.0, 100, 0, 0).unwrap();
    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &permissive,
        Some(&review),
    )
    .unwrap();
    assert_eq!(report.status, BenchmarkComparisonStatus::Pass);

    let strict =
        BenchmarkComparisonPolicy::new("benchmark-policy-cost-v1", 0, 0, 0.0, 49, 0, 0).unwrap();
    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &strict,
        Some(&review),
    )
    .unwrap();
    assert_eq!(report.status, BenchmarkComparisonStatus::Regression);
    assert!(report
        .regressions
        .iter()
        .any(|delta| { delta.case_id == "core-rust_bug" && delta.metric == MetricName::Cost }));
}

// @spec:AC-1493
#[test]
fn partial_or_self_selected_benchmark_is_rejected() {
    let (baseline, corpus) = baseline_run();
    let mut candidate = candidate_run(&baseline, &corpus, |_case_id, _metrics, _terminal| {});
    candidate.reports.pop();
    let review = review(&baseline, &candidate);

    let error = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        Some(&review),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        BenchmarkComparisonError::MissingCase { .. }
    ));
}

// @spec:AC-1493
#[test]
fn policy_or_environment_drift_is_incomparable() {
    let (baseline, corpus) = baseline_run();
    let mut candidate = candidate_run(&baseline, &corpus, |_case_id, _metrics, _terminal| {});
    candidate.environment.policy_digest = "policy-digest-different-v1".into();
    let review = review(&baseline, &candidate);

    let error = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        Some(&review),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        BenchmarkComparisonError::IncomparableEnvironment
    ));
}

// @spec:AC-1494
#[test]
fn missing_review_and_unknown_schema_fail_closed() {
    let (baseline, corpus) = baseline_run();
    let candidate = candidate_run(&baseline, &corpus, |_case_id, _metrics, _terminal| {});

    let error = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        BenchmarkComparisonError::MissingIndependentReview
    ));

    let review = review(&baseline, &candidate);
    let report = BenchmarkComparison::compare(
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline,
        &candidate,
        &BenchmarkComparisonPolicy::default(),
        Some(&review),
    )
    .unwrap();
    let mut encoded = serde_json::to_value(&report).unwrap();
    encoded["unexpected"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<test_support::benchmark_comparison::BenchmarkComparisonReport>(
            encoded
        )
        .is_err()
    );

    let mut tampered = serde_json::to_value(&report).unwrap();
    tampered["report_digest"] = serde_json::json!("forged-report-digest");
    assert!(
        serde_json::from_value::<test_support::benchmark_comparison::BenchmarkComparisonReport>(
            tampered
        )
        .is_err()
    );

    let self_review = IndependentReviewArtifact::new(
        CANDIDATE_ID,
        "reviewer-tool-v1",
        BASELINE_ID,
        CANDIDATE_ID,
        &baseline.run_digest,
        &candidate.run_digest,
        IndependentReviewDisposition::Reviewed,
    )
    .unwrap_err();
    assert!(matches!(
        self_review,
        BenchmarkComparisonError::SelfApproval
    ));
}
