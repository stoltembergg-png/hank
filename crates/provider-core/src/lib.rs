//! Provider-neutral contracts for model adapters.
//!
//! This crate intentionally contains no HTTP client, SDK, secret storage, or
//! concrete provider implementation. Adapters implement [`ModelProvider`]
//! behind this boundary.

use futures_core::Stream;
use futures_util::stream as futures_stream;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use thiserror::Error;

pub mod capabilities;
pub mod credentials;
pub mod registry;
pub mod request;
pub mod response;
pub mod stream;
pub mod transport;

pub const MAX_PROVIDER_ID_LEN: usize = 120;
pub const MAX_MODEL_ID_LEN: usize = 200;
pub const MAX_CREDENTIAL_REF_LEN: usize = 128;
pub const MAX_REQUEST_ID_LEN: usize = 128;
pub const MAX_PROMPT_BYTES: usize = 1_048_576;
pub const MAX_STREAM_BUFFERED_EVENTS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelProviderError> {
        let value = value.into();
        validate_identifier(&value, MAX_PROVIDER_ID_LEN, "provider")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(String);

impl ModelId {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelProviderError> {
        let value = value.into();
        validate_identifier(&value, MAX_MODEL_ID_LEN, "model")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Opaque reference to a credential managed outside this crate.
///
/// The value is deliberately not a key, token, password, endpoint, or secret.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CredentialRef(String);

impl CredentialRef {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelProviderError> {
        let value = value.into();
        let normalized = value.to_ascii_lowercase();
        if !value.starts_with("cred_")
            || value.len() > MAX_CREDENTIAL_REF_LEN
            || value.len() <= 5
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            || ["api_key", "secret", "token", "password", "bearer"]
                .iter()
                .any(|marker| normalized.contains(marker))
        {
            return Err(ModelProviderError::InvalidCredentialRef);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CredentialRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CredentialRef([REDACTED])")
    }
}

impl fmt::Display for CredentialRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("credential_ref:[REDACTED]")
    }
}

fn validate_identifier(
    value: &str,
    max_len: usize,
    kind: &'static str,
) -> Result<(), ModelProviderError> {
    let normalized = value.to_ascii_lowercase();
    if value.trim().is_empty()
        || value.len() > max_len
        || value.contains("://")
        || value.chars().any(char::is_control)
        || ["api_key", "secret", "token", "password", "bearer"]
            .iter()
            .any(|marker| normalized.contains(marker))
    {
        return Err(match kind {
            "provider" => ModelProviderError::InvalidProviderId,
            _ => ModelProviderError::InvalidModelId,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRequest {
    pub request_id: String,
    pub model_id: ModelId,
    pub credential_ref: CredentialRef,
    pub prompt: String,
    pub max_tokens: Option<u32>,
}

impl ProviderRequest {
    pub fn new(
        request_id: impl Into<String>,
        model_id: ModelId,
        credential_ref: CredentialRef,
        prompt: impl Into<String>,
    ) -> Result<Self, ModelProviderError> {
        let request_id = request_id.into();
        let prompt = prompt.into();
        if request_id.trim().is_empty()
            || request_id.len() > MAX_REQUEST_ID_LEN
            || request_id.chars().any(char::is_control)
            || prompt.len() > MAX_PROMPT_BYTES
        {
            return Err(ModelProviderError::InvalidRequest);
        }
        Ok(Self {
            request_id,
            model_id,
            credential_ref,
            prompt,
            max_tokens: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamConfig {
    pub max_buffered_events: usize,
}

impl StreamConfig {
    pub fn new(max_buffered_events: usize) -> Result<Self, ModelProviderError> {
        if !(1..=MAX_STREAM_BUFFERED_EVENTS).contains(&max_buffered_events) {
            return Err(ModelProviderError::Backpressure);
        }
        Ok(Self {
            max_buffered_events,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub model_id: ModelId,
    pub display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub model_id: ModelId,
    pub text: String,
    pub finish_reason: FinishReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderStreamEvent {
    pub sequence: u64,
    pub text: String,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModelProviderError {
    #[error("invalid provider identifier")]
    InvalidProviderId,
    #[error("invalid model identifier")]
    InvalidModelId,
    #[error("invalid opaque credential reference")]
    InvalidCredentialRef,
    #[error("invalid provider request")]
    InvalidRequest,
    #[error("unsupported provider operation: {0}")]
    UnsupportedOperation(String),
    #[error("provider operation cancelled")]
    Cancelled,
    #[error("provider stream backpressure configuration is invalid")]
    Backpressure,
    #[error("provider is unavailable")]
    Unavailable,
    #[error("provider internal error")]
    Internal,
}

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type ProviderStream<'a> =
    Pin<Box<dyn Stream<Item = Result<ProviderStreamEvent, ModelProviderError>> + Send + 'a>>;

/// Stable provider-neutral contract implemented by adapters.
pub trait ModelProvider: Send + Sync {
    fn provider_id(&self) -> &ProviderId;
    fn version(&self) -> &str;
    fn capabilities(&self) -> crate::capabilities::CapabilityReport;
    fn complete(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProviderResponse, ModelProviderError>>;
    fn stream(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
        config: StreamConfig,
    ) -> Result<ProviderStream<'_>, ModelProviderError>;
    fn list_models(&self) -> ProviderFuture<'_, Result<Vec<ModelDescriptor>, ModelProviderError>>;
    fn health(&self) -> ProviderFuture<'_, Result<HealthStatus, ModelProviderError>>;
}

/// Deterministic contract fixture. It is not a real provider or network adapter.
pub struct MockProvider {
    provider_id: ProviderId,
    version: String,
    model_id: ModelId,
    capabilities: crate::capabilities::CapabilityReport,
}

impl MockProvider {
    pub fn new(provider_id: ProviderId, version: impl Into<String>) -> Self {
        let version_owned = version.into();
        let capabilities = crate::capabilities::CapabilityReport {
            schema_version: 1,
            provider_id: provider_id.clone(),
            model_id: ModelId::parse("mock-model").expect("static mock model id is valid"),
            version: version_owned.clone(),
            source: crate::capabilities::CapabilitySource::Provider,
            modalities: std::collections::BTreeMap::from([
                (
                    crate::capabilities::ModelModality::Text,
                    crate::capabilities::CapabilityState::Supported,
                ),
                (
                    crate::capabilities::ModelModality::Image,
                    crate::capabilities::CapabilityState::Unsupported,
                ),
                (
                    crate::capabilities::ModelModality::Audio,
                    crate::capabilities::CapabilityState::Unsupported,
                ),
                (
                    crate::capabilities::ModelModality::Video,
                    crate::capabilities::CapabilityState::Unsupported,
                ),
            ]),
            features: std::collections::BTreeMap::from([
                (
                    crate::capabilities::CapabilityFeature::Streaming,
                    crate::capabilities::CapabilityState::Supported,
                ),
                (
                    crate::capabilities::CapabilityFeature::ToolUse,
                    crate::capabilities::CapabilityState::Supported,
                ),
                (
                    crate::capabilities::CapabilityFeature::Vision,
                    crate::capabilities::CapabilityState::Supported,
                ),
                (
                    crate::capabilities::CapabilityFeature::AudioInput,
                    crate::capabilities::CapabilityState::Unsupported,
                ),
            ]),
            limits: crate::capabilities::CapabilityLimits {
                max_context_tokens: Some(32_768),
                max_output_tokens: Some(8_192),
            },
        };
        Self {
            provider_id,
            version: version_owned,
            model_id: ModelId::parse("mock-model").expect("static mock model id is valid"),
            capabilities,
        }
    }
}

impl ModelProvider for MockProvider {
    fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn capabilities(&self) -> crate::capabilities::CapabilityReport {
        self.capabilities.clone()
    }

    fn complete(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProviderResponse, ModelProviderError>> {
        let model_id = request.model_id.clone();
        let prompt = request.prompt.clone();
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(ModelProviderError::Cancelled);
            }
            Ok(ProviderResponse {
                model_id,
                text: format!("mock response: {prompt}"),
                finish_reason: FinishReason::Stop,
                usage: Usage {
                    input_tokens: prompt.len() as u32,
                    output_tokens: 3,
                },
            })
        })
    }

    fn stream(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
        config: StreamConfig,
    ) -> Result<ProviderStream<'_>, ModelProviderError> {
        if cancellation.is_cancelled() {
            return Err(ModelProviderError::Cancelled);
        }
        if !self
            .capabilities
            .supports_feature(crate::capabilities::CapabilityFeature::Streaming)
        {
            return Err(ModelProviderError::UnsupportedOperation("stream".into()));
        }
        if config.max_buffered_events < 2 {
            return Err(ModelProviderError::Backpressure);
        }
        Ok(Box::pin(futures_stream::iter([
            Ok(ProviderStreamEvent {
                sequence: 0,
                text: format!("mock response: {}", request.prompt),
                terminal: false,
            }),
            Ok(ProviderStreamEvent {
                sequence: 1,
                text: String::new(),
                terminal: true,
            }),
        ])))
    }

    fn list_models(&self) -> ProviderFuture<'_, Result<Vec<ModelDescriptor>, ModelProviderError>> {
        let supported = self
            .capabilities
            .supports_feature(crate::capabilities::CapabilityFeature::ToolUse);
        let model_id = self.model_id.clone();
        Box::pin(async move {
            if !supported {
                return Err(ModelProviderError::UnsupportedOperation(
                    "list_models".into(),
                ));
            }
            Ok(vec![ModelDescriptor {
                model_id,
                display_name: "Mock model".into(),
            }])
        })
    }

    fn health(&self) -> ProviderFuture<'_, Result<HealthStatus, ModelProviderError>> {
        let supported = self
            .capabilities
            .supports_feature(crate::capabilities::CapabilityFeature::Vision);
        Box::pin(async move {
            if !supported {
                return Err(ModelProviderError::UnsupportedOperation("health".into()));
            }
            Ok(HealthStatus::Healthy)
        })
    }
}
