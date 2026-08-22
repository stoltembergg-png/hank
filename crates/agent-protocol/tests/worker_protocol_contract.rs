use agent_protocol::capability::{Action, Capability, Resource};
use agent_protocol::envelope::TerminalResult;
use agent_protocol::ids::{ProjectId, RequestId, SessionId, TraceId};
use agent_protocol::worker::{
    WorkerContext, WorkerErrorCode, WorkerErrorDetail, WorkerHealthStatus, WorkerMessage,
    WorkerProtocolError, WorkerSession, WORKER_PROTOCOL_SCHEMA_VERSION,
};
use serde_json::json;

fn context() -> WorkerContext {
    WorkerContext {
        project_id: ProjectId::parse("proj-00000000-0000-4000-8000-000000000301")
            .expect("fixture project id"),
        session_id: SessionId::parse("sess-00000000-0000-4000-8000-000000000302")
            .expect("fixture session id"),
        task_id: None,
        trace_id: TraceId::new(),
    }
}

fn handshake() -> WorkerMessage {
    WorkerMessage::Handshake {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        worker_id: "worker-python-1".to_string(),
        protocol_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        capabilities: vec![Capability::new(Resource::Tool, Action::Execute)],
    }
}

fn handshake_accepted() -> WorkerMessage {
    WorkerMessage::HandshakeAccepted {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        worker_id: "worker-python-1".to_string(),
        protocol_version: WORKER_PROTOCOL_SCHEMA_VERSION,
    }
}

fn request(id: RequestId, ctx: &WorkerContext) -> WorkerMessage {
    WorkerMessage::Request {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        request_id: id,
        context: *ctx,
        capability: Capability::new(Resource::Tool, Action::Execute),
        payload: json!({"task": "summarize", "input": {"rows": 3}}),
    }
}

fn response(id: RequestId, ctx: &WorkerContext) -> WorkerMessage {
    WorkerMessage::Response {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        request_id: id,
        context: *ctx,
        result: TerminalResult::Succeeded,
        value: Some(json!({"summary": "ok"})),
        error: None,
    }
}

#[test]
// @spec:AC-677
fn handshake_lifecycle_happy_path_with_deterministic_serialization() {
    let mut session = WorkerSession::new();
    let ctx = context();
    let request_id = RequestId::new();

    session
        .accept(handshake())
        .expect("handshake must open the session");
    session
        .accept(handshake_accepted())
        .expect("accept must complete the handshake");
    session
        .accept(request(request_id, &ctx))
        .expect("request must register");
    session
        .accept(response(request_id, &ctx))
        .expect("response must correlate");
    session
        .accept(WorkerMessage::Health {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        })
        .expect("health probe must be accepted");
    session
        .accept(WorkerMessage::HealthReport {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            worker_id: "worker-python-1".to_string(),
            status: WorkerHealthStatus::Healthy,
        })
        .expect("health report must be accepted");
    session
        .accept(WorkerMessage::Shutdown {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            reason: agent_protocol::worker::WorkerShutdownReason::User,
        })
        .expect("shutdown must be accepted");
    session
        .accept(WorkerMessage::ShutdownAck {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        })
        .expect("shutdown ack must close the session");

    assert!(session.is_shutdown());
    assert_eq!(session.pending_len(), 0);

    let first = serde_json::to_string(&handshake()).expect("handshake serializes");
    let second = serde_json::to_string(&handshake()).expect("handshake serializes again");
    assert_eq!(first, second, "serialization must be deterministic");
    assert!(first.contains("\"kind\":\"handshake\""));
    assert!(first.contains("\"schema_version\":1"));
}

#[test]
// @spec:AC-678
fn protocol_ordering_fails_closed() {
    let mut session = WorkerSession::new();
    let ctx = context();

    assert_eq!(
        session.accept(request(RequestId::new(), &ctx)),
        Err(WorkerProtocolError::NotHandshaked),
        "requests before handshake must fail"
    );
    assert_eq!(
        session.accept(handshake_accepted()),
        Err(WorkerProtocolError::NotHandshaked),
        "accept before handshake must fail"
    );

    session.accept(handshake()).expect("handshake");
    assert_eq!(
        session.accept(handshake()),
        Err(WorkerProtocolError::AlreadyHandshaked),
        "second handshake must fail"
    );

    session.accept(handshake_accepted()).expect("accepted");
    session
        .accept(WorkerMessage::Shutdown {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            reason: agent_protocol::worker::WorkerShutdownReason::Timeout,
        })
        .expect("shutdown");
    assert_eq!(
        session.accept(request(RequestId::new(), &ctx)),
        Err(WorkerProtocolError::AfterShutdown),
        "requests after shutdown must fail"
    );
    session
        .accept(WorkerMessage::ShutdownAck {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        })
        .expect("ack");
    assert_eq!(
        session.accept(handshake()),
        Err(WorkerProtocolError::AfterShutdown),
        "handshake after terminal shutdown must fail"
    );
}

