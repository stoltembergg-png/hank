use agent_protocol::chat_command::CallerIdentity;
use agent_protocol::chat_stream::{
    ChatErrorCode, ChatStreamEvent, ChatStreamPayload, ChatStreamQueue, ChatStreamSubscription,
    ChatStreamValidationError, ChatStreamValidator, ChatTerminalReason,
};
use agent_protocol::ids::{AgentId, ProjectId, SessionId};

fn subscription() -> ChatStreamSubscription {
    ChatStreamSubscription::new(
        "stream-1",
        "command-1",
        CallerIdentity::new("caller-1", "desktop").unwrap(),
        ProjectId::new(),
        AgentId::new(),
        SessionId::new(),
        3,
    )
    .unwrap()
}

fn event(
    subscription: &ChatStreamSubscription,
    sequence: u64,
    payload: ChatStreamPayload,
) -> ChatStreamEvent {
    ChatStreamEvent::new(subscription, sequence, payload).unwrap()
}

#[test]
fn ordered_stream_accepts_delta_and_exactly_one_terminal() {
    let subscription = subscription();
    let mut validator = ChatStreamValidator::new(subscription.clone()).unwrap();
    validator
        .accept(event(&subscription, 0, ChatStreamPayload::Start))
        .unwrap();
    validator
        .accept(event(
            &subscription,
            1,
            ChatStreamPayload::Delta {
                text: "hello".into(),
            },
        ))
        .unwrap();
    validator
        .accept(event(
            &subscription,
            2,
            ChatStreamPayload::Finish {
                reason: ChatTerminalReason::Completed,
            },
        ))
        .unwrap();
    assert!(validator.is_terminal());
    assert_eq!(
        validator.accept(event(
            &subscription,
            3,
            ChatStreamPayload::Delta {
                text: "late".into()
            },
        )),
        Err(ChatStreamValidationError::AfterTerminal)
    );
}

#[test]
fn foreign_stale_duplicate_and_out_of_order_events_fail_closed() {
    let subscription = subscription();
    let mut validator = ChatStreamValidator::new(subscription.clone()).unwrap();
    validator
        .accept(event(&subscription, 0, ChatStreamPayload::Start))
        .unwrap();

    let mut foreign = event(
        &subscription,
        1,
        ChatStreamPayload::Delta { text: "x".into() },
    );
    foreign.session_id = SessionId::new();
    assert_eq!(
        validator.accept(foreign),
        Err(ChatStreamValidationError::ForeignStream)
    );

    let mut stale = event(
        &subscription,
        1,
        ChatStreamPayload::Delta {
            text: "stale".into(),
        },
    );
    stale.generation = 2;
    assert_eq!(
        validator.accept(stale),
        Err(ChatStreamValidationError::StaleGeneration {
            expected: 3,
            actual: 2
        })
    );

    let gap = event(
        &subscription,
        2,
        ChatStreamPayload::Delta { text: "gap".into() },
    );
    assert_eq!(
        validator.accept(gap),
        Err(ChatStreamValidationError::OutOfOrder {
            expected: 1,
            actual: 2
        })
    );

    let duplicate = event(
        &subscription,
        0,
        ChatStreamPayload::Delta {
            text: "duplicate".into(),
        },
    );
    assert_eq!(
        validator.accept(duplicate),
        Err(ChatStreamValidationError::DuplicateSequence(0))
    );
}

#[test]
fn malformed_payload_and_terminal_errors_are_typed_and_redacted() {
    let subscription = subscription();
    let mut invalid = event(
        &subscription,
        0,
        ChatStreamPayload::Error {
            code: ChatErrorCode::ProviderFailure,
        },
    );
    invalid.schema_version = 2;
    assert_eq!(
        invalid.validate(),
        Err(ChatStreamValidationError::UnsupportedVersion)
    );

    let oversized = ChatStreamEvent::new(
        &subscription,
        0,
        ChatStreamPayload::Delta {
            text: "x".repeat(65_537),
        },
    );
    assert_eq!(
        oversized.unwrap_err(),
        ChatStreamValidationError::OversizedPayload
    );
    assert!(!format!("{invalid:?}").contains("api_key"));
}

#[test]
fn bounded_queue_rejects_deltas_but_preserves_terminal_delivery() {
    let subscription = subscription();
    let mut queue = ChatStreamQueue::new(2).unwrap();
    queue
        .push(event(&subscription, 0, ChatStreamPayload::Start))
        .unwrap();
    queue
        .push(event(
            &subscription,
            1,
            ChatStreamPayload::Delta { text: "one".into() },
        ))
        .unwrap();
    assert_eq!(
        queue.push(event(
            &subscription,
            2,
            ChatStreamPayload::Delta { text: "two".into() },
        )),
        Err(ChatStreamValidationError::Backpressure)
    );
    queue
        .push(event(
            &subscription,
            2,
            ChatStreamPayload::Finish {
                reason: ChatTerminalReason::Completed,
            },
        ))
        .unwrap();
    assert_eq!(queue.coalesced_count(), 1);
    assert!(queue.pop().unwrap().is_start());
    assert!(queue.pop().unwrap().is_terminal());
}
