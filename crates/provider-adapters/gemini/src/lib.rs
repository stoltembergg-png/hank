//! Isolated Gemini adapter and provider descriptor.

use provider_core::capabilities::{
    CapabilityError, CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::request::{MessageRole, NormalizedRequest};
use provider_core::response::{
    FinishReason, NormalizedResponse, OutputPart, OutputPartKind, ProviderErrorCode,
    ProviderErrorInfo, ResponseStatus, Usage,
};
use provider_core::stream::{
    StreamEvent, StreamEventPayload, StreamValidationError, StreamValidator,
};
use provider_core::transport::{
    validate_request_body, EndpointError, EndpointPolicy, HttpRequest, HttpTransport,
    TransportError,
};
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use thiserror::Error;

const GEMINI_DESCRIPTOR_VERSION: &str = "gemini-descriptor-1";
const MAX_RESPONSE_BYTES: usize = 2_097_152;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeminiModel {
    Gemini15Pro,
    Gemini15Flash,
}

impl GeminiModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gemini15Pro => "gemini-1.5-pro",
            Self::Gemini15Flash => "gemini-1.5-flash",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiModelDescriptor {
    pub model_id: ModelId,
    pub capabilities: CapabilityReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiProviderDescriptor {
    provider_id: ProviderId,
    version: String,
    models: Vec<GeminiModelDescriptor>,
}

#[derive(Debug, Error)]
pub enum ProviderDescriptorError {
    #[error("normalized request targets another provider")]
    ProviderMismatch,
    #[error("Gemini model is not declared: {0}")]
    UnsupportedModel(String),
    #[error("normalized request capability is unsupported: {0:?}")]
    UnsupportedCapability(CapabilityError),
    #[error("normalized request is invalid")]
    InvalidRequest,
    #[error("Gemini adapter error: {0}")]
    Adapter(#[from] AdapterError),
}

impl GeminiProviderDescriptor {
    pub fn new() -> Self {
        let provider_id = ProviderId::parse("gemini").expect("static provider id is valid");
        let models = [GeminiModel::Gemini15Pro, GeminiModel::Gemini15Flash]
            .into_iter()
            .map(|model| GeminiModelDescriptor {
                model_id: ModelId::parse(model.as_str()).expect("static model id is valid"),
                capabilities: capabilities_for(&provider_id, model),
            })
            .collect();
        Self {
            provider_id,
            version: GEMINI_DESCRIPTOR_VERSION.into(),
            models,
        }
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn models(&self) -> &[GeminiModelDescriptor] {
        &self.models
    }

    pub fn model(&self, model: GeminiModel) -> ModelId {
        ModelId::parse(model.as_str()).expect("static model id is valid")
    }

    pub fn capabilities(
        &self,
        model: GeminiModel,
    ) -> Result<&CapabilityReport, ProviderDescriptorError> {
        self.models
            .iter()
            .find(|descriptor| descriptor.model_id.as_str() == model.as_str())
            .map(|descriptor| &descriptor.capabilities)
            .ok_or_else(|| ProviderDescriptorError::UnsupportedModel(model.as_str().into()))
    }

    pub fn validate_request(
        &self,
        request: &NormalizedRequest,
    ) -> Result<(), ProviderDescriptorError> {
        if request.provider_id != self.provider_id {
            return Err(ProviderDescriptorError::ProviderMismatch);
        }
        request
            .validate()
            .map_err(|_| ProviderDescriptorError::InvalidRequest)?;
        let capabilities = self
            .models
            .iter()
            .find(|descriptor| descriptor.model_id == request.model_id)
            .ok_or_else(|| {
                ProviderDescriptorError::UnsupportedModel(request.model_id.as_str().into())
            })?
            .capabilities
            .clone();
        capabilities
            .check_compatibility(&request.capabilities)
            .map_err(ProviderDescriptorError::UnsupportedCapability)
    }
}

impl Default for GeminiProviderDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("invalid endpoint: {0}")]
    Endpoint(#[from] EndpointError),
    #[error("invalid normalized request")]
    InvalidRequest,
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("malformed Gemini response")]
    MalformedResponse,
    #[error("normalized response: {0:?}")]
    Response(Box<NormalizedResponse>),
    #[error("stream validation error: {0}")]
    Stream(#[from] StreamValidationError),
    #[error("Gemini stream ended without a terminal event")]
    IncompleteStream,
}

pub struct GeminiProvider<T> {
    descriptor: GeminiProviderDescriptor,
    endpoint: EndpointPolicy,
    credential_ref: CredentialRef,
    transport: T,
    timeout: Duration,
}

impl<T: HttpTransport> GeminiProvider<T> {
    pub fn new(
        endpoint: EndpointPolicy,
        credential_ref: CredentialRef,
        transport: T,
        timeout: Duration,
    ) -> Result<Self, ProviderDescriptorError> {
        if timeout.is_zero() {
            return Err(ProviderDescriptorError::Adapter(AdapterError::Transport(
                TransportError::Timeout,
            )));
        }
        Ok(Self {
            descriptor: GeminiProviderDescriptor::new(),
            endpoint,
            credential_ref,
            transport,
            timeout,
        })
    }

    pub fn descriptor(&self) -> &GeminiProviderDescriptor {
        &self.descriptor
    }

    pub fn complete(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<NormalizedResponse, ProviderDescriptorError> {
        self.descriptor.validate_request(&request)?;
        self.send(&request, cancellation, false)
            .map(|mut response| {
                response.provider_id = self.descriptor.provider_id.clone();
                response
            })
            .map_err(rewrite_adapter_error)
    }

    pub fn stream(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<Vec<StreamEvent>, ProviderDescriptorError> {
        self.descriptor.validate_request(&request)?;
        self.send_stream(&request, cancellation)
            .map(|mut events| {
                for event in &mut events {
                    if let StreamEventPayload::Start { provider_id, .. } = &mut event.payload {
                        *provider_id = self.descriptor.provider_id.clone();
                    }
                }
                events
            })
            .map_err(rewrite_adapter_error)
    }

    fn send(
        &self,
        request: &NormalizedRequest,
        cancellation: &CancellationToken,
        stream: bool,
    ) -> Result<NormalizedResponse, AdapterError> {
        let response = self.transport.send(
            self.build_request(request, stream)?,
            self.timeout,
            cancellation,
        )?;
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(TransportError::ResponseTooLarge.into());
        }
        if !(200..300).contains(&response.status) {
            return Err(AdapterError::Response(Box::new(
                self.error_response(request, response.status),
            )));
        }
        let parsed: GeminiResponse =
            serde_json::from_slice(&response.body).map_err(|_| AdapterError::MalformedResponse)?;
        let candidate = parsed
            .candidates
            .first()
            .ok_or(AdapterError::MalformedResponse)?;
        let parts: Vec<OutputPart> = candidate
            .content
            .as_ref()
            .map(|content| {
                content
                    .parts
                    .iter()
                    .filter_map(|part| part.text.clone())
                    .map(|text| OutputPart {
                        kind: OutputPartKind::Text,
                        content: text,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if parts.is_empty() {
            return Err(AdapterError::MalformedResponse);
        }
        let result = NormalizedResponse {
            schema_version: provider_core::response::NORMALIZED_RESPONSE_SCHEMA_VERSION,
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            provider_id: ProviderId::parse("gemini").expect("static provider id is valid"),
            model_id: ModelId::parse(
                parsed
                    .model_version
                    .unwrap_or_else(|| request.model_id.as_str().into()),
            )
            .map_err(|_| AdapterError::MalformedResponse)?,
            status: ResponseStatus::Complete,
            finish_reason: map_finish_reason(
                candidate.finish_reason.as_deref().unwrap_or("unknown"),
            ),
            parts,
            usage: parsed.usage_metadata.map(map_usage),
            cost: None,
            error: None,
            provider_version: GEMINI_DESCRIPTOR_VERSION.into(),
            latency_ms: None,
        };
        result
            .validate()
            .map_err(|_| AdapterError::MalformedResponse)?;
        Ok(result)
    }

    fn send_stream(
        &self,
        request: &NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<Vec<StreamEvent>, AdapterError> {
        let response = self.transport.send(
            self.build_request(request, true)?,
            self.timeout,
            cancellation,
        )?;
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(TransportError::ResponseTooLarge.into());
        }
        if !(200..300).contains(&response.status) {
            return Err(AdapterError::Response(Box::new(
                self.error_response(request, response.status),
            )));
        }
        let chunks: Vec<GeminiStreamChunk> =
            serde_json::from_slice(&response.body).map_err(|_| AdapterError::MalformedResponse)?;
        let mut validator = StreamValidator::new(&request.request_id, 1)?;
        let mut events = Vec::new();
        self.push_event(
            &mut events,
            &mut validator,
            request,
            StreamEventPayload::Start {
                provider_id: ProviderId::parse("gemini").expect("static provider id is valid"),
                model_id: request.model_id.clone(),
            },
        )?;
        for chunk in chunks {
            if cancellation.is_cancelled() {
                self.push_event(
                    &mut events,
                    &mut validator,
                    request,
                    StreamEventPayload::Cancel {
                        reason: "cancelled".into(),
                    },
                )?;
                return Ok(events);
            }
            if let Some(delta) = chunk.delta {
                self.push_event(
                    &mut events,
                    &mut validator,
                    request,
                    StreamEventPayload::Delta {
                        part: OutputPart {
                            kind: OutputPartKind::Text,
                            content: delta,
                        },
                    },
                )?;
            }
            if let Some(usage) = chunk.usage_metadata {
                self.push_event(
                    &mut events,
                    &mut validator,
                    request,
                    StreamEventPayload::Usage {
                        usage: map_usage(usage),
                    },
                )?;
            }
            if chunk.error.is_some() {
                self.push_event(
                    &mut events,
                    &mut validator,
                    request,
                    StreamEventPayload::Error {
                        error: ProviderErrorInfo {
                            code: ProviderErrorCode::ProviderRejected,
                            message: "provider stream error".into(),
                            retryable: false,
                        },
                    },
                )?;
                return Ok(events);
            }
            if let Some(reason) = chunk.finish_reason {
                self.push_event(
                    &mut events,
                    &mut validator,
                    request,
                    StreamEventPayload::Finish {
                        reason: map_finish_reason(&reason),
                    },
                )?;
                return Ok(events);
            }
        }
        if validator.is_terminal() {
            Ok(events)
        } else {
            Err(AdapterError::IncompleteStream)
        }
    }

    fn push_event(
        &self,
        events: &mut Vec<StreamEvent>,
        validator: &mut StreamValidator,
        request: &NormalizedRequest,
        payload: StreamEventPayload,
    ) -> Result<(), AdapterError> {
        let event = StreamEvent {
            schema_version: provider_core::stream::STREAM_SCHEMA_VERSION,
            stream_id: request.request_id.clone(),
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            generation: 1,
            sequence: events.len() as u64,
            payload,
        };
        validator.accept(event.clone())?;
        events.push(event);
        Ok(())
    }

    fn build_request(
        &self,
        request: &NormalizedRequest,
        stream: bool,
    ) -> Result<HttpRequest, AdapterError> {
        let contents = request
            .messages
            .iter()
            .map(|message| GeminiContent {
                role: match message.role {
                    MessageRole::Assistant => "model",
                    MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                },
                parts: vec![GeminiPart {
                    text: Some(message.content.clone()),
                }],
            })
            .collect();
        let body = GeminiRequest {
            contents,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: request.budget.max_tokens,
                temperature: request.temperature,
            },
            stream,
        };
        let body = serde_json::to_vec(&body).map_err(|_| AdapterError::InvalidRequest)?;
        validate_request_body(&body)?;
        let operation = if stream {
            "streamGenerateContent"
        } else {
            "generateContent"
        };
        Ok(HttpRequest {
            method: "POST".into(),
            url: self.endpoint.path(&format!(
                "models/{}:{}",
                request.model_id.as_str(),
                operation
            )),
            headers: BTreeMap::from([("content-type".into(), "application/json".into())]),
            body,
            credential_ref: self.credential_ref.clone(),
        })
    }

    fn error_response(&self, request: &NormalizedRequest, status: u16) -> NormalizedResponse {
        let (code, retryable, message) = match status {
            408 | 504 => (ProviderErrorCode::Timeout, true, "provider timeout"),
            401 | 403 => (
                ProviderErrorCode::Authentication,
                false,
                "provider authentication failed",
            ),
            429 => (
                ProviderErrorCode::RateLimited,
                true,
                "provider rate limited",
            ),
            400..=499 => (
                ProviderErrorCode::InvalidRequest,
                false,
                "provider rejected request",
            ),
            500..=599 => (
                ProviderErrorCode::ProviderUnavailable,
                true,
                "provider unavailable",
            ),
            _ => (
                ProviderErrorCode::Unknown,
                false,
                "unknown provider response",
            ),
        };
        NormalizedResponse {
            schema_version: provider_core::response::NORMALIZED_RESPONSE_SCHEMA_VERSION,
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            provider_id: ProviderId::parse("gemini").expect("static provider id is valid"),
            model_id: request.model_id.clone(),
            status: ResponseStatus::Error,
            finish_reason: FinishReason::Error,
            parts: Vec::new(),
            usage: None,
            cost: None,
            error: Some(ProviderErrorInfo {
                code,
                message: message.into(),
                retryable,
            }),
            provider_version: GEMINI_DESCRIPTOR_VERSION.into(),
            latency_ms: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GeminiGenerationConfig,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: &'static str,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
    #[serde(rename = "modelVersion")]
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentOwned>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContentOwned {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount", alias = "inputTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", alias = "outputTokenCount")]
    candidates_token_count: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamChunk {
    delta: Option<String>,
    #[serde(rename = "finishReason", alias = "finish_reason")]
    finish_reason: Option<String>,
    #[serde(rename = "usageMetadata", alias = "usage")]
    usage_metadata: Option<GeminiUsage>,
    error: Option<GeminiChunkError>,
}

#[derive(Debug, Deserialize)]
struct GeminiChunkError {
    #[allow(dead_code)]
    message: Option<String>,
}

fn map_usage(usage: GeminiUsage) -> Usage {
    Usage {
        input_tokens: usage.prompt_token_count,
        output_tokens: usage.candidates_token_count,
    }
}

fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "STOP" | "FINISH_REASON_UNSPECIFIED" => FinishReason::Stop,
        "MAX_TOKENS" => FinishReason::Length,
        "SAFETY" | "RECITATION" => FinishReason::ContentFilter,
        "TOOL_CALL" => FinishReason::ToolCall,
        "CANCELLED" | "CANCELED" => FinishReason::Cancelled,
        "ERROR" => FinishReason::Error,
        _ => FinishReason::Unknown,
    }
}

fn capabilities_for(provider_id: &ProviderId, model: GeminiModel) -> CapabilityReport {
    let report = CapabilityReport {
        schema_version: 1,
        provider_id: provider_id.clone(),
        model_id: ModelId::parse(model.as_str()).expect("static model id is valid"),
        version: GEMINI_DESCRIPTOR_VERSION.into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([
            (ModelModality::Text, CapabilityState::Supported),
            (ModelModality::Image, CapabilityState::Supported),
            (ModelModality::Audio, CapabilityState::Unsupported),
            (ModelModality::Video, CapabilityState::Unsupported),
        ]),
        features: BTreeMap::from([
            (CapabilityFeature::Streaming, CapabilityState::Supported),
            (CapabilityFeature::ToolUse, CapabilityState::Supported),
            (CapabilityFeature::Vision, CapabilityState::Supported),
            (CapabilityFeature::AudioInput, CapabilityState::Unsupported),
        ]),
        limits: CapabilityLimits {
            max_context_tokens: Some(match model {
                GeminiModel::Gemini15Pro => 2_000_000,
                GeminiModel::Gemini15Flash => 1_000_000,
            }),
            max_output_tokens: Some(8_192),
        },
    };
    report
        .validate()
        .expect("static capability report is valid");
    report
}

fn rewrite_adapter_error(error: AdapterError) -> ProviderDescriptorError {
    match error {
        AdapterError::Response(mut response) => {
            response.provider_id =
                ProviderId::parse("gemini").expect("static provider id is valid");
            ProviderDescriptorError::Adapter(AdapterError::Response(response))
        }
        other => ProviderDescriptorError::Adapter(other),
    }
}
