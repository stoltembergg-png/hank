use agent_core::budget::ReservationId;
use agent_core::ids::{AgentId, ProjectId};
use agent_protocol::ids::TraceId;
use serde_json::json;
use tool_core::{
    ApprovalRequest, ConfirmationError, ConfirmationLedger, ConfirmationPolicy, ToolEffect,
};

fn project(suffix: &str) -> ProjectId {
    ProjectId::parse(&format!("proj-00000000-0000-4000-8000-000000000{suffix}"))
        .expect("fixture project id")
}

fn request(policy: ConfirmationPolicy) -> ApprovalRequest {
    ApprovalRequest::new(
        project("101"),
        Some(AgentId::new()),
        "filesystem.write",
        "1.0.0",
        &json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        &json!({"path": "notes.txt", "secret": "must-not-be-stored"}),
        ToolEffect::Write,
        Some(ReservationId::new()),
        TraceId::new(),
        "agent-runtime",
        policy,
        1_000,
        2_000,
    )
    .expect("valid approval request")
}

#[test]
// @spec:AC-669
fn approval_binds_exact_request_without_retaining_raw_arguments() {
    let ledger = ConfirmationLedger::new();
    let request = request(ConfirmationPolicy::AskEveryTime);
    ledger.register(request.clone()).unwrap();
    let grant = ledger
        .approve(request.request_id, "user:gabriel", 1_100)
        .unwrap();

    ledger
        .authorize(&request, &grant, "user:gabriel", 1_101)
        .expect("the presented request is authorized");

    let mut changed = request.clone();
    changed.args_hash = ApprovalRequest::hash_payload(&json!({
        "path": "notes.txt",
        "secret": "changed"
    }))
    .unwrap();
    assert_eq!(
        ledger.authorize(&changed, &grant, "user:gabriel", 1_102),
        Err(ConfirmationError::RequestMismatch)
    );

    let serialized = serde_json::to_string(&grant).unwrap();
    assert!(!serialized.contains("must-not-be-stored"));
    assert_eq!(request.args_hash.len(), 64);
    assert_eq!(request.schema_hash.len(), 64);
}

#[test]
fn hash_payload_returns_stable_lowercase_sha256_hex() {
    let hash = ApprovalRequest::hash_payload(&json!({"a": 1})).unwrap();

    assert_eq!(
        hash,
        "015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
    );
    assert!(hash.chars().all(|character| character.is_ascii_hexdigit()));
}

#[test]
// @spec:AC-670
fn approval_expiry_and_revocation_fail_closed() {
    let ledger = ConfirmationLedger::new();
    let expired = request(ConfirmationPolicy::AskEveryTime);
    ledger.register(expired.clone()).unwrap();
    let expired_grant = ledger
        .approve(expired.request_id, "user:gabriel", 1_999)
        .unwrap();
    assert_eq!(
        ledger.authorize(&expired, &expired_grant, "user:gabriel", 2_000),
        Err(ConfirmationError::Expired)
    );

    let revoked = request(ConfirmationPolicy::AskEveryTime);
    ledger.register(revoked.clone()).unwrap();
    let revoked_grant = ledger
        .approve(revoked.request_id, "user:gabriel", 1_100)
        .unwrap();
    ledger.revoke(&revoked).unwrap();
    assert_eq!(
        ledger.authorize(&revoked, &revoked_grant, "user:gabriel", 1_101),
        Err(ConfirmationError::Revoked)
    );
}

#[test]
// @spec:AC-671
fn ask_every_time_rejects_replay_and_ask_once_reuses_only_same_scope() {
    let ledger = ConfirmationLedger::new();
    let every_time = request(ConfirmationPolicy::AskEveryTime);
    ledger.register(every_time.clone()).unwrap();
    let every_time_grant = ledger
        .approve(every_time.request_id, "user:gabriel", 1_100)
        .unwrap();
    ledger
        .authorize(&every_time, &every_time_grant, "user:gabriel", 1_101)
        .unwrap();
    assert_eq!(
        ledger.authorize(&every_time, &every_time_grant, "user:gabriel", 1_102),
        Err(ConfirmationError::Replay)
    );

    let once = request(ConfirmationPolicy::AskOnce);
    ledger.register(once.clone()).unwrap();
    let once_grant = ledger
        .approve(once.request_id, "user:gabriel", 1_100)
        .unwrap();
    ledger
        .authorize(&once, &once_grant, "user:gabriel", 1_101)
        .unwrap();
    let same_scope = request_with_same_scope(&once);
    ledger
        .authorize(&same_scope, &once_grant, "user:gabriel", 1_102)
        .expect("ask_once may reuse the exact bounded scope");
}

#[test]
// @spec:AC-672
fn project_agent_actor_and_policy_changes_are_rejected() {
    let ledger = ConfirmationLedger::new();
    let request = request(ConfirmationPolicy::AskEveryTime);
    ledger.register(request.clone()).unwrap();
    let grant = ledger
        .approve(request.request_id, "user:gabriel", 1_100)
        .unwrap();

    assert_eq!(
        ledger.authorize(&request, &grant, "user:other", 1_101),
        Err(ConfirmationError::ActorMismatch)
    );

    let mut other_project = request.clone();
    other_project.project_id = project("102");
    assert_eq!(
        ledger.authorize(&other_project, &grant, "user:gabriel", 1_101),
        Err(ConfirmationError::RequestMismatch)
    );

    let mut other_agent = request.clone();
    other_agent.agent_id = Some(AgentId::new());
    assert_eq!(
        ledger.authorize(&other_agent, &grant, "user:gabriel", 1_101),
        Err(ConfirmationError::RequestMismatch)
    );

    let mut other_tool = request.clone();
    other_tool.tool_version = "2.0.0".into();
    assert_eq!(
        ledger.authorize(&other_tool, &grant, "user:gabriel", 1_101),
        Err(ConfirmationError::RequestMismatch)
    );

    let mut denied = request.clone();
    denied.policy = ConfirmationPolicy::Deny;
    assert_eq!(
        ledger.authorize(&denied, &grant, "user:gabriel", 1_101),
        Err(ConfirmationError::PolicyDenied)
    );
}

fn request_with_same_scope(request: &ApprovalRequest) -> ApprovalRequest {
    let mut same = request.clone();
    same.request_id = uuid::Uuid::new_v4();
    same
}
