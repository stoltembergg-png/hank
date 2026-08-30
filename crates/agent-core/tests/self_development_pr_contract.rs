use agent_core::self_development_pr::*;

fn valid() -> PrRequest {
    PrRequest::new(
        "candidate-1",
        "issue-1",
        "feature/task-1",
        "base-1",
        "head-1",
        "tree-1",
        "proposal-1",
        "evaluation-1",
        "regression-1",
        "rollback-1",
    )
    .unwrap()
}

// @spec:AC-1375
#[test]
fn valid_request_creates_draft_with_all_evidence() {
    let draft = PrDraft::create(valid()).unwrap();
    assert!(draft.is_draft());
    assert!(draft.review_required());
    assert!(!draft.approved());
    assert_eq!(
        draft.idempotency_key(),
        PrDraft::create(valid()).unwrap().idempotency_key()
    );
}

// @spec:AC-1376
#[test]
fn missing_evidence_and_identity_changes_fail_closed() {
    let mut missing = valid();
    missing.rollback_evidence.clear();
    assert!(matches!(
        PrDraft::create(missing),
        Err(PrError::MissingEvidence)
    ));
    let draft = PrDraft::create(valid()).unwrap();
    assert_eq!(draft.status("head-1", "tree-1"), DraftStatus::Current);
    assert_eq!(draft.status("head-2", "tree-2"), DraftStatus::Stale);
}
