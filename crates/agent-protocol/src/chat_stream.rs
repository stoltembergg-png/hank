//! Typed, bounded chat stream protocol for the desktop event bridge.

use crate::chat_command::CallerIdentity;
use crate::ids::{AgentId, ProjectId, SessionId};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

pub const CHAT_STREAM_SCHEMA_VERSION: u32 = 1;
pub const CHAT_STREAM_EVENT_NAME: &str = "hank://chat/stream";
const MAX_ID_LEN: usize = 128;
const MAX_DELTA_BYTES: usize = 65_536;
const MAX_QUEUE_EVENTS: usize = 256;
const MAX_CANCEL_REASON_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatTerminalReason {
    Completed,
    Length,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatErrorCode {
    Unauthorized,
    ProviderFailure,
    BudgetExceeded,
    InvalidStream,
    Backpressure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatCancelReason {
    User,
    SessionClosed,
    Deadline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatStreamPayload {
    Start,
    Delta {
        text: String,
    },
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    Finish {
        reason: ChatTerminalReason,
    },
    Error {
        code: ChatErrorCode,
    },
    Cancel {
        reason: ChatCancelReason,
    },
}

impl ChatStreamPayload {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Finish { .. } | Self::Error { .. } | Self::Cancel { .. }
        )
    }

    fn validate(&self) -> Result<(), ChatStreamValidationError> {
        match self {
            Self::Delta { text } => {
                if text.len() > MAX_DELTA_BYTES {
                    return Err(ChatStreamValidationError::OversizedPayload);
                }
                if text.is_empty() || text.chars().any(|character| character == '\0') {
                    return Err(ChatStreamValidationError::InvalidPayload);
                }
            }
            Self::Cancel { .. } | Self::Finish { .. } | Self::Error { .. } => {}
            Self::Usage {
                input_tokens,
                output_tokens,
            } => {
                if u64::from(*input_tokens) + u64::from(*output_tokens) == 0 {
                    return Err(ChatStreamValidationError::InvalidPayload);
                }
            }
            Self::Start => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatStreamSubscription {
    pub stream_id: String,
    pub command_id: String,
    pub caller: CallerIdentity,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub generation: u64,
}

impl ChatStreamSubscription {
    pub fn new(
        stream_id: impl Into<String>,
        command_id: impl Into<String>,
        caller: CallerIdentity,
        project_id: ProjectId,
        agent_id: AgentId,
        session_id: SessionId,
        generation: u64,
    ) -> Result<Self, ChatStreamValidationError> {
        let stream_id = stream_id.into();
        let command_id = command_id.into();
        if !valid_id(&stream_id) || !valid_id(&command_id) || generation == 0 {
            return Err(ChatStreamValidationError::InvalidIdentity);
        }
        Ok(Self {
            stream_id,
            command_id,
            caller,
            project_id,
            agent_id,
            session_id,
            generation,
        })
    }

    fn matches(&self, event: &ChatStreamEvent) -> bool {
        self.stream_id == event.stream_id
            && self.command_id == event.command_id
            && self.caller == event.caller
            && self.project_id == event.project_id
            && self.agent_id == event.agent_id
            && self.session_id == event.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatStreamEvent {
    pub schema_version: u32,
    pub stream_id: String,
    pub command_id: String,
    pub caller: CallerIdentity,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub generation: u64,
    pub sequence: u64,
    pub payload: ChatStreamPayload,
}

impl ChatStreamEvent {
    pub fn new(
        subscription: &ChatStreamSubscription,
        sequence: u64,
        payload: ChatStreamPayload,
    ) -> Result<Self, ChatStreamValidationError> {
        let event = Self {
            schema_version: CHAT_STREAM_SCHEMA_VERSION,
            stream_id: subscription.stream_id.clone(),
            command_id: subscription.command_id.clone(),
            caller: subscription.caller.clone(),
            project_id: subscription.project_id,
            agent_id: subscription.agent_id,
            session_id: subscription.session_id,
            generation: subscription.generation,
            sequence,
            payload,
        };
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> Result<(), ChatStreamValidationError> {
        if self.schema_version != CHAT_STREAM_SCHEMA_VERSION {
            return Err(ChatStreamValidationError::UnsupportedVersion);
        }
        if !valid_id(&self.stream_id) || !valid_id(&self.command_id) || self.generation == 0 {
            return Err(ChatStreamValidationError::InvalidIdentity);
        }
        self.payload.validate()
    }

    pub fn is_terminal(&self) -> bool {
        self.payload.is_terminal()
    }

    pub fn is_start(&self) -> bool {
        matches!(self.payload, ChatStreamPayload::Start)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChatStreamValidationError {
    #[error("unsupported chat stream schema version")]
    UnsupportedVersion,
    #[error("chat stream identity is invalid")]
    InvalidIdentity,
    #[error("chat stream is not authorized for this subscription")]
    ForeignStream,
    #[error("chat stream payload is invalid")]
    InvalidPayload,
    #[error("chat stream payload exceeds the bounded size")]
    OversizedPayload,
    #[error("chat stream generation is stale: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("chat stream generation is from the future: expected {expected}, got {actual}")]
    FutureGeneration { expected: u64, actual: u64 },
    #[error("chat stream must start with a start event")]
    MustStart,
    #[error("chat stream already started")]
    AlreadyStarted,
    #[error("chat stream sequence duplicated: {0}")]
    DuplicateSequence(u64),
    #[error("chat stream sequence out of order: expected {expected}, got {actual}")]
    OutOfOrder { expected: u64, actual: u64 },
    #[error("chat stream already has a terminal event")]
    AfterTerminal,
    #[error("chat stream buffer is full")]
    Backpressure,
    #[error("chat stream buffer size is invalid")]
    InvalidBuffer,
}

#[derive(Debug, Clone)]
pub struct ChatStreamValidator {
    subscription: ChatStreamSubscription,
    next_sequence: u64,
    started: bool,
    terminal: bool,
}

impl ChatStreamValidator {
    pub fn new(subscription: ChatStreamSubscription) -> Result<Self, ChatStreamValidationError> {
        if subscription.generation == 0
            || !valid_id(&subscription.stream_id)
            || !valid_id(&subscription.command_id)
        {
            return Err(ChatStreamValidationError::InvalidIdentity);
        }
        Ok(Self {
            subscription,
            next_sequence: 0,
            started: false,
            terminal: false,
        })
    }

    pub fn accept(&mut self, event: ChatStreamEvent) -> Result<(), ChatStreamValidationError> {
        event.validate()?;
        if !self.subscription.matches(&event) {
            return Err(ChatStreamValidationError::ForeignStream);
        }
        if event.generation < self.subscription.generation {
            return Err(ChatStreamValidationError::StaleGeneration {
                expected: self.subscription.generation,
                actual: event.generation,
            });
        }
        if event.generation > self.subscription.generation {
            return Err(ChatStreamValidationError::FutureGeneration {
                expected: self.subscription.generation,
                actual: event.generation,
            });
        }
        if self.terminal {
            return Err(ChatStreamValidationError::AfterTerminal);
        }
        if event.sequence < self.next_sequence {
            return Err(ChatStreamValidationError::DuplicateSequence(event.sequence));
        }
        if event.sequence > self.next_sequence {
            return Err(ChatStreamValidationError::OutOfOrder {
                expected: self.next_sequence,
                actual: event.sequence,
            });
        }
        if !self.started && !event.is_start() {
            return Err(ChatStreamValidationError::MustStart);
        }
        if self.started && event.is_start() {
            return Err(ChatStreamValidationError::AlreadyStarted);
        }
        self.started = true;
        self.terminal = event.is_terminal();
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ChatStreamValidationError::InvalidPayload)?;
        Ok(())
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
}

#[derive(Debug, Clone)]
pub struct ChatStreamQueue {
    max_events: usize,
    events: VecDeque<ChatStreamEvent>,
    coalesced_count: u64,
}

impl ChatStreamQueue {
    pub fn new(max_events: usize) -> Result<Self, ChatStreamValidationError> {
        if !(1..=MAX_QUEUE_EVENTS).contains(&max_events) {
            return Err(ChatStreamValidationError::InvalidBuffer);
        }
        Ok(Self {
            max_events,
            events: VecDeque::with_capacity(max_events),
            coalesced_count: 0,
        })
    }

    pub fn push(&mut self, event: ChatStreamEvent) -> Result<(), ChatStreamValidationError> {
        event.validate()?;
        if self.events.len() >= self.max_events {
            if !event.is_terminal() {
                return Err(ChatStreamValidationError::Backpressure);
            }
            let delta_index = self
                .events
                .iter()
                .position(|queued| !queued.is_terminal() && !queued.is_start());
            let Some(index) = delta_index else {
                return Err(ChatStreamValidationError::Backpressure);
            };
            self.events.remove(index);
            self.coalesced_count = self.coalesced_count.saturating_add(1);
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<ChatStreamEvent> {
        self.events.pop_front()
    }

    pub fn front(&self) -> Option<&ChatStreamEvent> {
        self.events.front()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn coalesced_count(&self) -> u64 {
        self.coalesced_count
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_LEN && !value.chars().any(char::is_control)
}

#[allow(dead_code)]
fn _valid_cancel_reason(value: &str) -> bool {
    valid_id(value) && value.len() <= MAX_CANCEL_REASON_LEN
}
