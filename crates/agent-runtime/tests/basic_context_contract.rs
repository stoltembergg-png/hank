use agent_core::ids::{AgentId, ProjectId};
use agent_runtime::context::{
    basic::{BasicContextBuilder, BasicContextLayer, BasicContextRequest, BasicContextSource},
    ContextBuildError, ContextOmissionReason, ContextSource, ContextSourceKind,
};
use provider_core::CancellationToken;

fn source(id: &str, kind: ContextSourceKind, content: &str, tokens: u32) -> ContextSource {
    ContextSource::new(id, kind, content, tokens).unwrap()
}

fn request(
    sources: Vec<BasicContextSource>,
    max_tokens: u32,
    window: usize,
) -> BasicContextRequest {
    BasicContextRequest::new(
        ProjectId::new(),
        AgentId::new(),
        sources,
        max_tokens,
        window,
        CancellationToken::new(),
    )
    .unwrap()
}

fn basic(layer: BasicContextLayer, source: ContextSource) -> BasicContextSource {
    BasicContextSource::new(layer, source).unwrap()
}

#[test]
fn basic_builder_assembles_hierarchy_deterministically() {
    let result = BasicContextBuilder::build(request(
        vec![
            basic(
                BasicContextLayer::Conversation,
                source("conversation", ContextSourceKind::User, "conversation", 1),
            ),
            basic(
                BasicContextLayer::Project,
                source("project", ContextSourceKind::Project, "project", 1),
            ),
            basic(
                BasicContextLayer::Security,
                source("security", ContextSourceKind::Security, "security", 1),
            ),
            basic(
                BasicContextLayer::System,
                source("system", ContextSourceKind::System, "system", 1),
            ),
            basic(
                BasicContextLayer::Agent,
                source("agent", ContextSourceKind::Agent, "agent", 1),
            ),
        ],
        10,
        10,
    ))
    .unwrap();
    assert_eq!(
        result
            .context
            .entries
            .iter()
            .map(|entry| entry.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["security", "system", "project", "agent", "conversation"]
    );
    assert_eq!(result.context.consumed_tokens, 5);
}

#[test]
fn conversation_window_omits_oldest_messages_deterministically() {
    let result = BasicContextBuilder::build(request(
        (0..4)
            .map(|index| {
                basic(
                    BasicContextLayer::Conversation,
                    source(
                        &format!("message-{index}"),
                        ContextSourceKind::User,
                        &format!("content-{index}"),
                        1,
                    ),
                )
            })
            .collect(),
        10,
        2,
    ))
    .unwrap();
    assert_eq!(
        result
            .context
            .entries
            .iter()
            .map(|entry| entry.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["message-2", "message-3"]
    );
    assert_eq!(
        result
            .context
            .omissions
            .iter()
            .filter(|item| item.reason == ContextOmissionReason::ConversationWindow)
            .count(),
        2
    );
}

#[test]
fn lower_layer_kind_mismatch_is_disallowed_and_tools_never_execute() {
    let wrong = BasicContextSource::new(
        BasicContextLayer::System,
        source("wrong", ContextSourceKind::User, "user", 1),
    )
    .unwrap();
    let result = BasicContextBuilder::build(request(
        vec![
            wrong,
            basic(
                BasicContextLayer::Tools,
                source("tool", ContextSourceKind::Tool, "describe", 1),
            ),
        ],
        10,
        10,
    ))
    .unwrap();
    assert!(result
        .context
        .entries
        .iter()
        .any(|entry| entry.source_id == "tool" && !entry.tool_executable));
    assert!(result
        .context
        .omissions
        .iter()
        .any(|item| item.source_id == "wrong" && item.reason == ContextOmissionReason::Disallowed));
}

#[test]
fn basic_builder_preserves_budget_sensitive_duplicate_and_cancelled_results() {
    let mut duplicate = source("duplicate", ContextSourceKind::Project, "first", 2);
    duplicate.duplicate_key = Some("same".into());
    let mut duplicate_lower = source("duplicate-lower", ContextSourceKind::User, "second", 2);
    duplicate_lower.duplicate_key = Some("same".into());
    let mut sensitive = source("sensitive", ContextSourceKind::Project, "private", 1);
    sensitive.sensitive = true;
    let result = BasicContextBuilder::build(request(
        vec![
            basic(BasicContextLayer::Project, duplicate),
            basic(BasicContextLayer::Conversation, duplicate_lower),
            basic(BasicContextLayer::Project, sensitive),
            basic(
                BasicContextLayer::Agent,
                source("budget", ContextSourceKind::Agent, "too much", 10),
            ),
        ],
        2,
        10,
    ))
    .unwrap();
    assert!(result
        .context
        .omissions
        .iter()
        .any(|item| item.reason == ContextOmissionReason::Duplicate));
    assert!(result
        .context
        .omissions
        .iter()
        .any(|item| item.reason == ContextOmissionReason::Sensitive));
    assert!(result
        .context
        .omissions
        .iter()
        .any(|item| item.reason == ContextOmissionReason::Budget));

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = BasicContextRequest::new(
        ProjectId::new(),
        AgentId::new(),
        vec![],
        10,
        1,
        cancellation,
    )
    .unwrap();
    assert!(matches!(
        BasicContextBuilder::build(cancelled),
        Err(ContextBuildError::Cancelled)
    ));
}

#[test]
fn invalid_window_and_empty_sources_fail_closed() {
    assert!(BasicContextRequest::new(
        ProjectId::new(),
        AgentId::new(),
        vec![],
        10,
        0,
        CancellationToken::new()
    )
    .is_err());
    assert!(ContextSource::new("", ContextSourceKind::Project, "x", 1).is_err());
    let valid = ContextSource::new("project", ContextSourceKind::Project, "x", 1).unwrap();
    assert!(BasicContextSource::new(BasicContextLayer::Project, valid).is_ok());
}
