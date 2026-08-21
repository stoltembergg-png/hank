//! Normalized provider stream consumer for Execution and Message state.

use crate::execution::{Execution, ExecutionError, ExecutionEvent};
use crate::provider_service::{
    InvocationError, InvocationRequest, InvocationStreamEvent, ProviderApplicationService,
};
use agent_core::session::{Message, MessageError, MessagePart, MessagePartKind};
use provider_core::CancellationToken;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StreamError {
    #[error("stream event sequence is duplicated")]
    DuplicateSequence,
    #[error("stream event sequence is out of order: expected {expected}, got {actual}")]
    OutOfOrder { expected: u64, actual: u64 },
    #[error("stream event generation is stale")]
    StaleGeneration,
    #[error("stream produced more than one terminal event")]
    MultipleTerminal,
    #[error("stream ended without a terminal event")]
    Incomplete,
    #[error("stream was cancelled")]
    Cancelled,
    #[error("stream payload is invalid")]
    InvalidPayload,
    #[error("provider stream failed")]
    ProviderFailed,
    #[error("execution state transition failed")]
    Execution,
    #[error("message state transition failed")]
    Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamOutcome {
    pub attempt_id: String,
    pub delta_count: u32,
    pub terminal_sequence: u64,
}

pub struct StreamEventConsumer;

impl StreamEventConsumer {
    pub fn apply(
        execution: &mut Execution,
        message: &mut Message,
        events: Vec<InvocationStreamEvent>,
        generation: u64,
    ) -> Result<StreamOutcome, StreamError> {
        Self::apply_with_cancellation(
            execution,
            message,
            events,
            generation,
            CancellationToken::new(),
        )
    }

    pub fn apply_with_cancellation(
        execution: &mut Execution,
        message: &mut Message,
        events: Vec<InvocationStreamEvent>,
        generation: u64,
        cancellation: CancellationToken,
    ) -> Result<StreamOutcome, StreamError> {
        execution
            .accept_generation(generation)
            .map_err(map_execution_error)?;
        if cancellation.is_cancelled() {
            execution
                .apply(ExecutionEvent::Cancelled)
                .map_err(map_execution_error)?;
            message.cancel().map_err(map_message_error)?;
            return Err(StreamError::Cancelled);
        }
        prevalidate_events(&events)?;
        execution
            .apply(ExecutionEvent::Start)
            .map_err(map_execution_error)?;
        let attempt_id = events
            .first()
            .map(|event| event.attempt_id.clone())
            .unwrap_or_else(|| "stream:attempt_1".into());
        execution
            .apply(ExecutionEvent::ProviderInvoked(attempt_id.clone()))
            .map_err(map_execution_error)?;
        execution
            .apply(ExecutionEvent::StreamStarted)
            .map_err(map_execution_error)?;
        message.start_stream().map_err(map_message_error)?;
        Self::consume_started(execution, message, events, cancellation, attempt_id)
    }

    pub async fn stream(
        service: &ProviderApplicationService,
        request: InvocationRequest,
        execution: &mut Execution,
        message: &mut Message,
        generation: u64,
    ) -> Result<StreamOutcome, StreamError> {
        execution
            .accept_generation(generation)
            .map_err(map_execution_error)?;
        if request.access.cancellation.is_cancelled() {
            execution
                .apply(ExecutionEvent::Cancelled)
                .map_err(map_execution_error)?;
            message.cancel().map_err(map_message_error)?;
            return Err(StreamError::Cancelled);
        }
        execution
            .apply(ExecutionEvent::Start)
            .map_err(map_execution_error)?;
        let invocation_id = request.normalized.request_id.clone();
        execution
            .apply(ExecutionEvent::ProviderInvoked(invocation_id))
            .map_err(map_execution_error)?;
        execution
            .apply(ExecutionEvent::StreamStarted)
            .map_err(map_execution_error)?;
        message.start_stream().map_err(map_message_error)?;
        let cancellation = request.access.cancellation.clone();
        let events = match service.stream(request).await {
            Ok(events) => events,
            Err(InvocationError::Cancelled) => {
                execution
                    .apply(ExecutionEvent::Cancelled)
                    .map_err(map_execution_error)?;
                message.cancel().map_err(map_message_error)?;
                return Err(StreamError::Cancelled);
            }
            Err(_) => {
                execution
                    .apply(ExecutionEvent::Failed("provider_error".into()))
                    .map_err(map_execution_error)?;
                message.fail("provider_error").map_err(map_message_error)?;
                return Err(StreamError::ProviderFailed);
            }
        };
        let attempt_id = events
            .first()
            .map(|event| event.attempt_id.clone())
            .unwrap_or_else(|| "stream:attempt_1".into());
        Self::consume_started(execution, message, events, cancellation, attempt_id)
    }

    fn consume_started(
        execution: &mut Execution,
        message: &mut Message,
        events: Vec<InvocationStreamEvent>,
        cancellation: CancellationToken,
        attempt_id: String,
    ) -> Result<StreamOutcome, StreamError> {
        let mut delta_count = 0;
        let mut terminal_sequence = None;
        for (index, event) in events.into_iter().enumerate() {
            let expected_sequence = index as u64;
            if cancellation.is_cancelled() {
                execution
                    .apply(ExecutionEvent::Cancelled)
                    .map_err(map_execution_error)?;
                message.cancel().map_err(map_message_error)?;
                return Err(StreamError::Cancelled);
            }
            if terminal_sequence.is_some() {
                return Err(StreamError::MultipleTerminal);
            }
            if event.sequence < expected_sequence {
                return Err(StreamError::DuplicateSequence);
            }
            if event.sequence > expected_sequence {
                return Err(StreamError::OutOfOrder {
                    expected: expected_sequence,
                    actual: event.sequence,
                });
            }
            let part = MessagePart::new(MessagePartKind::Text, event.text, true)
                .map_err(|_| StreamError::InvalidPayload)?;
            message.add_part(part).map_err(map_message_error)?;
            delta_count += 1;
            if event.terminal {
                message.complete().map_err(map_message_error)?;
                execution
                    .apply(ExecutionEvent::Completed)
                    .map_err(map_execution_error)?;
                terminal_sequence = Some(event.sequence);
            }
        }
        let terminal_sequence = match terminal_sequence {
            Some(sequence) => sequence,
            None => {
                message
                    .fail("stream_incomplete")
                    .map_err(map_message_error)?;
                execution
                    .apply(ExecutionEvent::Failed("stream_incomplete".into()))
                    .map_err(map_execution_error)?;
                return Err(StreamError::Incomplete);
            }
        };
        Ok(StreamOutcome {
            attempt_id,
            delta_count,
            terminal_sequence,
        })
    }
}

fn prevalidate_events(events: &[InvocationStreamEvent]) -> Result<(), StreamError> {
    let mut terminal_seen = false;
    for (index, event) in events.iter().enumerate() {
        let expected_sequence = index as u64;
        if terminal_seen {
            return Err(StreamError::MultipleTerminal);
        }
        if event.sequence < expected_sequence {
            return Err(StreamError::DuplicateSequence);
        }
        if event.sequence > expected_sequence {
            return Err(StreamError::OutOfOrder {
                expected: expected_sequence,
                actual: event.sequence,
            });
        }
        MessagePart::new(MessagePartKind::Text, event.text.clone(), true)
            .map_err(|_| StreamError::InvalidPayload)?;
        terminal_seen = event.terminal;
    }
    Ok(())
}

fn map_execution_error(error: ExecutionError) -> StreamError {
    match error {
        ExecutionError::StaleGeneration => StreamError::StaleGeneration,
        ExecutionError::TerminalState => StreamError::MultipleTerminal,
        _ => StreamError::Execution,
    }
}

fn map_message_error(error: MessageError) -> StreamError {
    match error {
        MessageError::ForbiddenContent | MessageError::PartLimit => StreamError::InvalidPayload,
        _ => StreamError::Message,
    }
}
