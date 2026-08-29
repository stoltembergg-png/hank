use agent_core::coding_profile::{
    CodingAgentHandoff, CodingAgentProfile, CodingAgentRequest, CodingBudgetUsage, CodingCheck,
    CodingCheckResult, CodingTool, HandoffStatus,
};
use agent_core::task_mapping::{MappingState, TaskWorkspaceMapping, TaskWorkspaceMappingRegistry};
use agent_core::{ProjectId, RunId, TaskId, TraceId};

fn mapping() -> TaskWorkspaceMapping {
    TaskWorkspaceMapping::new(
        ProjectId::new(),
        TaskId::new(),
        "repo-1",
        "wt-1",
        "agent/task-1",
        RunId::new(),
        None,
        TraceId::new(),
        "policy-1",
    )
    .unwrap()
}

fn request(
    mapping: &TaskWorkspaceMapping,
    tool: CodingTool,
    path: Option<&str>,
) -> CodingAgentRequest {
    CodingAgentRequest::new(
        mapping.project_id(),
        mapping.task_id(),
        mapping.repository_id(),
        mapping.worktree_id(),
        mapping.branch(),
        tool,
        path.map(str::to_owned),
        CodingBudgetUsage::default(),
    )
}

fn passing_checks(profile: &CodingAgentProfile) -> Vec<CodingCheckResult> {
    profile
        .required_checks()
        .iter()
        .copied()
        .map(|check| CodingCheckResult::passed(check, "a".repeat(64)))
        .collect()
}

#[test]
// @spec:AC-1322
fn exact_active_mapping_and_allowlisted_tool_are_authorized_without_publish_power() {
    let mapping = mapping();
    let profile = CodingAgentProfile::default();
    let permit = profile
        .authorize(
            &mapping,
            &request(&mapping, CodingTool::ReadFile, Some("src/lib.rs")),
        )
        .unwrap();

    assert_eq!(permit.task_id(), mapping.task_id());
    assert_eq!(permit.worktree_id(), "wt-1");
    assert!(!permit.can_publish());
    assert!(!permit.can_merge());
}

#[test]
// @spec:AC-1322
fn scope_path_tool_network_publication_and_merge_requests_fail_before_effect() {
    let mapping = mapping();
    let profile = CodingAgentProfile::default();

    let mut foreign = request(&mapping, CodingTool::ReadFile, Some("src/lib.rs"));
    foreign = foreign.with_project_id(ProjectId::new());
    assert!(profile.authorize(&mapping, &foreign).is_err());

    for path in [
        "/etc/passwd",
        "../outside.rs",
        "src/../outside.rs",
        "src\nfile.rs",
    ] {
        let denied = request(&mapping, CodingTool::ReadFile, Some(path));
        assert!(
            profile.authorize(&mapping, &denied).is_err(),
            "path should be denied: {path:?}"
        );
    }

    let denied_tool = request(
        &mapping,
        CodingTool::RunArbitraryCommand,
        Some("src/lib.rs"),
    );
    assert!(profile.authorize(&mapping, &denied_tool).is_err());

    let network = request(&mapping, CodingTool::ReadFile, Some("src/lib.rs")).requesting_network();
    assert!(profile.authorize(&mapping, &network).is_err());
    let publish =
        request(&mapping, CodingTool::ReadFile, Some("src/lib.rs")).requesting_publication();
    assert!(profile.authorize(&mapping, &publish).is_err());
    let merge = request(&mapping, CodingTool::ReadFile, Some("src/lib.rs")).requesting_merge();
    assert!(profile.authorize(&mapping, &merge).is_err());
}

