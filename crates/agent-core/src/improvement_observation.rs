//! Append-only improvement observations treated as untrusted data.

use thiserror::Error;

pub const OBSERVATION_SCHEMA_VERSION: u32 = 1;
pub const MAX_OBSERVATION_PAYLOAD: usize = 16 * 1024;
const MAX_VALUE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationType {
    FailureSignal,
    SuccessSignal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationSource {
    Tool,
    Test,
    User,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyClass {
    Internal,
    Sensitive,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionClass {
    Short,
    Long,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustClass {
    Untrusted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionState {
    None,
    Redacted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRequest {
    pub schema_version: u32,
    pub source: ObservationSource,
    pub observation_type: ObservationType,
    pub project_id: String,
    pub run_id: Option<String>,
    pub trace_id: String,
    pub dedup_key: String,
    pub payload: String,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}
impl ObservationRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        schema_version: u32,
        source: &str,
        observation_type: ObservationType,
        project: &str,
        run: Option<&str>,
        trace: &str,
        key: &str,
        payload: &str,
        privacy: PrivacyClass,
        retention: RetentionClass,
    ) -> Self {
        let source = match source {
            "tool" => ObservationSource::Tool,
            "test" => ObservationSource::Test,
            _ => ObservationSource::User,
        };
        Self {
            schema_version,
            source,
            observation_type,
            project_id: project.into(),
            run_id: run.map(str::to_owned),
            trace_id: trace.into(),
            dedup_key: key.into(),
            payload: payload.into(),
            privacy,
            retention,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ObservationError {
    #[error("unsupported observation schema version")]
    UnsupportedVersion,
    #[error("observation payload exceeds limit")]
    PayloadTooLarge,
    #[error("invalid observation metadata")]
    InvalidMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImprovementObservation {
    source: ObservationSource,
    observation_type: ObservationType,
    project_id: String,
    run_id: Option<String>,
    trace_id: String,
    dedup_key: String,
    payload: String,
    privacy: PrivacyClass,
    retention: RetentionClass,
    redaction: RedactionState,
}
impl ImprovementObservation {
    pub fn accept(request: ObservationRequest) -> Result<Self, ObservationError> {
        if request.schema_version != OBSERVATION_SCHEMA_VERSION {
            return Err(ObservationError::UnsupportedVersion);
        }
        if request.payload.len() > MAX_OBSERVATION_PAYLOAD {
            return Err(ObservationError::PayloadTooLarge);
        }
        let values = [
            &request.project_id[..],
            &request.trace_id[..],
            &request.dedup_key[..],
        ];
        if values.iter().any(|value| {
            value.is_empty() || value.len() > MAX_VALUE || value.chars().any(char::is_control)
        }) {
            return Err(ObservationError::InvalidMetadata);
        }
        if request.run_id.as_deref().is_some_and(|run| {
            run.is_empty() || run.len() > MAX_VALUE || run.chars().any(char::is_control)
        }) {
            return Err(ObservationError::InvalidMetadata);
        }
        let redacted = contains_secret_like(&request.payload);
        Ok(Self {
            source: request.source,
            observation_type: request.observation_type,
            project_id: request.project_id,
            run_id: request.run_id,
            trace_id: request.trace_id,
            dedup_key: request.dedup_key,
            payload: if redacted {
                "[REDACTED]".into()
            } else {
                request.payload
            },
            privacy: request.privacy,
            retention: request.retention,
            redaction: if redacted {
                RedactionState::Redacted
            } else {
                RedactionState::None
            },
        })
    }
    pub fn trust(&self) -> TrustClass {
        TrustClass::Untrusted
    }
    pub fn has_mutation_capability(&self) -> bool {
        false
    }
    pub fn redaction(&self) -> RedactionState {
        self.redaction
    }
    pub fn dedup_key(&self) -> &str {
        &self.dedup_key
    }
    pub fn is_duplicate_of(&self, other: &Self) -> bool {
        self.project_id == other.project_id && self.dedup_key == other.dedup_key
    }
}

fn contains_secret_like(payload: &str) -> bool {
    let lower = payload.to_ascii_lowercase();
    ["api_key=", "apikey=", "token=", "password=", "secret="]
        .iter()
        .any(|marker| lower.contains(marker))
}
