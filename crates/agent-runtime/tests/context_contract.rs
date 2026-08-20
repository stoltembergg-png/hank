use agent_core::ids::{AgentId, ProjectId};
use agent_runtime::context::{
    ContextBuildError, ContextBuilder, ContextOmissionReason, ContextRequest, ContextSource,
    ContextSourceKind,
};
use provider_core::CancellationToken;

fn request(sources: Vec<ContextSource>, max_tokens: u32) -> ContextRequest {
    ContextRequest::new(
        ProjectId::new(),
        AgentId::new(),
        sources,
        max_tokens,
        CancellationToken::new(),
    )
    .unwrap()
}

fn source(id: &str, kind: ContextSourceKind, content: &str, tokens: u32) -> ContextSource {
    ContextSource::new(id, kind, content, tokens).unwrap()
}

#[test]
fn priority_is_deterministic_and_budget_omissions_are_explicit() {
    let result = ContextBuilder::build(request(
        vec![
            source("user", ContextSourceKind::User, "user", 4),
            source("system", ContextSourceKind::System, "system", 4),
            source("security", ContextSourceKind::Security, "security", 4),
            source("project", ContextSourceKind::Project, "project", 4),
        ],
        12,
    ))
    .unwrap();
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| entry.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["security", "system", "project"]
    );
    assert!(result
        .omissions
        .iter()
        .any(|item| item.source_id == "user" && item.reason == ContextOmissionReason::Budget));
}

#[test]
fn duplicate_and_sensitive_sources_are_never_silently_included() {
    let mut duplicate = source("duplicate-2", ContextSourceKind::User, "second", 1);
    duplicate.duplicate_key = Some("same".into());
    let mut first = source("duplicate-1", ContextSourceKind::Project, "first", 1);
    first.duplicate_key = Some("same".into());
    let mut sensitive = source("secret", ContextSourceKind::Project, "private", 1);
    sensitive.sensitive = true;
    let result = ContextBuilder::build(request(vec![duplicate, first, sensitive], 10)).unwrap();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].source_id, "duplicate-1");
    assert!(result
        .omissions
        .iter()
        .any(|item| item.source_id == "duplicate-2"
            && item.reason == ContextOmissionReason::Duplicate));
    assert!(result
        .omissions
        .iter()
        .any(|item| item.source_id == "secret" && item.reason == ContextOmissionReason::Sensitive));
}

#[test]
fn required_missing_sources_are_reported_and_untrusted_content_is_marked() {
    let mut request = request(
        vec![source("user", ContextSourceKind::User, "untrusted", 1)],
        10,
    );
    request.required_source_ids = vec!["system".into(), "user".into()];
    let result = ContextBuilder::build(request).unwrap();
    assert!(result.entries[0].untrusted);
    assert!(result
        .omissions
        .iter()
        .any(|item| item.source_id == "system" && item.reason == ContextOmissionReason::Missing));
}

#[test]
fn tool_sources_are_metadata_only_and_never_executed() {
    let result = ContextBuilder::build(request(
        vec![source("tool", ContextSourceKind::Tool, "tool output", 2)],
        10,
    ))
    .unwrap();
    assert_eq!(result.entries[0].source_id, "tool");
    assert!(result.entries[0].untrusted);
    assert!(!result.entries[0].tool_executable);
}

#[test]
fn cancellation_is_explicit_before_context_selection() {
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let request =
        ContextRequest::new(ProjectId::new(), AgentId::new(), vec![], 10, cancellation).unwrap();
    assert!(matches!(
        ContextBuilder::build(request),
        Err(ContextBuildError::Cancelled)
    ));
}

#[test]
fn bounds_and_forbidden_content_fail_closed() {
    assert!(ContextSource::new("", ContextSourceKind::User, "x", 1).is_err());
    assert!(ContextSource::new("x", ContextSourceKind::User, "x".repeat(1_048_577), 1).is_err());
    assert!(ContextSource::new("x", ContextSourceKind::User, "api_key=secret", 1).is_err());
    assert!(ContextRequest::new(
        ProjectId::new(),
        AgentId::new(),
        vec![],
        0,
        CancellationToken::new()
    )
    .is_err());
}

#[test]
fn output_is_bounded_and_does_not_expose_raw_omission_content() {
    let result = ContextBuilder::build(request(
        vec![source("user", ContextSourceKind::User, "hello", 1)],
        1,
    ))
    .unwrap();
    let debug = format!("{result:?}");
    assert!(debug.contains("user"));
    assert!(!debug.contains("api_key"));
}
