use agent_core::{AgentGroup, AgentGroupError, AgentGroupLifecycle, AgentId, ProjectId, TraceId};

fn group(project_id: ProjectId) -> AgentGroup {
    AgentGroup::new(
        project_id,
        "research-team".into(),
        AgentId::new(),
        TraceId::new(),
    )
}

#[test]
// @spec:AC-855
fn group_is_project_scoped_and_bounded() {
    let project = ProjectId::new();
    let mut value = group(project);
    value.max_rounds = 4;
    value.budget.max_tokens = 2_000;
    value.context_refs.push("project://context/brief".into());
    assert!(value.validate().is_ok());
    assert_eq!(value.project_id, project);
    assert_eq!(value.lifecycle, AgentGroupLifecycle::Draft);
}

#[test]
// @spec:AC-856
fn invalid_members_limits_budget_and_context_are_rejected() {
    let project = ProjectId::new();
    let mut value = group(project);
    let foreign_member = AgentId::new();
    value.members.push(foreign_member);
    value
        .member_projects
        .push((foreign_member, ProjectId::new()));
    assert_eq!(value.validate(), Err(AgentGroupError::MemberProjectUnknown));

    let mut invalid = group(project);
    invalid.max_rounds = 0;
    assert_eq!(invalid.validate(), Err(AgentGroupError::InvalidLimits));

    let mut context = group(project);
    context.context_refs.push("file:///etc/passwd".into());
    assert_eq!(
        context.validate(),
        Err(AgentGroupError::InvalidContextReference)
    );
}

#[test]
// @spec:AC-857
fn lifecycle_archive_is_idempotent_and_activation_requires_pin() {
    let project = ProjectId::new();
    let mut value = group(project);
    value.lifecycle = AgentGroupLifecycle::Archived;
    assert!(value.validate().is_ok());
    assert!(value.archive().is_ok());
    assert_eq!(value.archive(), Ok(false));

    let mut active = group(project);
    assert_eq!(
        active.activate(),
        Err(AgentGroupError::MissingPinnedVersion)
    );
    active.pinned_version = Some("1.0.0".into());
    assert_eq!(active.activate(), Ok(()));
    assert_eq!(active.lifecycle, AgentGroupLifecycle::Active);
}
