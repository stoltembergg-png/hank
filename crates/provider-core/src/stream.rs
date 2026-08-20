//! Provider-neutral streaming event contract.

use crate::response::{FinishReason, OutputPart, ProviderErrorInfo, Usage};
use crate::{ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

pub const STREAM_SCHEMA_VERSION: u32 = 1;
const MAX_STREAM_ID_LEN: usize = 128;
const MAX_REQUEST_ID_LEN: usize = 128;
const MAX_CORRELATION_ID_LEN: usize = 128;
const MAX_TOOL_ID_LEN: usize = 128;
const MAX_FINGERPRINT_LEN: usize = 128;
const MAX_CONTEXT_LEN: usize = 512;
const MAX_CANCEL_REASON_LEN: usize = 256;
const MAX_BUFFERED_EVENTS: usize = 1024;
const MAX_EVENT_PAYLOAD_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEventPayload {
    Start {
        provider_id: ProviderId,
        model_id: ModelId,
    },
    Delta {
        part: OutputPart,
    },
    ToolRequest {
        tool_id: String,
        capability_fingerprint: String,
        context: Option<String>,
    },
    Usage {
        usage: Usage,
    },
    Finish {
        reason: FinishReason,
    },
    Error {
        error: ProviderErrorInfo,
    },
    Cancel {
        reason: String,
    },
    #[serde(other)]
    Unknown,
}

impl StreamEventPayload {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Finish { .. } | Self::Error { .. } | Self::Cancel { .. }
        )
    }

    fn validate(&self) -> Result<(), StreamValidationError> {
        match self {
            Self::Delta { part } => {
                if part.content.len() > MAX_EVENT_PAYLOAD_BYTES {
                    return Err(StreamValidationError::OversizedPayload);
                }
            }
            Self::ToolRequest {
                tool_id,
                capability_fingerprint,
                context,
            } => {
                if contains_forbidden_marker(capability_fingerprint) {
                    return Err(StreamValidationError::ForbiddenMetadata);
                }
                if !valid_text(tool_id, MAX_TOOL_ID_LEN)
                    || !valid_text(capability_fingerprint, MAX_FINGERPRINT_LEN)
                    || !capability_fingerprint.starts_with("cap_")
                {
                    return Err(StreamValidationError::InvalidPayload);
                }
                if context
                    .as_deref()
                    .is_some_and(|value| !valid_text(value, MAX_CONTEXT_LEN))
                {
                    return Err(StreamValidationError::InvalidPayload);
                }
            }
            Self::Error { error } => {
                if error.message.trim().is_empty()
                    || error.message.len() > MAX_EVENT_PAYLOAD_BYTES
                    || contains_forbidden_marker(&error.message)
                {
                    return Err(StreamValidationError::ForbiddenMetadata);
                }
            }
            Self::Cancel { reason } => {
                if !valid_text(reason, MAX_CANCEL_REASON_LEN) {
                    return Err(StreamValidationError::InvalidPayload);
                }
            }
            Self::Start { .. } | Self::Usage { .. } | Self::Finish { .. } => {}
            Self::Unknown => return Err(StreamValidationError::UnknownPayload),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamEvent {
    pub schema_version: u32,
    pub stream_id: String,
    pub request_id: String,
    pub correlation_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub payload: StreamEventPayload,
}

impl StreamEvent {
    pub fn validate(&self) -> Result<(), StreamValidationError> {
        if self.schema_version != STREAM_SCHEMA_VERSION {
            return Err(StreamValidationError::UnsupportedVersion);
        }
        if !valid_text(&self.stream_id, MAX_STREAM_ID_LEN)
            || !valid_text(&self.request_id, MAX_REQUEST_ID_LEN)
            || !valid_text(&self.correlation_id, MAX_CORRELATION_ID_LEN)
            || self.generation == 0
        {
            return Err(StreamValidationError::InvalidIdentity);
        }
        self.payload.validate()
    }

    pub fn is_terminal(&self) -> bool {
        self.payload.is_terminal()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StreamValidationError {
    #[error("unsupported stream schema version")]
    UnsupportedVersion,
    #[error("stream identity is invalid")]
    InvalidIdentity,
    #[error("stream payload is invalid")]
    InvalidPayload,
    #[error("stream payload exceeds bounded size")]
    OversizedPayload,
    #[error("stream payload contains forbidden metadata")]
    ForbiddenMetadata,
    #[error("unknown stream payload")]
    UnknownPayload,
    #[error("stream must start with a start event")]
    MustStart,
    #[error("stream already started")]
    AlreadyStarted,
    #[error("stream sequence was duplicated: {0}")]
    DuplicateSequence(u64),
    #[error("stream sequence is out of order: expected {expected}, got {actual}")]
    OutOfOrder { expected: u64, actual: u64 },
    #[error("stream generation is stale: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("stream generation is ahead: expected {expected}, got {actual}")]
    FutureGeneration { expected: u64, actual: u64 },
    #[error("stream already has a terminal event")]
    AfterTerminal,
    #[error("stream buffer is full")]
    Backpressure,
    #[error("stream buffer is empty")]
    EmptyBuffer,
}

#[derive(Debug, Clone)]
pub struct StreamValidator {
    stream_id: String,
    generation: u64,
    next_sequence: u64,
    started: bool,
    terminal: bool,
}

impl StreamValidator {
    pub fn new(
        stream_id: impl Into<String>,
        generation: u64,
    ) -> Result<Self, StreamValidationError> {
        let stream_id = stream_id.into();
        if !valid_text(&stream_id, MAX_STREAM_ID_LEN) || generation == 0 {
            return Err(StreamValidationError::InvalidIdentity);
        }
        Ok(Self {
            stream_id,
            generation,
            next_sequence: 0,
            started: false,
            terminal: false,
        })
    }

    pub fn accept(&mut self, event: StreamEvent) -> Result<(), StreamValidationError> {
        event.validate()?;
        if event.stream_id != self.stream_id {
            return Err(StreamValidationError::InvalidIdentity);
        }
        if event.generation < self.generation {
            return Err(StreamValidationError::StaleGeneration {
                expected: self.generation,
                actual: event.generation,
            });
        }
        if event.generation > self.generation {
            return Err(StreamValidationError::FutureGeneration {
                expected: self.generation,
                actual: event.generation,
            });
        }
        if self.terminal {
            return Err(StreamValidationError::AfterTerminal);
        }
        if !self.started && !matches!(event.payload, StreamEventPayload::Start { .. }) {
            return Err(StreamValidationError::MustStart);
        }
        if self.started && matches!(event.payload, StreamEventPayload::Start { .. }) {
            return Err(StreamValidationError::AlreadyStarted);
        }
        if event.sequence < self.next_sequence {
            return Err(StreamValidationError::DuplicateSequence(event.sequence));
        }
        if event.sequence > self.next_sequence {
            return Err(StreamValidationError::OutOfOrder {
                expected: self.next_sequence,
                actual: event.sequence,
            });
        }
        if matches!(event.payload, StreamEventPayload::Start { .. }) && event.sequence != 0 {
            return Err(StreamValidationError::OutOfOrder {
                expected: 0,
                actual: event.sequence,
            });
        }

        self.started = true;
        self.terminal = event.is_terminal();
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(StreamValidationError::InvalidPayload)?;
        Ok(())
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
}

#[derive(Debug, Clone)]
pub struct StreamBuffer {
    max_events: usize,
    events: VecDeque<StreamEvent>,
}

impl StreamBuffer {
    pub fn new(max_events: usize) -> Result<Self, StreamValidationError> {
        if !(1..=MAX_BUFFERED_EVENTS).contains(&max_events) {
            return Err(StreamValidationError::Backpressure);
        }
        Ok(Self {
            max_events,
            events: VecDeque::with_capacity(max_events),
        })
    }

    pub fn push(&mut self, event: StreamEvent) -> Result<(), StreamValidationError> {
        event.validate()?;
        if self.events.len() >= self.max_events {
            return Err(StreamValidationError::Backpressure);
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<StreamEvent> {
        self.events.pop_front()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

fn valid_text(value: &str, max_len: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_len && !value.chars().any(char::is_control)
}

fn contains_forbidden_marker(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "api_key",
        "authorization:",
        "password",
        "secret",
        "token",
        "bearer",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}
