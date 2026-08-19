use crate::ids::{AgentId, EventId, ProjectId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const EVENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_EVENT_PAYLOAD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    ProjectCreated,
    ProjectUpdated,
    ProjectArchived,
    AgentCreated,
    SessionStarted,
    ProviderUsageRecorded,
    RunCompleted,
    RunFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationEvent {
    pub schema_version: u32,
    pub event_id: EventId,
    pub event_type: EventKind,
    pub project_id: ProjectId,
    pub aggregate_id: String,
    pub agent_id: Option<AgentId>,
    pub session_id: Option<SessionId>,
    pub occurred_at: DateTime<Utc>,
    pub sequence: u64,
    pub payload: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EventValidationError {
    #[error("unsupported event schema version")]
    UnsupportedVersion,
    #[error("event payload exceeds limit")]
    PayloadTooLarge,
    #[error("aggregate id is empty")]
    EmptyAggregate,
    #[error("event sequence must be non-zero")]
    InvalidSequence,
}

impl ApplicationEvent {
    pub fn validate(&self) -> Result<(), EventValidationError> {
        if self.schema_version != EVENT_SCHEMA_VERSION {
            return Err(EventValidationError::UnsupportedVersion);
        }
        if self.payload.len() > MAX_EVENT_PAYLOAD_BYTES {
            return Err(EventValidationError::PayloadTooLarge);
        }
        if self.aggregate_id.trim().is_empty() {
            return Err(EventValidationError::EmptyAggregate);
        }
        if self.sequence == 0 {
            return Err(EventValidationError::InvalidSequence);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> ApplicationEvent {
        ApplicationEvent {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(),
            event_type: EventKind::SessionStarted,
            project_id: ProjectId::new(),
            aggregate_id: "session-1".into(),
            agent_id: Some(AgentId::new()),
            session_id: Some(SessionId::new()),
            occurred_at: Utc::now(),
            sequence: 1,
            payload: "synthetic".into(),
        }
    }

    #[test]
    fn application_event_roundtrips_and_validates() {
        let event = event();
        event.validate().unwrap();
        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: ApplicationEvent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn malformed_and_oversized_events_fail_closed() {
        let mut unsupported = event();
        unsupported.schema_version = 2;
        assert_eq!(
            unsupported.validate(),
            Err(EventValidationError::UnsupportedVersion)
        );
        let mut oversized = event();
        oversized.payload = "x".repeat(MAX_EVENT_PAYLOAD_BYTES + 1);
        assert_eq!(
            oversized.validate(),
            Err(EventValidationError::PayloadTooLarge)
        );
        let mut unordered = event();
        unordered.sequence = 0;
        assert_eq!(
            unordered.validate(),
            Err(EventValidationError::InvalidSequence)
        );
    }
}
