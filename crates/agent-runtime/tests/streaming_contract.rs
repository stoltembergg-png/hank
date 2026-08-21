use agent_core::ids::{AgentId, SessionId};
use agent_core::session::{Message, MessageProvenance, MessageRole, MessageStatus};
use agent_runtime::execution::{Execution, ExecutionState};
use agent_runtime::provider_service::InvocationStreamEvent;
use agent_runtime::streaming::{StreamError, StreamEventConsumer};

fn setup() -> (Execution, Message) {
    let session_id = SessionId::new();
    let execution =
        Execution::new("exec-1", session_id, AgentId::new(), "corr-1", 1, 100, 1000).unwrap();
    let message = Message::new(
        session_id,
        MessageRole::Assistant,
        MessageProvenance::Provider,
        0,
        1,
        "",
    )
    .unwrap();
    (execution, message)
}

fn event(sequence: u64, text: &str, terminal: bool) -> InvocationStreamEvent {
    InvocationStreamEvent {
        attempt_id: "request-1:attempt_1".into(),
        sequence,
        text: text.into(),
        terminal,
    }
}

#[test]
fn ordered_stream_appends_deltas_and_exactly_one_terminal() {
    let (mut execution, mut message) = setup();
    let outcome = StreamEventConsumer::apply(
        &mut execution,
        &mut message,
        vec![event(0, "hello ", false), event(1, "world", true)],
        1,
    )
    .unwrap();
    assert_eq!(outcome.delta_count, 2);
    assert_eq!(message.content, "hello world");
    assert_eq!(message.status, MessageStatus::Complete);
    assert_eq!(execution.state(), ExecutionState::Completed);
}

#[test]
fn duplicate_out_of_order_and_stale_events_fail_without_overwrite() {
    let (mut execution, mut message) = setup();
    assert!(matches!(
        StreamEventConsumer::apply(
            &mut execution,
            &mut message,
            vec![event(1, "late", true)],
            1
        ),
        Err(StreamError::OutOfOrder { .. })
    ));
    assert_eq!(message.status, MessageStatus::Draft);
    assert!(matches!(
        StreamEventConsumer::apply(
            &mut execution,
            &mut message,
            vec![event(0, "ok", false), event(0, "dup", true)],
            1
        ),
        Err(StreamError::DuplicateSequence)
    ));
    assert!(matches!(
        StreamEventConsumer::apply(
            &mut execution,
            &mut message,
            vec![event(0, "stale", true)],
            2
        ),
        Err(StreamError::StaleGeneration)
    ));
}

#[test]
fn missing_or_multiple_terminal_events_are_explicit() {
    let (mut execution, mut message) = setup();
    assert!(matches!(
        StreamEventConsumer::apply(
            &mut execution,
            &mut message,
            vec![event(0, "partial", false)],
            1
        ),
        Err(StreamError::Incomplete)
    ));
    assert_eq!(message.status, MessageStatus::Failed);
    let (mut execution, mut message) = setup();
    assert!(matches!(
        StreamEventConsumer::apply(
            &mut execution,
            &mut message,
            vec![event(0, "done", true), event(1, "late", true)],
            1
        ),
        Err(StreamError::MultipleTerminal)
    ));
}

#[test]
fn cancellation_and_payload_bounds_fail_closed() {
    let (mut execution, mut message) = setup();
    let token = provider_core::CancellationToken::new();
    token.cancel();
    assert!(matches!(
        StreamEventConsumer::apply_with_cancellation(
            &mut execution,
            &mut message,
            vec![],
            1,
            token
        ),
        Err(StreamError::Cancelled)
    ));
    let (mut execution, mut message) = setup();
    assert!(matches!(
        StreamEventConsumer::apply(
            &mut execution,
            &mut message,
            vec![event(0, "api_key=secret", true)],
            1
        ),
        Err(StreamError::InvalidPayload)
    ));
}

#[test]
fn consumer_preserves_attempt_identity_and_redacts_errors() {
    let (mut execution, mut message) = setup();
    let outcome =
        StreamEventConsumer::apply(&mut execution, &mut message, vec![event(0, "ok", true)], 1)
            .unwrap();
    assert_eq!(outcome.attempt_id, "request-1:attempt_1");
    let debug = format!("{outcome:?}");
    assert!(!debug.contains("api_key"));
}
