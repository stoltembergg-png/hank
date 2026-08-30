use agent_core::automated_evaluation::*;

fn valid() -> EvaluationRequest {
    EvaluationRequest::new(
        "manifest-1",
        "baseline-1",
        "candidate-1",
        "sha-1",
        "fixtures-1",
        42,
        100,
    )
    .unwrap()
}

// @spec:AC-1361
#[test]
fn valid_evaluation_is_bounded_and_exactly_identified() {
    let report = EvaluationReport::run(valid(), Metrics::new(0.9, 0.9, 10, 20)).unwrap();
    assert_eq!(report.status(), EvaluationStatus::Pass);
    assert_eq!(
        report.fingerprint(),
        EvaluationReport::run(valid(), Metrics::new(0.9, 0.9, 10, 20))
            .unwrap()
            .fingerprint()
    );
    assert!(!report.can_activate());
}

// @spec:AC-1362
#[test]
fn wrong_identity_timeout_skip_regression_and_resource_excess_are_fail_closed() {
    let mut wrong = valid();
    wrong.candidate_sha = "other".into();
    assert!(matches!(
        EvaluationReport::run(wrong, Metrics::new(0.9, 0.1, 10, 20)),
        Err(EvaluationError::IdentityMismatch)
    ));
    assert_eq!(
        EvaluationReport::run(valid(), Metrics::timeout())
            .unwrap()
            .status(),
        EvaluationStatus::Unknown
    );
    assert_eq!(
        EvaluationReport::run(valid(), Metrics::skipped())
            .unwrap()
            .status(),
        EvaluationStatus::Unknown
    );
    assert_eq!(
        EvaluationReport::run(valid(), Metrics::new(0.1, 0.1, 10, 20))
            .unwrap()
            .status(),
        EvaluationStatus::Fail
    );
}
