//! Typed bridge between the desktop shell and the confirmation lifecycle.
//!
//! Commands transport only the bounded approval artifacts from the
//! application service; raw schemas and arguments never cross this bridge.
//! Submission emits a current-schema confirmation event with a monotonic
//! sequence so the UI can consume pending approvals deterministically.

use std::sync::atomic::{AtomicU64, Ordering};

use agent_runtime::confirmation_application::{
    ApproveConfirmationInput, ConfirmationApplicationError, ConfirmationApplicationService,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tool_core::{ApprovalGrant, ApprovalRequest, ConfirmationLedger};
use uuid::Uuid;

pub const CONFIRMATION_EVENT_NAME: &str = "hank://confirmation";
pub const CONFIRMATION_SCHEMA_VERSION: u64 = 1;

/// Managed state owning the application service and event sequence.
pub struct ConfirmationBridgeState {
    service: ConfirmationApplicationService,
    next_sequence: AtomicU64,
}

impl ConfirmationBridgeState {
    pub fn new() -> Self {
        Self {
            service: ConfirmationApplicationService::new(ConfirmationLedger::new()),
            next_sequence: AtomicU64::new(0),
        }
    }

    pub fn with_service(service: ConfirmationApplicationService) -> Self {
        Self {
            service,
            next_sequence: AtomicU64::new(0),
        }
    }

    pub fn service(&self) -> &ConfirmationApplicationService {
        &self.service
    }

    fn next_event_sequence(&self) -> u64 {
        self.next_sequence.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for ConfirmationBridgeState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn bridge_state() -> ConfirmationBridgeState {
    ConfirmationBridgeState::new()
}

/// Serializable bridge error; carries only bounded, fixed messages.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationBridgeError {
    Ledger(String),
    EventInvalid,
    EmitFailed,
}

impl std::fmt::Display for ConfirmationBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The application error Display already carries the bounded prefix.
            Self::Ledger(message) => write!(f, "{message}"),
            Self::EventInvalid => write!(f, "confirmation event is invalid"),
            Self::EmitFailed => write!(f, "confirmation event could not be emitted"),
        }
    }
}

impl std::error::Error for ConfirmationBridgeError {}

impl From<ConfirmationApplicationError> for ConfirmationBridgeError {
    fn from(error: ConfirmationApplicationError) -> Self {
        Self::Ledger(error.to_string())
    }
}

/// Bounded confirmation event payload.
#[derive(Debug, Clone, Serialize)]
pub struct ConfirmationEventPayload {
    pub kind: ConfirmationEventKind,
    pub request: ApprovalRequest,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationEventKind {
    RequestSubmitted,
}

/// Current-schema confirmation event emitted on submission.
#[derive(Debug, Clone, Serialize)]
pub struct ConfirmationEvent {
    pub schema_version: u64,
    pub event_id: Uuid,
    pub request_id: Uuid,
    pub sequence: u64,
    pub payload: ConfirmationEventPayload,
}

impl ConfirmationEvent {
    pub fn request_submitted(
        request: &ApprovalRequest,
        sequence: u64,
    ) -> Result<Self, ConfirmationBridgeError> {
        Ok(Self {
            schema_version: CONFIRMATION_SCHEMA_VERSION,
            event_id: Uuid::new_v4(),
            request_id: request.request_id,
            sequence,
            payload: ConfirmationEventPayload {
                kind: ConfirmationEventKind::RequestSubmitted,
                request: request.clone(),
            },
        })
    }
}

/// Registers an approval request and emits the bounded submission event.
#[tauri::command]
pub fn submit_confirmation_request(
    app: AppHandle,
    state: State<'_, ConfirmationBridgeState>,
    request: ApprovalRequest,
) -> Result<ApprovalRequest, ConfirmationBridgeError> {
    let artifact = state.service().submit(request)?;
    let sequence = state.next_event_sequence();
    let event = ConfirmationEvent::request_submitted(&artifact, sequence)?;
    app.emit(CONFIRMATION_EVENT_NAME, &event)
        .map_err(|_| ConfirmationBridgeError::EmitFailed)?;
    Ok(artifact)
}

/// Approves a submitted request for the presenting actor.
#[tauri::command]
pub fn approve_confirmation_request(
    state: State<'_, ConfirmationBridgeState>,
    input: ApproveConfirmationInput,
) -> Result<ApprovalGrant, ConfirmationBridgeError> {
    state.service().approve(input).map_err(Into::into)
}

/// Revokes a request or its bounded ask-once scope.
#[tauri::command]
pub fn revoke_confirmation_request(
    state: State<'_, ConfirmationBridgeState>,
    request: ApprovalRequest,
) -> Result<(), ConfirmationBridgeError> {
    state.service().revoke(&request).map_err(Into::into)
}

/// Confirmation lifecycle handlers for the desktop shell.
pub fn command_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        submit_confirmation_request,
        approve_confirmation_request,
        revoke_confirmation_request,
        crate::memory::list_memories,
        crate::memory::mutate_memory,
        crate::skills::list_skills,
        crate::skills::rollback_skill,
        crate::skills::get_skill_editor,
        crate::skills::validate_skill_draft,
        crate::skills::save_skill_draft,
        crate::skills::discard_skill_draft,
        crate::projects::create_project,
        crate::projects::list_projects,
        crate::projects::get_project,
        crate::projects::update_project,
        crate::projects::archive_project
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::ids::{AgentId, ProjectId};
    use agent_protocol::ids::TraceId;
    use serde_json::json;
    use tool_core::{ConfirmationPolicy, ToolEffect};

