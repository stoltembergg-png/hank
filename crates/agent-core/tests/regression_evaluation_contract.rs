use agent_core::regression_evaluation::*;

fn valid() -> RegressionRequest {
    RegressionRequest::new(
        "corpus-1",
        "rev-1",
        "baseline-1",
        "candidate-1",
        "sha-1",
        ImpactClass::Workflow,
    )
    .unwrap()
}

// @spec:AC-1363
#[test]
fn valid_corpus_is_comparable_and_fingerprint_is_stable() {
    let report = RegressionReport::evaluate(valid(), RegressionOutcome::Passed).unwrap();
    assert_eq!(report.status(), RegressionStatus::Pass);
    assert_eq!(
        report.fingerprint(),
        RegressionReport::evaluate(valid(), RegressionOutcome::Passed)
            .unwrap()
            .fingerprint()
    );
    assert!(!report.can_activate());
}

// @spec:AC-1364
#[test]
fn missing_skip_stale_unknown_and_critical_regression_are_no_go() {
    for outcome in [
        RegressionOutcome::FixtureMissing,
        RegressionOutcome::Skipped,
        RegressionOutcome::NoRun,
        RegressionOutcome::StaleIdentity,
        RegressionOutcome::ClassifierUnknown,
        RegressionOutcome::CriticalFailure,
    ] {
        assert_eq!(
            RegressionReport::evaluate(valid(), outcome)
                .unwrap()
                .status(),
            RegressionStatus::NoGo
        );
    }
}
