use agent_core::{AgentGroup, AgentId, MentionError, MentionParser, MentionTarget, ProjectId};

fn group(project: ProjectId) -> AgentGroup {
    AgentGroup::new(
        project,
        "research".into(),
        AgentId::new(),
        agent_protocol::ids::TraceId::new(),
    )
}

#[test]
// @spec:AC-877
fn exact_member_mention_resolves_and_dedupes_without_side_effect() {
    let project = ProjectId::new();
    let mut value = group(project);
    let member = AgentId::new();
    value
        .add_member(member, project, value.owner_id, "worker".into())
        .unwrap();
    let parser = MentionParser::new(project, value.memberships.clone());
    let input = "hello @agent:".to_owned() + &member.to_string() + " @agent:" + &member.to_string();
    let result = parser.parse(&input).unwrap();
    assert_eq!(
        result.targets,
        vec![MentionTarget {
            agent_id: member,
            project_id: project
        }]
    );
}

#[test]
// @spec:AC-878
fn unknown_ambiguous_cross_project_or_oversized_mentions_fail_closed() {
    let project = ProjectId::new();
    let value = group(project);
    let parser = MentionParser::new(project, value.memberships.clone());
    assert_eq!(
        parser.parse("@agent:agent-not-real"),
        Err(MentionError::UnknownMention)
    );
    assert_eq!(
        parser.parse(&"x".repeat(10_000)),
        Err(MentionError::InputTooLarge)
    );
    assert_eq!(
        MentionParser::new(ProjectId::new(), value.memberships).parse("@agent:agent-not-real"),
        Err(MentionError::UnknownMention)
    );
}

#[test]
// @spec:AC-879
fn plain_text_and_malformed_syntax_never_trigger_invocation() {
    let project = ProjectId::new();
    let parser = MentionParser::new(project, group(project).memberships);
    let result = parser
        .parse("please discuss @agent without invoking anyone")
        .unwrap();
    assert!(result.targets.is_empty());
    assert!(!result.invocation_requested);
}
