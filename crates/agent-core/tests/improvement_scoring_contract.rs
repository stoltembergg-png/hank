use agent_core::improvement_scoring::*;

fn valid() -> ScoreRequest {
    ScoreRequest::new("policy-1", "evidence-1", Metrics::new(0.9, 0.9, 0.8, 0.7)).unwrap()
}

// @spec:AC-1365
#[test]
fn valid_score_is_stable_explainable_and_bounded() {
    let score = ImprovementScore::calculate(valid()).unwrap();
    assert_eq!(score.class(), ScoreClass::Pass);
    assert_eq!(
        score.fingerprint(),
        ImprovementScore::calculate(valid()).unwrap().fingerprint()
    );
    assert!((score.value() - 0.86).abs() < 0.001);
    assert!(!score.can_activate());
}

// @spec:AC-1366
#[test]
fn unknown_missing_policy_stale_evidence_and_hard_blockers_are_no_go() {
    let mut unknown = valid();
    unknown.metrics.quality = None;
    assert_eq!(
        ImprovementScore::calculate(unknown).unwrap().class(),
        ScoreClass::Unknown
    );
    let mut security = valid();
    security.security_failure = true;
    assert_eq!(
        ImprovementScore::calculate(security).unwrap().class(),
        ScoreClass::NoGo
    );
    let mut stale = valid();
    stale.evidence_stale = true;
    assert_eq!(
        ImprovementScore::calculate(stale).unwrap().class(),
        ScoreClass::NoGo
    );
    let missing = ScoreRequest::new("", "evidence-1", Metrics::new(0.9, 0.9, 0.8, 0.7));
    assert!(missing.is_err());
}