    fn project(suffix: &str) -> ProjectId {
        ProjectId::parse(&format!("proj-00000000-0000-4000-8000-000000000{suffix}"))
            .expect("fixture project id")
    }

    fn sample_request() -> ApprovalRequest {
        ApprovalRequest::new(
            project("201"),
            Some(AgentId::new()),
            "git_commit",
            "1.0.0",
            &json!({"type": "object"}),
            &json!({"message": "chore: release"}),
            ToolEffect::Write,
            None,
            TraceId::new(),
            "operator-1",
            ConfirmationPolicy::AskEveryTime,
            1_000,
            61_000,
        )
        .expect("valid request")
    }

    #[test]
    fn submit_approve_authorize_roundtrip_keeps_bindings() {
        let state = ConfirmationBridgeState::new();
        let request = sample_request();

        let artifact = state
            .service()
            .submit(request.clone())
            .expect("submit must register the bounded artifact");
        assert_eq!(artifact, request);

        let grant = state
            .service()
            .approve(ApproveConfirmationInput {
                request_id: request.request_id,
                actor_id: "operator-1".to_string(),
                now_ms: 2_000,
            })
            .expect("approve must issue a grant for the presenting actor");

        state
            .service()
            .authorize(&request, &grant, "operator-1", 3_000)
            .expect("authorize must accept the exact presented context");
    }

    #[test]
    fn replay_and_foreign_actor_are_rejected() {
        let state = ConfirmationBridgeState::new();
        let request = sample_request();
        state.service().submit(request.clone()).expect("submit");

        let grant = state
            .service()
            .approve(ApproveConfirmationInput {
                request_id: request.request_id,
                actor_id: "operator-1".to_string(),
                now_ms: 2_000,
            })
            .expect("approve");

        state
            .service()
            .authorize(&request, &grant, "operator-1", 3_000)
            .expect("first authorize consumes the ask_every_time grant");

        let replay = state
            .service()
            .authorize(&request, &grant, "operator-1", 3_500);
        assert!(replay.is_err(), "ask_every_time must reject replay");

        let foreign = state
            .service()
            .authorize(&request, &grant, "operator-2", 3_600);
        assert!(foreign.is_err(), "foreign actor must be rejected");
    }

    #[test]
    fn bridge_error_never_leaks_raw_payload() {
        let state = ConfirmationBridgeState::new();
        let request = sample_request();
        state.service().submit(request.clone()).expect("submit");

        let grant = state
            .service()
            .approve(ApproveConfirmationInput {
                request_id: request.request_id,
                actor_id: "operator-1".to_string(),
                now_ms: 2_000,
            })
            .expect("approve");

        state
            .service()
            .authorize(&request, &grant, "operator-1", 3_000)
            .expect("first authorize consumes the grant");

        let error = ConfirmationBridgeError::from(
            state
                .service()
                .authorize(&request, &grant, "operator-1", 3_500)
                .expect_err("replay must fail"),
        )
        .to_string();

        assert!(
            !error.contains("chore: release"),
            "args payload leaked: {error}"
        );
        assert!(!error.contains("message"), "args key leaked: {error}");
    }

    #[test]
    fn submission_events_use_current_schema_and_monotonic_sequence() {
        let state = ConfirmationBridgeState::new();
        let first = sample_request();
        let mut second = sample_request();
        second.request_id = Uuid::new_v4();

        let event_a = ConfirmationEvent::request_submitted(&first, state.next_event_sequence())
            .expect("event a");
        let event_b = ConfirmationEvent::request_submitted(&second, state.next_event_sequence())
            .expect("event b");

        assert_eq!(event_a.schema_version, CONFIRMATION_SCHEMA_VERSION);
        assert_eq!(event_b.schema_version, CONFIRMATION_SCHEMA_VERSION);
        assert!(matches!(
            event_a.payload.kind,
            ConfirmationEventKind::RequestSubmitted
        ));
        assert_eq!(event_a.sequence, 0);
        assert_eq!(event_b.sequence, 1);
        assert_eq!(event_a.request_id, first.request_id);
        assert_eq!(event_b.request_id, second.request_id);

        let serialized = serde_json::to_string(&event_a).expect("event must be serializable");
        assert!(
            !serialized.contains("chore: release"),
            "raw args leaked in event"
        );
        assert!(serialized.contains("request_submitted"));
        assert_eq!(event_a.payload.request.args_hash, first.args_hash);
    }
}
