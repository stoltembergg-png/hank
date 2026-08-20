//! Project/Agent-scoped Session domain entity.

use crate::ids::{AgentId, MessageId, ProjectId, SessionId};
use agent_protocol::ids::TraceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

const MAX_CORRELATION_ID_LEN: usize = 128;
const MAX_PARTICIPANTS: usize = 32;
const MAX_PARTICIPANT_LABEL_LEN: usize = 128;
const MAX_METADATA_ENTRIES: usize = 64;
const MAX_METADATA_KEY_LEN: usize = 128;
const MAX_METADATA_VALUE_BYTES: usize = 4_096;
const MAX_REFERENCE_LEN: usize = 128;
const MAX_FAILURE_REASON_LEN: usize = 256;
const MAX_MESSAGE_PARTS: usize = 64;
const MAX_MESSAGE_PART_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Created,
    Active,
    Closing,
    Closed,
    Failed,
}

impl SessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRole {
    Owner,
    Participant,
    Observer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionParticipant {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub role: SessionRole,
    pub label: String,
}

impl SessionParticipant {
    pub fn new(
        project_id: ProjectId,
        agent_id: AgentId,
        role: SessionRole,
        label: impl Into<String>,
    ) -> Result<Self, SessionError> {
        let label = label.into();
        validate_text(&label, MAX_PARTICIPANT_LABEL_LEN, false)?;
        Ok(Self {
            project_id,
            agent_id,
            role,
            label,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionError {
    #[error("session identity or metadata is invalid")]
    InvalidMetadata,
    #[error("session lifecycle transition is invalid: {from:?} -> {to:?}")]
    InvalidTransition {
        from: SessionStatus,
        to: SessionStatus,
    },
    #[error("session is terminal")]
    Terminal,
    #[error("session participant is outside the project scope")]
    ScopeMismatch,
    #[error("session participant already exists")]
    DuplicateParticipant,
    #[error("session participant limit is exceeded")]
    ParticipantLimit,
    #[error("session metadata key already exists")]
    DuplicateMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub schema_version: u32,
    pub id: SessionId,
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub status: SessionStatus,
    pub correlation_id: String,
    pub participants: Vec<SessionParticipant>,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub budget_ref: Option<String>,
    pub trace_id: Option<TraceId>,
    pub failure_reason: Option<String>,
    pub title: Option<String>,
    pub message_count: usize,
    pub token_count: u64,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl Session {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(
        project_id: ProjectId,
        agent_id: AgentId,
        correlation_id: impl Into<String>,
    ) -> Result<Self, SessionError> {
        let correlation_id = correlation_id.into();
        validate_text(&correlation_id, MAX_CORRELATION_ID_LEN, true)?;
        let now = Utc::now();
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            id: SessionId::new(),
            project_id,
            agent_id,
            status: SessionStatus::Created,
            correlation_id,
            participants: Vec::new(),
            metadata: BTreeMap::new(),
            budget_ref: None,
            trace_id: None,
            failure_reason: None,
            title: None,
            message_count: 0,
            token_count: 0,
            cost_usd: 0.0,
            created_at: now,
            updated_at: now,
            closed_at: None,
        })
    }

    pub fn activate(&mut self) -> Result<(), SessionError> {
        self.transition(SessionStatus::Active)
    }

    pub fn begin_close(&mut self) -> Result<(), SessionError> {
        self.transition(SessionStatus::Closing)
    }

    pub fn close(&mut self) -> Result<(), SessionError> {
        if self.status == SessionStatus::Closed {
            return Ok(());
        }
        self.transition(SessionStatus::Closed)?;
        self.closed_at = Some(Utc::now());
        Ok(())
    }

    pub fn fail(&mut self, reason: impl Into<String>) -> Result<(), SessionError> {
        if self.status.is_terminal() {
            return Err(SessionError::Terminal);
        }
        let reason = reason.into();
        validate_text(&reason, MAX_FAILURE_REASON_LEN, true)?;
        self.failure_reason = Some(reason);
        self.transition(SessionStatus::Failed)?;
        self.closed_at = Some(Utc::now());
        Ok(())
    }

    pub fn add_participant(&mut self, participant: SessionParticipant) -> Result<(), SessionError> {
        self.ensure_mutable()?;
        if participant.project_id != self.project_id {
            return Err(SessionError::ScopeMismatch);
        }
        if self
            .participants
            .iter()
            .any(|existing| existing.agent_id == participant.agent_id)
        {
            return Err(SessionError::DuplicateParticipant);
        }
        if self.participants.len() >= MAX_PARTICIPANTS {
            return Err(SessionError::ParticipantLimit);
        }
        self.participants.push(participant);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn set_budget_ref(&mut self, reference: impl Into<String>) -> Result<(), SessionError> {
        self.ensure_mutable()?;
        let reference = reference.into();
        validate_text(&reference, MAX_REFERENCE_LEN, true)?;
        if !reference.starts_with("budget_") {
            return Err(SessionError::InvalidMetadata);
        }
        self.budget_ref = Some(reference);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn set_trace_id(&mut self, trace_id: TraceId) -> Result<(), SessionError> {
        self.ensure_mutable()?;
        self.trace_id = Some(trace_id);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn add_metadata(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<(), SessionError> {
        self.ensure_mutable()?;
        let key = key.into();
        validate_text(&key, MAX_METADATA_KEY_LEN, true)?;
        if self.metadata.len() >= MAX_METADATA_ENTRIES {
            return Err(SessionError::InvalidMetadata);
        }
        if self.metadata.contains_key(&key) {
            return Err(SessionError::DuplicateMetadata);
        }
        let encoded = serde_json::to_string(&value).map_err(|_| SessionError::InvalidMetadata)?;
        if encoded.len() > MAX_METADATA_VALUE_BYTES || contains_forbidden_marker(&encoded) {
            return Err(SessionError::InvalidMetadata);
        }
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn add_message(&mut self, message: Message) {
        self.message_count += 1;
        self.token_count += message.tokens as u64;
        self.cost_usd += message.cost_usd;
        self.updated_at = Utc::now();
    }

    fn transition(&mut self, next: SessionStatus) -> Result<(), SessionError> {
        let valid = matches!(
            (self.status, next),
            (SessionStatus::Created, SessionStatus::Active)
                | (SessionStatus::Active, SessionStatus::Closing)
                | (SessionStatus::Closing, SessionStatus::Closed)
                | (SessionStatus::Created, SessionStatus::Failed)
                | (SessionStatus::Active, SessionStatus::Failed)
                | (SessionStatus::Closing, SessionStatus::Failed)
        );
        if !valid {
            return if self.status.is_terminal() {
                Err(SessionError::Terminal)
            } else {
                Err(SessionError::InvalidTransition {
                    from: self.status,
                    to: next,
                })
            };
        }
        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }

    fn ensure_mutable(&self) -> Result<(), SessionError> {
        if matches!(self.status, SessionStatus::Created | SessionStatus::Active) {
            Ok(())
        } else {
            Err(SessionError::Terminal)
        }
    }
}

fn validate_text(value: &str, max_len: usize, forbidden_markers: bool) -> Result<(), SessionError> {
    if value.trim().is_empty()
        || value.len() > max_len
        || value.chars().any(char::is_control)
        || (forbidden_markers && contains_forbidden_marker(value))
    {
        return Err(SessionError::InvalidMetadata);
    }
    Ok(())
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

/// Papel semântico da mensagem. Provenance é separado para que um campo de
/// role não possa elevar instruções de conteúdo não confiável.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
    ToolResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageProvenance {
    System,
    User,
    Agent,
    Provider,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Draft,
    Streaming,
    Complete,
    Failed,
    Cancelled,
}

impl MessageStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePartKind {
    Text,
    ToolCall,
    ToolResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagePart {
    pub kind: MessagePartKind,
    pub content: String,
    pub untrusted: bool,
}

impl MessagePart {
    pub fn new(
        kind: MessagePartKind,
        content: impl Into<String>,
        untrusted: bool,
    ) -> Result<Self, MessageError> {
        let content = content.into();
        validate_message_text(&content)?;
        if contains_forbidden_marker(&content) {
            return Err(MessageError::ForbiddenContent);
        }
        Ok(Self {
            kind,
            content,
            untrusted,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MessageError {
    #[error("message metadata or content is invalid")]
    InvalidMetadata,
    #[error("message contains forbidden secret-like content")]
    ForbiddenContent,
    #[error("message part limit is exceeded")]
    PartLimit,
    #[error("message state transition is invalid: {from:?} -> {to:?}")]
    InvalidTransition {
        from: MessageStatus,
        to: MessageStatus,
    },
    #[error("message is terminal")]
    Terminal,
    #[error("message belongs to another session")]
    SessionMismatch,
    #[error("message generation is stale: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("message generation is ahead: expected {expected}, got {actual}")]
    FutureGeneration { expected: u64, actual: u64 },
    #[error("message sequence was duplicated: {0}")]
    DuplicateSequence(u64),
    #[error("message sequence is out of order: expected {expected}, got {actual}")]
    OutOfOrder { expected: u64, actual: u64 },
    #[error("message ordering is already terminal")]
    AfterTerminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub schema_version: u32,
    pub id: MessageId,
    pub session_id: SessionId,
    pub role: MessageRole,
    pub provenance: MessageProvenance,
    pub status: MessageStatus,
    pub correlation_id: String,
    pub sequence: u64,
    pub generation: u64,
    pub parts: Vec<MessagePart>,
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<ToolResult>,
    pub tokens: u32,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(
        session_id: SessionId,
        role: MessageRole,
        provenance: MessageProvenance,
        sequence: u64,
        generation: u64,
        content: impl Into<String>,
    ) -> Result<Self, MessageError> {
        if generation == 0 {
            return Err(MessageError::InvalidMetadata);
        }
        let id = MessageId::new();
        let content = content.into();
        let untrusted = matches!(
            provenance,
            MessageProvenance::User | MessageProvenance::Provider | MessageProvenance::Tool
        );
        let part = MessagePart::new(MessagePartKind::Text, content.clone(), untrusted)?;
        validate_message_text(&format!("corr_{id}"))?;
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            id,
            session_id,
            role,
            provenance,
            status: MessageStatus::Draft,
            correlation_id: format!("corr_{id}"),
            sequence,
            generation,
            parts: vec![part],
            content,
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            tokens: 0,
            cost_usd: 0.0,
            created_at: Utc::now(),
        })
    }

    pub fn validate(&self) -> Result<(), MessageError> {
        if self.schema_version != Self::SCHEMA_VERSION
            || self.generation == 0
            || self.correlation_id.trim().is_empty()
            || self.correlation_id.len() > MAX_CORRELATION_ID_LEN
            || self.correlation_id.chars().any(char::is_control)
            || self.parts.is_empty()
            || self.parts.len() > MAX_MESSAGE_PARTS
        {
            return Err(MessageError::InvalidMetadata);
        }
        validate_message_text(&self.content)?;
        for part in &self.parts {
            validate_message_text(&part.content)?;
            if contains_forbidden_marker(&part.content) {
                return Err(MessageError::ForbiddenContent);
            }
        }
        Ok(())
    }

    pub fn add_part(&mut self, part: MessagePart) -> Result<(), MessageError> {
        self.ensure_mutable()?;
        if self.parts.len() >= MAX_MESSAGE_PARTS {
            return Err(MessageError::PartLimit);
        }
        self.parts.push(part);
        self.content = self
            .parts
            .iter()
            .filter(|part| part.kind == MessagePartKind::Text)
            .map(|part| part.content.as_str())
            .collect::<Vec<_>>()
            .join("");
        Ok(())
    }

    pub fn start_stream(&mut self) -> Result<(), MessageError> {
        if self.status == MessageStatus::Streaming {
            return Ok(());
        }
        self.transition(MessageStatus::Streaming)
    }

    pub fn complete(&mut self) -> Result<(), MessageError> {
        if self.status == MessageStatus::Complete {
            return Ok(());
        }
        self.transition(MessageStatus::Complete)
    }

    pub fn fail(&mut self, reason: impl Into<String>) -> Result<(), MessageError> {
        let reason = reason.into();
        validate_message_text(&reason)?;
        self.transition(MessageStatus::Failed)
    }

    pub fn cancel(&mut self) -> Result<(), MessageError> {
        self.transition(MessageStatus::Cancelled)
    }

    fn ensure_mutable(&self) -> Result<(), MessageError> {
        if self.status.is_terminal() {
            Err(MessageError::Terminal)
        } else {
            Ok(())
        }
    }

    fn transition(&mut self, next: MessageStatus) -> Result<(), MessageError> {
        if self.status.is_terminal() {
            return Err(MessageError::Terminal);
        }
        let valid = matches!(
            (self.status, next),
            (MessageStatus::Draft, MessageStatus::Streaming)
                | (MessageStatus::Draft, MessageStatus::Failed)
                | (MessageStatus::Draft, MessageStatus::Cancelled)
                | (MessageStatus::Streaming, MessageStatus::Complete)
                | (MessageStatus::Streaming, MessageStatus::Failed)
                | (MessageStatus::Streaming, MessageStatus::Cancelled)
        );
        if !valid {
            return Err(MessageError::InvalidTransition {
                from: self.status,
                to: next,
            });
        }
        self.status = next;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MessageOrdering {
    session_id: SessionId,
    generation: u64,
    next_sequence: u64,
    terminal: bool,
}

impl MessageOrdering {
    pub fn new(session_id: SessionId, generation: u64) -> Result<Self, MessageError> {
        if generation == 0 {
            return Err(MessageError::InvalidMetadata);
        }
        Ok(Self {
            session_id,
            generation,
            next_sequence: 0,
            terminal: false,
        })
    }

    pub fn accept(&mut self, message: Message) -> Result<(), MessageError> {
        message.validate()?;
        if message.session_id != self.session_id {
            return Err(MessageError::SessionMismatch);
        }
        if message.generation < self.generation {
            return Err(MessageError::StaleGeneration {
                expected: self.generation,
                actual: message.generation,
            });
        }
        if message.generation > self.generation {
            return Err(MessageError::FutureGeneration {
                expected: self.generation,
                actual: message.generation,
            });
        }
        if self.terminal {
            return Err(MessageError::AfterTerminal);
        }
        if message.sequence < self.next_sequence {
            return Err(MessageError::DuplicateSequence(message.sequence));
        }
        if message.sequence > self.next_sequence {
            return Err(MessageError::OutOfOrder {
                expected: self.next_sequence,
                actual: message.sequence,
            });
        }
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(MessageError::InvalidMetadata)?;
        self.terminal = message.status.is_terminal();
        Ok(())
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
}

fn validate_message_text(value: &str) -> Result<(), MessageError> {
    if value.len() > MAX_MESSAGE_PART_BYTES || value.chars().any(char::is_control) {
        return Err(MessageError::InvalidMetadata);
    }
    Ok(())
}

/// Existing tool-call payload. Tool execution remains outside this entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}
