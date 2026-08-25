use agent_core::{BudgetLimits, ProjectId};
use agent_protocol::ids::TraceId;
use agent_runtime::skill_rollback::{
    SkillRollbackReason, SkillRollbackRequest, SkillRollbackService, SkillRollbackStatus,
};

fn request(active: &str, target: &str, known_good: bool) -> SkillRollbackRequest {
    SkillRollbackRequest {
        project_id: ProjectId::new(),
        actor_id: "actor-rollback".into(),
        trace_id: TraceId::new(),
        budget: BudgetLimits::default(),
        active_version: active.into(),
        target_version: target.into(),
        target_digest: "b".repeat(64),
        known_good,
    }
}

#[test]
// @spec:AC-841
fn known_good_target_is_restorable_and_non_destructive_to_provenance() {
    let decision = SkillRollbackService::decide(request("2.0.0", "1.0.0", true)).unwrap();
    assert_eq!(decision.status, SkillRollbackStatus::Applied);
    assert_eq!(decision.reason, SkillRollbackReason::RestoredKnownGood);
    assert!(decision.active_pointer_changed);
    assert!(decision.cache_invalidation_required);
    assert_eq!(decision.decision_digest.len(), 64);
}

#[test]
// @spec:AC-842
fn repeated_rollback_is_idempotent() {
    let decision = SkillRollbackService::decide(request("1.0.0", "1.0.0", true)).unwrap();
    assert_eq!(decision.status, SkillRollbackStatus::AlreadyApplied);
    assert!(!decision.active_pointer_changed);
}

#[test]
// @spec:AC-843
fn unknown_target_and_invalid_identity_fail_closed() {
    let denied = SkillRollbackService::decide(request("2.0.0", "9.0.0", false)).unwrap();
    assert_eq!(denied.status, SkillRollbackStatus::Denied);
    assert_eq!(denied.reason, SkillRollbackReason::UnknownTarget);

    let mut invalid = request("2.0.0", "1.0.0", true);
    invalid.target_digest.clear();
    assert!(SkillRollbackService::decide(invalid).is_err());
}
