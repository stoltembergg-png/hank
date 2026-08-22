use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use agent_runtime::confirmation_application::{
    ApproveConfirmationInput, ConfirmationApplicationService,
};
use serde_json::json;
use tool_core::{ApprovalRequest, ConfirmationLedger, ConfirmationPolicy, ToolEffect};

fn request() -> ApprovalRequest {
    ApprovalRequest::new(
        ProjectId::new(),
        None,
        "filesystem.write",
        "1.0.0",
        &json!({"type": "object"}),
        &json!({"path": "note.txt", "content": "secret"}),
        ToolEffect::Write,
        None,
        TraceId::new(),
        "actor:user",
        ConfirmationPolicy::AskEveryTime,
        1_000,
        2_000,
    )
    .unwrap()
}

#[test]
// @spec:AC-674
fn application_boundary_registers_and_returns_redacted_confirmation_artifacts() {
    let service = ConfirmationApplicationService::new(ConfirmationLedger::new());
    let request = request();
    let submitted = service.submit(request.clone()).unwrap();
    let encoded = serde_json::to_string(&submitted).unwrap();

    assert_eq!(submitted, request);
    assert!(!encoded.contains("secret"));
    assert!(encoded.contains(&request.args_hash));
}

#[test]
// @spec:AC-674
fn application_boundary_approves_and_authorizes_only_the_presented_actor() {
    let service = ConfirmationApplicationService::new(ConfirmationLedger::new());
    let request = request();
    service.submit(request.clone()).unwrap();
    let grant = service
        .approve(ApproveConfirmationInput {
            request_id: request.request_id,
            actor_id: "actor:user".into(),
            now_ms: 1_500,
        })
        .unwrap();

    service
        .authorize(&request, &grant, "actor:user", 1_600)
        .unwrap();
    assert!(service
        .authorize(&request, &grant, "actor:other", 1_600)
        .is_err());
}

#[test]
// @spec:AC-674
fn application_boundary_revoke_blocks_authorization_and_replay() {
    let service = ConfirmationApplicationService::new(ConfirmationLedger::new());
    let request = request();
    service.submit(request.clone()).unwrap();
    let grant = service
        .approve(ApproveConfirmationInput {
            request_id: request.request_id,
            actor_id: "actor:user".into(),
            now_ms: 1_500,
        })
        .unwrap();

    service
        .authorize(&request, &grant, "actor:user", 1_600)
        .unwrap();
    assert!(service
        .authorize(&request, &grant, "actor:user", 1_700)
        .is_err());
    service.revoke(&request).unwrap();
    assert!(service
        .authorize(&request, &grant, "actor:user", 1_800)
        .is_err());
}
