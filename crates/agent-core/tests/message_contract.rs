use agent_core::session::{
    Message, MessageError, MessageOrdering, MessagePart, MessagePartKind, MessageProvenance,
    MessageRole, MessageStatus,
};
use agent_protocol::ids::SessionId;
use serde_json::json;

fn session() -> SessionId {
    SessionId::new()
}

fn message(session_id: SessionId, sequence: u64, generation: u64) -> Message {
    Message::new(
        session_id,
        MessageRole::User,
        MessageProvenance::User,
        sequence,
        generation,
        "hello",
    )
    .unwrap()
}

#[test]
fn message_marks_untrusted_provenance_and_has_bounded_parts() {
    let message = message(session(), 0, 1);
    assert_eq!(message.status, MessageStatus::Draft);
    assert_eq!(message.provenance, MessageProvenance::User);
    assert_eq!(message.parts.len(), 1);
    assert!(message.parts[0].untrusted);
    assert!(Message::new(
        session(),
        MessageRole::User,
        MessageProvenance::User,
        0,
        1,
        "x".repeat(1_048_577)
    )
    .is_err());
}

#[test]
fn provider_and_tool_content_are_untrusted_but_role_does_not_change_precedence() {
    let provider = Message::new(
        session(),
        MessageRole::System,
        MessageProvenance::Provider,
        0,
        1,
        "instruction-like provider text",
    )
    .unwrap();
    assert!(provider.parts[0].untrusted);
    assert_eq!(provider.role, MessageRole::System);
    let tool_part = MessagePart::new(MessagePartKind::ToolResult, "tool output", true).unwrap();
    assert!(tool_part.untrusted);
    assert!(MessagePart::new(MessagePartKind::Text, "x".repeat(1_048_577), true).is_err());
}

#[test]
fn message_state_transitions_are_terminal_and_idempotent() {
    let mut message = message(session(), 0, 1);
    message.start_stream().unwrap();
    assert_eq!(message.status, MessageStatus::Streaming);
    message.complete().unwrap();
    assert_eq!(message.status, MessageStatus::Complete);
    message.complete().unwrap();
    assert!(matches!(
        message.start_stream(),
        Err(MessageError::Terminal)
    ));
    assert!(matches!(message.fail("late"), Err(MessageError::Terminal)));
}

#[test]
fn invalid_message_transitions_fail_without_mutation() {
    let mut message = message(session(), 0, 1);
    assert!(matches!(
        message.complete(),
        Err(MessageError::InvalidTransition { .. })
    ));
    assert_eq!(message.status, MessageStatus::Draft);
    message.fail("provider outage").unwrap();
    assert_eq!(message.status, MessageStatus::Failed);
    assert!(message.cancel().is_err());
}

#[test]
fn ordering_rejects_cross_session_stale_duplicate_and_out_of_order_messages() {
    let session_id = session();
    let other_session = session();
    let mut ordering = MessageOrdering::new(session_id, 2).unwrap();
    assert!(matches!(
        ordering.accept(message(other_session, 0, 2)),
        Err(MessageError::SessionMismatch)
    ));
    assert!(matches!(
        ordering.accept(message(session_id, 1, 2)),
        Err(MessageError::OutOfOrder { .. })
    ));
    ordering.accept(message(session_id, 0, 2)).unwrap();
    let mut second = message(session_id, 1, 2);
    second.start_stream().unwrap();
    second.complete().unwrap();
    ordering.accept(second).unwrap();
    assert!(matches!(
        ordering.accept(message(session_id, 2, 1)),
        Err(MessageError::StaleGeneration { .. })
    ));
    assert!(matches!(
        ordering.accept(message(session_id, 2, 2)),
        Err(MessageError::AfterTerminal)
    ));
}

#[test]
fn ordering_rejects_duplicate_sequence_and_preserves_terminality() {
    let session_id = session();
    let mut ordering = MessageOrdering::new(session_id, 1).unwrap();
    ordering.accept(message(session_id, 0, 1)).unwrap();
    assert!(matches!(
        ordering.accept(message(session_id, 0, 1)),
        Err(MessageError::DuplicateSequence(0))
    ));
    let mut terminal = message(session_id, 1, 1);
    terminal.start_stream().unwrap();
    terminal.complete().unwrap();
    ordering.accept(terminal).unwrap();
    assert!(ordering.is_terminal());
}

#[test]
fn serde_roundtrip_rejects_unknown_role_and_preserves_provenance() {
    let message = message(session(), 0, 1);
    let encoded = serde_json::to_value(&message).unwrap();
    let decoded: Message = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.provenance, MessageProvenance::User);
    let mut invalid = serde_json::to_value(&message).unwrap();
    invalid["role"] = json!("unknown_role");
    assert!(serde_json::from_value::<Message>(invalid).is_err());
}

#[test]
fn correlation_and_parts_metadata_are_bounded_and_redacted() {
    let mut message = message(session(), 0, 1);
    message.correlation_id = "corr_1".into();
    assert!(message
        .add_part(MessagePart::new(MessagePartKind::Text, "more", true).unwrap())
        .is_ok());
    assert!(MessagePart::new(MessagePartKind::Text, "api_key=secret", true).is_err());
    message.correlation_id = "x".repeat(129);
    assert!(message.validate().is_err());
    assert!(!format!("{message:?}").contains("api_key"));
}