#[test]
// @spec:AC-679
fn request_correlation_rejects_unknown_and_duplicate_ids() {
    let mut session = WorkerSession::new();
    let ctx = context();
    session.accept(handshake()).expect("handshake");
    session.accept(handshake_accepted()).expect("accepted");

    let request_id = RequestId::new();
    session
        .accept(request(request_id, &ctx))
        .expect("request registers");

    assert_eq!(
        session.accept(request(request_id, &ctx)),
        Err(WorkerProtocolError::DuplicateRequest),
        "duplicate request id must fail"
    );
    assert_eq!(
        session.accept(response(RequestId::new(), &ctx)),
        Err(WorkerProtocolError::UnknownRequest),
        "response for unknown request must fail"
    );
    assert_eq!(
        session.accept(WorkerMessage::Cancel {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            request_id: RequestId::new(),
            reason: agent_protocol::worker::WorkerCancelReason::User,
        }),
        Err(WorkerProtocolError::UnknownRequest),
        "cancel for unknown request must fail"
    );

    session
        .accept(response(request_id, &ctx))
        .expect("response correlates");
    assert_eq!(
        session.accept(response(request_id, &ctx)),
        Err(WorkerProtocolError::UnknownRequest),
        "response replay must fail"
    );
}

#[test]
// @spec:AC-680
fn responses_must_preserve_the_exact_request_context() {
    let mut session = WorkerSession::new();
    let ctx = context();
    session.accept(handshake()).expect("handshake");
    session.accept(handshake_accepted()).expect("accepted");

    let request_id = RequestId::new();
    session.accept(request(request_id, &ctx)).expect("request");

    let mut foreign = ctx;
    foreign.project_id =
        ProjectId::parse("proj-00000000-0000-4000-8000-000000000309").expect("foreign project id");
    assert_eq!(
        session.accept(response(request_id, &foreign)),
        Err(WorkerProtocolError::ContextMismatch),
        "foreign project must fail closed"
    );

    let mut other_session = ctx;
    other_session.session_id =
        SessionId::parse("sess-00000000-0000-4000-8000-000000000310").expect("foreign session id");
    assert_eq!(
        session.accept(response(request_id, &other_session)),
        Err(WorkerProtocolError::ContextMismatch),
        "foreign session must fail closed"
    );
}

#[test]
// @spec:AC-681
fn bounds_version_and_capacity_fail_closed() {
    let mut session = WorkerSession::new();
    let ctx = context();
    session.accept(handshake()).expect("handshake");
    session.accept(handshake_accepted()).expect("accepted");

    let oversized = WorkerMessage::Request {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        request_id: RequestId::new(),
        context: ctx,
        capability: Capability::new(Resource::Tool, Action::Execute),
        payload: json!({"blob": "x".repeat(65_537)}),
    };
    assert_eq!(
        session.accept(oversized),
        Err(WorkerProtocolError::OversizedPayload),
        "payload above the bound must fail"
    );

    let stale = WorkerMessage::Handshake {
        schema_version: 0,
        worker_id: "worker-python-1".to_string(),
        protocol_version: 0,
        capabilities: vec![Capability::new(Resource::Tool, Action::Execute)],
    };
    assert_eq!(
        session.accept(stale),
        Err(WorkerProtocolError::UnsupportedVersion),
        "unsupported schema version must fail"
    );

    let empty_capabilities = WorkerMessage::Handshake {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        worker_id: "worker-python-1".to_string(),
        protocol_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        capabilities: Vec::new(),
    };
    assert_eq!(
        empty_capabilities.validate(),
        Err(WorkerProtocolError::InvalidPayload),
        "handshake without capabilities must fail"
    );

    for _ in 0..256 {
        session
            .accept(request(RequestId::new(), &ctx))
            .expect("bounded pending requests must register");
    }
    assert_eq!(
        session.accept(request(RequestId::new(), &ctx)),
        Err(WorkerProtocolError::Backpressure),
        "pending capacity must fail closed"
    );
}

#[test]
// @spec:AC-682
fn error_and_cancel_carry_no_executable_instruction_or_secret() {
    let mut session = WorkerSession::new();
    let ctx = context();
    session.accept(handshake()).expect("handshake");
    session.accept(handshake_accepted()).expect("accepted");

    let invalid_detail = WorkerMessage::Error {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        code: WorkerErrorCode::InternalError,
        detail: "x".repeat(257),
    };
    assert_eq!(
        invalid_detail.validate(),
        Err(WorkerProtocolError::InvalidPayload),
        "oversized error detail must fail"
    );

    let succeeded_with_error = WorkerMessage::Response {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        request_id: RequestId::new(),
        context: ctx,
        result: TerminalResult::Succeeded,
        value: Some(json!({"ok": true})),
        error: Some(WorkerErrorDetail {
            code: WorkerErrorCode::InternalError,
            detail: "must not carry error on success".to_string(),
        }),
    };
    assert_eq!(
        succeeded_with_error.validate(),
        Err(WorkerProtocolError::InvalidPayload),
        "success with error detail must fail"
    );

    let rejected_without_detail = WorkerMessage::Response {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        request_id: RequestId::new(),
        context: ctx,
        result: TerminalResult::Rejected,
        value: None,
        error: None,
    };
    assert_eq!(
        rejected_without_detail.validate(),
        Err(WorkerProtocolError::InvalidPayload),
        "rejection without bounded detail must fail"
    );

    let cancel = WorkerMessage::Cancel {
        schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        request_id: RequestId::new(),
        reason: agent_protocol::worker::WorkerCancelReason::Deadline,
    };
    cancel
        .validate()
        .expect("cancel carries only a bounded reason");
    let serialized = serde_json::to_string(&cancel).expect("cancel serializes");
    assert!(
        !serialized.contains("exec"),
        "cancel must not embed instructions: {serialized}"
    );
    assert!(
        !serialized.contains("cmd"),
        "cancel must not embed commands: {serialized}"
    );

    let mismatch = WorkerProtocolError::ContextMismatch;
    let rendered = format!("{mismatch:?}");
    assert!(
        !rendered.to_lowercase().contains("secret"),
        "errors must be redacted"
    );
}
