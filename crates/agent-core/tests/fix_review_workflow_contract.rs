use agent_core::fix_review_workflow::*;

fn mapping() -> FixReviewMapping {
    FixReviewMapping::new(
        "project", "task", "repo", "worktree", "branch", "commit-1", "tree-1", "policy-1",
    )
    .unwrap()
}
fn finding(commit: &str) -> ReviewFinding {
    ReviewFinding::blocker("finding-1", "fix required", commit, "tree-1", "review-1").unwrap()
}

// @spec:AC-1345
#[test]
fn correction_preserves_mapping_and_supersedes_old_evidence() {
    let plan = FixReviewWorkflow::plan(&mapping(), &finding("commit-1"), 0, 3).unwrap();
    assert_eq!(plan.state(), FixReviewState::CorrectionPlanned);
    let task = plan.task().expect("correction task expected");
    assert_eq!(task.mapping(), &mapping());
    assert_eq!(task.supersedes_review(), "review-1");
    assert!(!plan.fingerprint().is_empty());
}

// @spec:AC-1345
#[test]
fn evidence_from_another_commit_is_stale() {
    let result = FixReviewWorkflow::plan(&mapping(), &finding("other-commit"), 0, 3);
    assert!(matches!(result, Err(FixReviewError::StaleEvidence)));
}

// @spec:AC-1346
#[test]
fn cycle_below_cap_allows_next_bounded_retry() {
    let plan = FixReviewWorkflow::plan(&mapping(), &finding("commit-1"), 1, 3).unwrap();
    assert_eq!(plan.next_cycle(), 2);
    assert_eq!(plan.state(), FixReviewState::CorrectionPlanned);
}

// @spec:AC-1346
#[test]
fn cycle_at_cap_escalates_without_creating_task() {
    let plan = FixReviewWorkflow::plan(&mapping(), &finding("commit-1"), 3, 3).unwrap();
    assert_eq!(plan.state(), FixReviewState::Escalated);
    assert!(plan.task().is_none());
}
