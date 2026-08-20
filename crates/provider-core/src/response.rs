//! Normalized, provider-neutral response envelope.

use crate::{ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const NORMALIZED_RESPONSE_SCHEMA_VERSION: u32 = 1;
const MAX_PARTS: usize = 64;
const MAX_PART_BYTES: usize = 1_048_576;
const MAX_TOTAL_OUTPUT_BYTES: usize = 2_097_152;
const MAX_PROVIDER_VERSION_LEN: usize = 64;
const MAX_ERROR_MESSAGE_LEN: usize = 1_024;
const MAX_CURRENCY_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Complete,
    Error,
    Cancelled,
    Limit,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCall,
    Cancelled,
    Error,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputPartKind {
    Text,
    ToolRequest,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPart {
    pub kind: OutputPartKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    pub amount_micros: u64,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorCode {
    Timeout,
    RateLimited,
    Authentication,
    InvalidRequest,
    ProviderUnavailable,
    ProviderRejected,
    Internal,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderErrorInfo {
    pub code: ProviderErrorCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedResponse {
    pub schema_version: u32,
    pub request_id: String,
    pub correlation_id: String,
    pub provider_id: ProviderId,
    pub model_id: ModelId,
    pub status: ResponseStatus,
    pub finish_reason: FinishReason,
    pub parts: Vec<OutputPart>,
    pub usage: Option<Usage>,
    pub cost: Option<Cost>,
    pub error: Option<ProviderErrorInfo>,
    pub provider_version: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedResponseSummary {
    pub request_id: String,
    pub correlation_id: String,
    pub provider_id: ProviderId,
    pub model_id: ModelId,
    pub status: ResponseStatus,
    pub finish_reason: FinishReason,
    pub part_count: usize,
    pub output_bytes: usize,
    pub usage_present: bool,
    pub cost_present: bool,
    pub error_code: Option<ProviderErrorCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResponseValidationError {
    #[error("normalized response is invalid")]
    Invalid,
    #[error("normalized response is oversized")]
    Oversized,
    #[error("normalized response contains unredacted provider error data")]
    UnredactedError,
}

impl NormalizedResponse {
    pub fn validate(&self) -> Result<(), ResponseValidationError> {
        if self.schema_version != NORMALIZED_RESPONSE_SCHEMA_VERSION
            || !valid_id(&self.request_id)
            || !valid_id(&self.correlation_id)
            || self.provider_version.trim().is_empty()
            || self.provider_version.len() > MAX_PROVIDER_VERSION_LEN
            || self.provider_version.chars().any(char::is_control)
            || self.parts.len() > MAX_PARTS
        {
            return Err(ResponseValidationError::Invalid);
        }

        let output_bytes: usize = self.parts.iter().map(|part| part.content.len()).sum();
        if self
            .parts
            .iter()
            .any(|part| part.content.len() > MAX_PART_BYTES)
            || output_bytes > MAX_TOTAL_OUTPUT_BYTES
        {
            return Err(ResponseValidationError::Oversized);
        }

        if self.status == ResponseStatus::Error && self.error.is_none() {
            return Err(ResponseValidationError::Invalid);
        }
        if self.status != ResponseStatus::Error && self.error.is_some() {
            return Err(ResponseValidationError::Invalid);
        }

        if self.usage.is_some_and(|usage| {
            usage.input_tokens > 1_000_000_000 || usage.output_tokens > 1_000_000_000
        }) || self.cost.as_ref().is_some_and(|cost| {
            cost.amount_micros > 1_000_000_000_000
                || cost.currency.trim().is_empty()
                || cost.currency.len() > MAX_CURRENCY_LEN
                || cost
                    .currency
                    .chars()
                    .any(|character| !character.is_ascii_alphabetic())
        }) || self.latency_ms.is_some_and(|latency| latency > 86_400_000)
        {
            return Err(ResponseValidationError::Invalid);
        }

        if let Some(error) = &self.error {
            if error.message.trim().is_empty() || error.message.len() > MAX_ERROR_MESSAGE_LEN {
                return Err(ResponseValidationError::Invalid);
            }
            if contains_forbidden_marker(&error.message) {
                return Err(ResponseValidationError::UnredactedError);
            }
        }
        Ok(())
    }

    pub fn redacted_summary(&self) -> RedactedResponseSummary {
        RedactedResponseSummary {
            request_id: self.request_id.clone(),
            correlation_id: self.correlation_id.clone(),
            provider_id: self.provider_id.clone(),
            model_id: self.model_id.clone(),
            status: self.status,
            finish_reason: self.finish_reason,
            part_count: self.parts.len(),
            output_bytes: self.parts.iter().map(|part| part.content.len()).sum(),
            usage_present: self.usage.is_some(),
            cost_present: self.cost.is_some(),
            error_code: self.error.as_ref().map(|error| error.code),
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
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