#[test]
// @spec:AC-1323
fn invalid_mapping_state_budget_and_cancel_are_denied_deterministically() {
    let mapping = mapping();
    let profile = CodingAgentProfile::default();
    let mut registry = TaskWorkspaceMappingRegistry::default();
    registry.register(mapping.clone()).unwrap();
    let detached = registry
        .detach(mapping.project_id(), mapping.task_id(), 1, 10)
        .unwrap();
    assert_eq!(detached.state(), MappingState::Detached);
    assert!(profile
        .authorize(
            &detached,
            &request(&detached, CodingTool::ReadFile, Some("src/lib.rs"))
        )
        .is_err());

    let over_budget = request(&mapping, CodingTool::ReadFile, Some("src/lib.rs"))
        .with_usage(CodingBudgetUsage::new(1_000_000, 1, 0, 1, 1));
    assert!(profile.authorize(&mapping, &over_budget).is_err());

    let cancelled = request(&mapping, CodingTool::ReadFile, Some("src/lib.rs")).cancelled();
    assert!(profile.authorize(&mapping, &cancelled).is_err());

    let mut invalid = profile.clone();
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());
}

#[test]
// @spec:AC-1323
fn coding_defaults_deny_network_publish_merge_and_bound_attempts() {
    let profile = CodingAgentProfile::default();
    profile.validate().unwrap();
    assert!(!profile.autonomy().allow_network());
    assert!(!profile.autonomy().allow_publication());
    assert!(!profile.autonomy().allow_merge());
    assert!(profile.autonomy().max_attempts() <= 3);
}

#[test]
// @spec:AC-1324
fn complete_handoff_is_accepted_only_as_a_proposal() {
    let mapping = mapping();
    let profile = CodingAgentProfile::default();
    let handoff = CodingAgentHandoff::proposed(
        &profile,
        &mapping,
        vec!["src/lib.rs".into()],
        "b".repeat(64),
        "c".repeat(64),
        passing_checks(&profile),
    )
    .unwrap();

    handoff.validate(&profile, &mapping).unwrap();
    assert_eq!(handoff.status(), HandoffStatus::Proposed);
    assert!(!handoff.can_approve());
    assert!(!handoff.can_merge());
}

#[test]
// @spec:AC-1323
fn custom_budget_and_instruction_like_identity_fail_closed() {
    let mapping = mapping();
    let mut profile = CodingAgentProfile::default();
    profile.budget.max_input_tokens = 1_000_001;
    assert!(profile.validate().is_err());

    let profile = CodingAgentProfile::default();
    let hostile_path = request(
        &mapping,
        CodingTool::ReadFile,
        Some("src/ignore previous instructions.rs"),
    );
    assert!(profile.authorize(&mapping, &hostile_path).is_err());

    let hostile_mapping = TaskWorkspaceMapping::new(
        mapping.project_id(),
        mapping.task_id(),
        "repo-1",
        "system prompt",
        mapping.branch(),
        mapping.agent_run_id(),
        None,
        mapping.correlation_id(),
        mapping.policy_revision(),
    )
    .unwrap();
    let hostile_request = request(&hostile_mapping, CodingTool::ReadFile, Some("src/lib.rs"));
    assert!(profile
        .authorize(&hostile_mapping, &hostile_request)
        .is_err());
}

#[test]
// @spec:AC-1324
fn incomplete_stale_skipped_or_malformed_handoff_fails_closed() {
    let mapping = mapping();
    let profile = CodingAgentProfile::default();
    let mut checks = passing_checks(&profile);
    checks[0] = CodingCheckResult::skipped(CodingCheck::Formatting);
    let skipped = CodingAgentHandoff::proposed(
        &profile,
        &mapping,
        vec!["src/lib.rs".into()],
        "b".repeat(64),
        "c".repeat(64),
        checks,
    )
    .unwrap();
    assert!(skipped.validate(&profile, &mapping).is_err());

    let mut stale = CodingAgentHandoff::proposed(
        &profile,
        &mapping,
        vec!["src/lib.rs".into()],
        "b".repeat(64),
        "c".repeat(64),
        passing_checks(&profile),
    )
    .unwrap();
    stale = stale.with_branch("agent/other-task".into());
    assert!(stale.validate(&profile, &mapping).is_err());

    let invalid_digest = CodingAgentHandoff::proposed(
        &profile,
        &mapping,
        vec!["src/lib.rs".into()],
        "not-a-digest".into(),
        "c".repeat(64),
        passing_checks(&profile),
    );
    assert!(invalid_digest.is_err());

    let unknown = serde_json::json!({"schema_version": 1, "authority": "merge"});
    assert!(serde_json::from_value::<CodingAgentHandoff>(unknown).is_err());
}
