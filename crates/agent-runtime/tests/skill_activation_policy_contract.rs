use agent_core::{AutonomyLevel, AutonomyPolicy, BudgetLimits, ProjectId};
use agent_protocol::ids::TraceId;
use agent_runtime::skill_activation_policy::{
    SkillActivationDecision, SkillActivationPolicy, SkillActivationReason, SkillActivationRequest,
    SkillActivationStatus,
};

fn request(level: AutonomyLevel, approval: bool) -> SkillActivationRequest {
    let digest = "a".repeat(64);
    SkillActivationRequest {
        project_id: ProjectId::new(),
        actor_id: "actor-1".into(),
        trace_id: TraceId::new(),
        policy: AutonomyPolicy::defaults_for_level(level),
        budget: BudgetLimits::default(),
        candidate_version: "1.1.0".into(),
        candidate_digest: digest.clone(),
        validation_digest: digest.clone(),
        evaluation_digest: digest.clone(),
        autonomous_test_digest: digest,
        human_approval: approval,
    }
}

#[test]
// @spec:AC-836
fn l3_allows_activation_without_mutating_pointer() {
    let decision =
        SkillActivationPolicy::decide(request(AutonomyLevel::L3Autonomous, false)).unwrap();
    assert_eq!(decision.status, SkillActivationStatus::Allowed);
    assert_eq!(decision.reason, SkillActivationReason::AllowedByPolicy);
    assert!(!decision.active_pointer_changed);
    assert_eq!(decision.decision_digest.len(), 64);
}

#[test]
// @spec:AC-837
fn lower_autonomy_requires_approval_or_denies() {
    let required =
        SkillActivationPolicy::decide(request(AutonomyLevel::L2SemiAutonomous, false)).unwrap();
    assert_eq!(required.status, SkillActivationStatus::RequiresApproval);
    assert_eq!(
        required.reason,
        SkillActivationReason::HumanApprovalRequired
    );

    let approved =
        SkillActivationPolicy::decide(request(AutonomyLevel::L2SemiAutonomous, true)).unwrap();
    assert_eq!(approved.status, SkillActivationStatus::Allowed);

    let denied = SkillActivationPolicy::decide(request(AutonomyLevel::L1Assisted, false)).unwrap();
    assert_eq!(denied.status, SkillActivationStatus::Denied);
}

#[test]
// @spec:AC-838
fn missing_evidence_and_invalid_identity_fail_closed() {
    let mut incomplete = request(AutonomyLevel::L3Autonomous, false);
    incomplete.evaluation_digest.clear();
    assert!(SkillActivationPolicy::decide(incomplete).is_err());

    let mut invalid = request(AutonomyLevel::L3Autonomous, false);
    invalid.actor_id.clear();
    assert!(SkillActivationPolicy::decide(invalid).is_err());
}

#[allow(dead_code)]
fn _schema_type_is_stable(_: SkillActivationDecision) {}
