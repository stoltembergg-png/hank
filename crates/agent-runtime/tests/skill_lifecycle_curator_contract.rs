use agent_core::{ProjectId, Skill, SkillManifest, SkillScope, SkillStatus};
use agent_protocol::ids::TraceId;
use agent_runtime::skill_lifecycle_curator::{
    SkillLifecycleCurator, SkillLifecycleDecisionStatus, SkillLifecycleReason,
    SkillLifecycleRequest,
};

fn request(status: SkillStatus, target: SkillStatus) -> SkillLifecycleRequest {
    let project_id = ProjectId::new();
    let mut manifest = SkillManifest::new("curator", "1.0.0", SkillScope::Project);
    manifest.trace.trace_id = TraceId::new();
    SkillLifecycleRequest {
        project_id,
        actor_id: "curator-actor".into(),
        trace_id: manifest.trace.trace_id,
        skill: {
            let mut skill = Skill::new(manifest, Some(project_id));
            skill.status = status;
            skill
        },
        target,
        validation_passed: true,
        evaluation_passed: true,
        autonomous_test_passed: true,
        activation_allowed: true,
        rollback_available: true,
    }
}

#[test]
// @spec:AC-846
fn legal_transition_requires_gates_and_is_deterministic() {
    let decision =
        SkillLifecycleCurator::decide(request(SkillStatus::Draft, SkillStatus::Testing)).unwrap();
    assert_eq!(decision.status, SkillLifecycleDecisionStatus::Allowed);
    assert_eq!(decision.reason, SkillLifecycleReason::LegalTransition);
    assert!(decision.state_changed);
    assert_eq!(decision.decision_digest.len(), 64);
}

#[test]
// @spec:AC-847
fn activation_requires_all_evidence_and_rollback() {
    let mut missing = request(SkillStatus::Testing, SkillStatus::Active);
    missing.evaluation_passed = false;
    let decision = SkillLifecycleCurator::decide(missing).unwrap();
    assert_eq!(decision.status, SkillLifecycleDecisionStatus::Denied);
    assert_eq!(decision.reason, SkillLifecycleReason::EvidenceMissing);

    let mut no_rollback = request(SkillStatus::Testing, SkillStatus::Active);
    no_rollback.rollback_available = false;
    let decision = SkillLifecycleCurator::decide(no_rollback).unwrap();
    assert_eq!(decision.reason, SkillLifecycleReason::RollbackMissing);
}

#[test]
// @spec:AC-848
fn illegal_cross_project_and_repeated_transitions_fail_closed_or_are_idempotent() {
    let denied =
        SkillLifecycleCurator::decide(request(SkillStatus::Draft, SkillStatus::Active)).unwrap();
    assert_eq!(denied.status, SkillLifecycleDecisionStatus::Denied);
    assert_eq!(denied.reason, SkillLifecycleReason::InvalidTransition);

    let repeated =
        SkillLifecycleCurator::decide(request(SkillStatus::Testing, SkillStatus::Testing)).unwrap();
    assert_eq!(
        repeated.status,
        SkillLifecycleDecisionStatus::AlreadyApplied
    );

    let mut outside = request(SkillStatus::Draft, SkillStatus::Testing);
    outside.skill.project_id = Some(ProjectId::new());
    assert!(SkillLifecycleCurator::decide(outside).is_err());
}
