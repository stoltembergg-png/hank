use agent_core::group_entity::MAX_GROUP_MEMBERS;
use agent_core::{AgentGroup, AgentGroupError, AgentGroupLifecycle, AgentId, ProjectId};

fn group(project_id: ProjectId) -> AgentGroup {
    AgentGroup::new(
        project_id,
        "research-team".into(),
        AgentId::new(),
        agent_protocol::ids::TraceId::new(),
    )
}

#[test]
// @spec:AC-866
fn accessible_member_is_added_once_with_bounded_role() {
    let project = ProjectId::new();
    let mut value = group(project);
    let member = AgentId::new();
    value
        .add_member(member, project, value.owner_id, "worker".into())
        .unwrap();
    assert_eq!(value.members.len(), 2);
    assert_eq!(value.memberships[1].role, "worker");
    assert_eq!(
        value.add_member(member, project, value.owner_id, "worker".into()),
        Err(AgentGroupError::DuplicateMember)
    );
}

#[test]
// @spec:AC-867
fn cross_project_or_unauthorized_membership_is_denied() {
    let project = ProjectId::new();
    let mut value = group(project);
    let member = AgentId::new();
    assert_eq!(
        value.add_member(member, ProjectId::new(), value.owner_id, "worker".into()),
        Err(AgentGroupError::MemberProjectUnknown)
    );
    assert_eq!(
        value.add_member(member, project, AgentId::new(), "worker".into()),
        Err(AgentGroupError::MembershipPermissionDenied)
    );
}

#[test]
// @spec:AC-868
fn remove_and_rollback_snapshot_are_bounded() {
    let project = ProjectId::new();
    let mut value = group(project);
    let member = AgentId::new();
    value
        .add_member(member, project, value.owner_id, "worker".into())
        .unwrap();
    let snapshot = value.memberships.clone();
    value.remove_member(member, value.owner_id).unwrap();
    assert_eq!(value.memberships.len(), 1);
    value.restore_memberships(snapshot).unwrap();
    assert_eq!(value.memberships.len(), 2);
    assert_ne!(value.lifecycle, AgentGroupLifecycle::Archived);
    assert!(value.members.len() <= MAX_GROUP_MEMBERS);
}
