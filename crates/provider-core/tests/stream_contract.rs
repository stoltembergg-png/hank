use provider_core::response::{
    FinishReason, OutputPart, OutputPartKind, ProviderErrorCode, ProviderErrorInfo, Usage,
};
use provider_core::stream::{
    StreamBuffer, StreamEvent, StreamEventPayload, StreamValidationError, StreamValidator,
    STREAM_SCHEMA_VERSION,
};
use provider_core::{ModelId, ProviderId};

fn event(payload: StreamEventPayload, sequence: u64) -> StreamEvent {
    StreamEvent {
        schema_version: STREAM_SCHEMA_VERSION,
        stream_id: "stream-1".into(),
        request_id: "req-1".into(),
        correlation_id: "corr-1".into(),
        generation: 1,
        sequence,
        payload,
    }
}

fn start() -> StreamEvent {
    event(
        StreamEventPayload::Start {
            provider_id: ProviderId::parse("mock-provider").unwrap(),
            model_id: ModelId::parse("mock-model").unwrap(),
        },
        0,
    )
}

fn delta(sequence: u64, content: &str) -> StreamEvent {
    event(
        StreamEventPayload::Delta {
            part: OutputPart {
                kind: OutputPartKind::Text,
                content: content.into(),
            },
        },
        sequence,
    )
}

#[test]
fn stream_validator_accepts_ordered_events_and_exactly_one_terminal() {
    let mut validator = StreamValidator::new("stream-1", 1).unwrap();
    validator.accept(start()).unwrap();
    validator.accept(delta(1, "hello")).unwrap();
    validator
        .accept(event(
            StreamEventPayload::Usage {
                usage: Usage {
                    input_tokens: 2,
                    output_tokens: 1,
                },
            },
            2,
        ))
        .unwrap();
    validator
        .accept(event(
            StreamEventPayload::Finish {
                reason: FinishReason::Stop,
            },
            3,
        ))
        .unwrap();
    assert!(validator.is_terminal());
    assert_eq!(
        validator.accept(delta(4, "late")),
        Err(StreamValidationError::AfterTerminal)
    );
}

#[test]
fn stream_validator_rejects_duplicate_out_of_order_and_stale_generation() {
    let mut validator = StreamValidator::new("stream-1", 4).unwrap();
    let mut generation_start = start();
    generation_start.generation = 4;
    validator.accept(generation_start).unwrap();

    let mut duplicate = delta(0, "duplicate");
    duplicate.generation = 4;
    assert_eq!(
        validator.accept(duplicate),
        Err(StreamValidationError::DuplicateSequence(0))
    );
    let mut gap = delta(2, "gap");
    gap.generation = 4;
    assert_eq!(
        validator.accept(gap),
        Err(StreamValidationError::OutOfOrder {
            expected: 1,
            actual: 2
        })
    );

    let mut stale = delta(1, "stale");
    stale.generation = 3;
    assert_eq!(
        validator.accept(stale),
        Err(StreamValidationError::StaleGeneration {
            expected: 4,
            actual: 3
        })
    );
}

#[test]
fn stream_validator_requires_start_and_rejects_second_terminal() {
    let mut validator = StreamValidator::new("stream-1", 1).unwrap();
    assert_eq!(
        validator.accept(delta(1, "before start")),
        Err(StreamValidationError::MustStart)
    );
    validator.accept(start()).unwrap();
    validator
        .accept(event(
            StreamEventPayload::Cancel {
                reason: "user".into(),
            },
            1,
        ))
        .unwrap();
    assert_eq!(
        validator.accept(event(
            StreamEventPayload::Error {
                error: ProviderErrorInfo {
                    code: ProviderErrorCode::Internal,
                    message: "provider failed".into(),
                    retryable: false,
                },
            },
            2
        )),
        Err(StreamValidationError::AfterTerminal)
    );
}

#[test]
fn stream_buffer_is_bounded_and_preserves_terminal_events() {
    let mut buffer = StreamBuffer::new(2).unwrap();
    buffer.push(start()).unwrap();
    buffer.push(delta(1, "one")).unwrap();
    assert_eq!(
        buffer.push(delta(2, "two")),
        Err(StreamValidationError::Backpressure)
    );
    assert_eq!(buffer.pop().unwrap().sequence, 0);
    buffer
        .push(event(
            StreamEventPayload::Finish {
                reason: FinishReason::Stop,
            },
            2,
        ))
        .unwrap();
    assert_eq!(buffer.pop().unwrap().sequence, 1);
    assert!(buffer.pop().unwrap().is_terminal());
}

#[test]
fn stream_payloads_are_bounded_and_tool_requests_are_metadata_only() {
    let mut validator = StreamValidator::new("stream-1", 1).unwrap();
    validator.accept(start()).unwrap();
    let mut oversized = delta(1, &"x".repeat(1_048_577));
    assert_eq!(
        validator.accept(oversized.clone()),
        Err(StreamValidationError::OversizedPayload)
    );

    oversized.payload = StreamEventPayload::ToolRequest {
        tool_id: "calendar".into(),
        capability_fingerprint: "cap_calendar".into(),
        context: Some("project scope".into()),
    };
    validator.accept(oversized).unwrap();
    assert!(!validator.is_terminal());
}

#[test]
fn malformed_schema_stream_id_and_secret_tool_metadata_fail_closed() {
    let mut invalid = start();
    invalid.schema_version = 2;
    assert_eq!(
        invalid.validate(),
        Err(StreamValidationError::UnsupportedVersion)
    );

    let mut invalid = start();
    invalid.stream_id.clear();
    assert_eq!(
        invalid.validate(),
        Err(StreamValidationError::InvalidIdentity)
    );

    let mut validator = StreamValidator::new("stream-1", 1).unwrap();
    validator.accept(start()).unwrap();
    let invalid_tool = event(
        StreamEventPayload::ToolRequest {
            tool_id: "calendar".into(),
            capability_fingerprint: "api_key=secret".into(),
            context: None,
        },
        1,
    );
    assert_eq!(
        validator.accept(invalid_tool),
        Err(StreamValidationError::ForbiddenMetadata)
    );
}
