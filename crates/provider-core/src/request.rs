//! Normalized, provider-neutral request envelope.

use crate::capabilities::{
    CapabilityError, CapabilityReport, CapabilityRequirement, ModelModality,
};
use crate::{ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const NORMALIZED_REQUEST_SCHEMA_VERSION: u32 = 1;
const MAX_ID_LEN: usize = 128;
const MAX_MESSAGES: usize = 128;
const MAX_TOOLS: usize = 64;
const MAX_MESSAGE_BYTES: usize = 1_048_576;
const MAX_TOTAL_CONTENT_BYTES: usize = 2_097_152;
const MAX_TOOL_ID_LEN: usize = 128;
const MAX_FINGERPRINT_LEN: usize = 128;
const MAX_COST_MICROS: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolContext {
    pub tool_id: String,
    pub capability_fingerprint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequestBudget {
    pub max_tokens: Option<u32>,
    pub max_cost_micros: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancellationMetadata {
    pub cancellation_id: String,
    pub deadline_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRequest {
    pub schema_version: u32,
    pub request_id: String,
    pub correlation_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub provider_id: ProviderId,
    pub model_id: ModelId,
    pub messages: Vec<NormalizedMessage>,
    pub modalities: std::collections::BTreeSet<ModelModality>,
    pub capabilities: CapabilityRequirement,
    pub tools: Vec<ToolContext>,
    pub budget: RequestBudget,
    pub cancellation: CancellationMetadata,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedRequestSummary {
    pub request_id: String,
    pub correlation_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub provider_id: ProviderId,
    pub model_id: ModelId,
    pub message_count: usize,
    pub total_content_bytes: usize,
    pub tool_count: usize,
    pub modality_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RequestValidationError {
    #[error("normalized request is invalid")]
    Invalid,
    #[error("normalized request exceeds its bounded size")]
    Oversized,
    #[error("normalized request contains forbidden tool metadata")]
    ForbiddenToolMetadata,
}

impl NormalizedRequest {
    pub fn validate(&self) -> Result<(), RequestValidationError> {
        if self.schema_version != NORMALIZED_REQUEST_SCHEMA_VERSION
            || !valid_id(&self.request_id)
            || !valid_id(&self.correlation_id)
            || !valid_id(&self.project_id)
            || !valid_id(&self.agent_id)
            || self.session_id.as_deref().is_some_and(|id| !valid_id(id))
            || self.messages.is_empty()
            || self.messages.len() > MAX_MESSAGES
            || self.modalities.is_empty()
            || self.modalities.len() > 4
            || self.capabilities.modalities.is_empty()
            || self.tools.len() > MAX_TOOLS
            || self.cancellation.cancellation_id.trim().is_empty()
            || !valid_id(&self.cancellation.cancellation_id)
        {
            return Err(RequestValidationError::Invalid);
        }

        let total_content_bytes: usize = self
            .messages
            .iter()
            .map(|message| message.content.len())
            .sum();
        if self
            .messages
            .iter()
            .any(|message| message.content.len() > MAX_MESSAGE_BYTES)
            || total_content_bytes > MAX_TOTAL_CONTENT_BYTES
        {
            return Err(RequestValidationError::Oversized);
        }

        for tool in &self.tools {
            if !valid_bounded_text(&tool.tool_id, MAX_TOOL_ID_LEN)
                || !tool.capability_fingerprint.starts_with("cap_")
                || !valid_bounded_text(&tool.capability_fingerprint, MAX_FINGERPRINT_LEN)
            {
                return Err(RequestValidationError::Invalid);
            }
            if contains_forbidden_marker(&tool.capability_fingerprint) {
                return Err(RequestValidationError::ForbiddenToolMetadata);
            }
        }

        if self
            .budget
            .max_tokens
            .is_some_and(|value| !(1..=1_000_000).contains(&value))
            || self
                .budget
                .max_cost_micros
                .is_some_and(|value| value > MAX_COST_MICROS)
            || self
                .temperature
                .is_some_and(|value| !value.is_finite() || !(0.0..=2.0).contains(&value))
            || self
                .cancellation
                .deadline_unix_ms
                .is_some_and(|deadline| deadline <= 0)
        {
            return Err(RequestValidationError::Invalid);
        }

        Ok(())
    }

    pub fn validate_against_capabilities(
        &self,
        report: &CapabilityReport,
    ) -> Result<(), CapabilityError> {
        report.check_compatibility(&self.capabilities)
    }

    pub fn redacted_summary(&self) -> RedactedRequestSummary {
        RedactedRequestSummary {
            request_id: self.request_id.clone(),
            correlation_id: self.correlation_id.clone(),
            project_id: self.project_id.clone(),
            agent_id: self.agent_id.clone(),
            session_id: self.session_id.clone(),
            provider_id: self.provider_id.clone(),
            model_id: self.model_id.clone(),
            message_count: self.messages.len(),
            total_content_bytes: self
                .messages
                .iter()
                .map(|message| message.content.len())
                .sum(),
            tool_count: self.tools.len(),
            modality_count: self.modalities.len(),
        }
    }
}

fn valid_id(value: &str) -> bool {
    valid_bounded_text(value, MAX_ID_LEN)
}

fn valid_bounded_text(value: &str, max_len: usize) -> bool {
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
