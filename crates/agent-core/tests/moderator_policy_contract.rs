use agent_core::{
    AgentId, GroupModeratorPolicy, ModeratorDecision, PolicyRollbackError, ProjectId,
};

fn policy() -> GroupModeratorPolicy {
    GroupModeratorPolicy::new(ProjectId::new(), uuid::Uuid::new_v4(), AgentId::new(), 2).unwrap()
}

#[test]
// @spec:AC-915
fn eligible_member_routes_only_when_all_hard_gates_pass() {
    let target = AgentId::new();
    let mut value = policy();
    value.add_eligible_member(target).unwrap();
    assert_eq!(
        value.decide(target, true, true, true),
        ModeratorDecision::Route { target }
    );
    assert_eq!(
        value.decide(target, false, true, true),
        ModeratorDecision::DenyCycleOrDepth
    );
    assert_eq!(
        value.decide(target, true, true, false),
        ModeratorDecision::DenyBudget
    );
}

#[test]
// @spec:AC-916
fn spoofed_actor_or_ineligible_target_fails_closed() {
    let value = policy();
    assert_eq!(
        value.decide(value.moderator_id(), true, true, true),
        ModeratorDecision::DenyTargetNotEligible
    );
    assert_eq!(
        value.decide(AgentId::new(), true, true, true),
        ModeratorDecision::DenyTargetNotEligible
    );
}

#[test]
// @spec:AC-917
fn policy_version_rollback_restores_previous_route() {
    let mut value = policy();
    let old = value.snapshot();
    value.set_max_participants(1).unwrap();
    let version = value.version();
    assert!(version > old.version);
    value.rollback(old).unwrap();
    assert_eq!(value.version(), version + 1);
    assert!(matches!(
        value.rollback(value.snapshot()),
        Err(PolicyRollbackError::InvalidSnapshot)
    ));
}
