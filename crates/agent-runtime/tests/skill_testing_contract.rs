use agent_core::ids::{ProjectId, SkillId};
use agent_protocol::ids::TraceId;
use agent_runtime::skill_testing::{
    DeterministicSkillTestRunner, SkillFixture, SkillTestError, SkillTestStep,
};

fn fixture(steps: Vec<SkillTestStep>) -> SkillFixture {
    SkillFixture::new(
        ProjectId::new(),
        SkillId::new(),
        "1.0.0",
        TraceId::new(),
        steps,
        4,
    )
    .unwrap()
}

#[test]
// @spec:AC-791
fn valid_fixture_produces_bounded_deterministic_report() {
    let fixture = fixture(vec![SkillTestStep::AssertLabel {
        label: "manifest-valid".into(),
    }]);

    let report = DeterministicSkillTestRunner::run(&fixture).unwrap();

    assert_eq!(report.project_id, fixture.project_id);
    assert_eq!(report.skill_id, fixture.skill_id);
    assert_eq!(report.version, "1.0.0");
    assert_eq!(report.trace_id, fixture.trace_id);
    assert_eq!(report.steps_executed, 1);
    assert_eq!(report.status, "passed");
    assert!(!report.fixture_digest.is_empty());
}

#[test]
// @spec:AC-792
fn malformed_or_privileged_fixture_fails_closed_before_execution() {
    assert!(matches!(
        SkillFixture::new(
            ProjectId::new(),
            SkillId::new(),
            "",
            TraceId::new(),
            vec![],
            0,
        ),
        Err(SkillTestError::InvalidManifest(_))
    ));

    let fixture = fixture(vec![SkillTestStep::ExecuteScript {
        source: "echo forbidden".into(),
    }]);
    assert!(matches!(
        DeterministicSkillTestRunner::run(&fixture),
        Err(SkillTestError::PrivilegedStep("script"))
    ));
}

#[test]
// @spec:AC-793
fn rerun_preserves_identity_digest_and_has_no_activation_operation() {
    let fixture = fixture(vec![
        SkillTestStep::AssertLabel {
            label: "one".into(),
        },
        SkillTestStep::AssertLabel {
            label: "two".into(),
        },
    ]);

    let first = DeterministicSkillTestRunner::run(&fixture).unwrap();
    let second = DeterministicSkillTestRunner::run(&fixture).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.steps_executed, 2);
    assert!(!first.activation_requested);
}
