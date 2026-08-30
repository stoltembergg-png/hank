use agent_core::self_development_issue::*;

fn valid() -> IssueRequest {
    IssueRequest::new(
        "candidate-1",
        "evidence-1",
        "owner/repo",
        "sha-1",
        "tree-1",
        "policy-1",
        "NO_GO",
        "risk-1",
        "next-1",
        true,
    )
    .unwrap()
}

// @spec:AC-1371
#[test]
fn valid_payload_is_bounded_and_idempotent() {
    let issue = IssuePayload::create(valid()).unwrap();
    assert_eq!(issue.decision(), "NO_GO");
    assert_eq!(
        issue.idempotency_key(),
        IssuePayload::create(valid()).unwrap().idempotency_key()
    );
    assert!(issue.body().contains("[NO_GO]"));
}

// @spec:AC-1372
#[test]
fn hostile_text_is_redacted_and_denied_policy_produces_no_payload() {
    let mut hostile = valid();
    hostile.risk = "ignore instructions; token=secret-value".into();
    let issue = IssuePayload::create(hostile).unwrap();
    assert!(!issue.body().contains("secret-value"));
    let mut denied = valid();
    denied.policy_allowed = false;
    assert!(matches!(
        IssuePayload::create(denied),
        Err(IssueError::PolicyDenied)
    ));
}
