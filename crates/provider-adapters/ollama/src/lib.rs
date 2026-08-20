//! Isolated Ollama adapter with localhost endpoint validation.

use provider_core::capabilities::{
    CapabilityError, CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::request::NormalizedRequest;
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

const OLLAMA_DESCRIPTOR_VERSION: &str = "ollama-descriptor-1";
const MAX_RESPONSE_BYTES: usize = 2_097_152;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OllamaModel {
    Llama318b,
    Llama323b,
}

impl OllamaModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Llama318b => "llama3.1:8b",
            Self::Llama323b => "llama3.2:3b",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OllamaModelDescriptor {
    pub model_id: ModelId,
    pub capabilities: CapabilityReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OllamaProviderDescriptor {
    provider_id: ProviderId,
    version: String,
    models: Vec<OllamaModelDescriptor>,
}

#[derive(Debug, Error)]
pub enum ProviderDescriptorError {
    #[error("normalized request targets another provider")]
    ProviderMismatch,
    #[error("Ollama model is not declared: {0}")]
    UnsupportedModel(String),
    #[error("normalized request capability is unsupported: {0:?}")]
    UnsupportedCapability(CapabilityError),
    #[error("normalized request is invalid")]
    InvalidRequest,
    #[error("Ollama adapter error: {0}")]
    Adapter(#[from] AdapterError),
}

impl OllamaProviderDescriptor {
    pub fn new() -> Self {
        let provider_id = ProviderId::parse("ollama").expect("static provider id is valid");
        let models = [OllamaModel::Llama318b, OllamaModel::Llama323b]
            .into_iter()
            .map(|model| OllamaModelDescriptor {
                model_id: ModelId::parse(model.as_str()).expect("static model id is valid"),
                capabilities: capabilities_for(&provider_id, model),
            })
            .collect();
        Self {
            provider_id,
            version: OLLAMA_DESCRIPTOR_VERSION.into(),
            models,
        }
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn models(&self) -> &[OllamaModelDescriptor] {
        &self.models
    }

    pub fn model(&self, model: OllamaModel) -> ModelId {
        ModelId::parse(model.as_str()).expect("static model id is valid")
    }

    pub fn capabilities(
        &self,
        model: OllamaModel,
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

impl Default for OllamaProviderDescriptor {
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
    #[error("malformed Ollama response")]
    MalformedResponse,
    #[error("normalized response: {0:?}")]
    Response(Box<NormalizedResponse>),
    #[error("stream validation error: {0}")]
    Stream(#[from] StreamValidationError),
    #[error("Ollama stream ended without a terminal event")]
    IncompleteStream,
}

pub struct OllamaProvider<T> {
    descriptor: OllamaProviderDescriptor,
    endpoint: EndpointPolicy,
    credential_ref: CredentialRef,
    transport: T,
    timeout: Duration,
}

impl<T: HttpTransport> OllamaProvider<T> {
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
            descriptor: OllamaProviderDescriptor::new(),
            endpoint,
            credential_ref,
            transport,
            timeout,
        })
    }

    pub fn descriptor(&self) -> &OllamaProviderDescriptor {
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
                    if let StreamEventPayload::Start {
                        provider_id,
                        model_id,
                    } = &mut event.payload
                    {
                        *provider_id = self.descriptor.provider_id.clone();
                        *model_id = request.model_id.clone();
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
        let parsed: OllamaResponse =
            serde_json::from_slice(&response.body).map_err(|_| AdapterError::MalformedResponse)?;
        let parts = vec![OutputPart {
            kind: OutputPartKind::Text,
            content: parsed.message.content,
        }];
        let result = NormalizedResponse {
            schema_version: provider_core::response::NORMALIZED_RESPONSE_SCHEMA_VERSION,
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            provider_id: ProviderId::parse("ollama").expect("static provider id is valid"),
            model_id: ModelId::parse(
                parsed
                    .model
                    .unwrap_or_else(|| request.model_id.as_str().into()),
            )
            .map_err(|_| AdapterError::MalformedResponse)?,
            status: ResponseStatus::Complete,
            finish_reason: if parsed.done {
                FinishReason::Stop
            } else {
                FinishReason::Unknown
            },
            parts,
            usage: parsed.eval_count.map(|c| Usage {
                input_tokens: 0,
                output_tokens: c,
            }),
            cost: None,
            error: None,
            provider_version: OLLAMA_DESCRIPTOR_VERSION.into(),
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
        let chunks: Vec<OllamaStreamChunk> =
            serde_json::from_slice(&response.body).map_err(|_| AdapterError::MalformedResponse)?;
        let mut validator = StreamValidator::new(&request.request_id, 1)?;
        let mut events = Vec::new();
        self.push_event(
            &mut events,
            &mut validator,
            request,
            StreamEventPayload::Start {
                provider_id: ProviderId::parse("ollama").expect("static provider id is valid"),
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
            self.push_event(
                &mut events,
                &mut validator,
                request,
                StreamEventPayload::Delta {
                    part: OutputPart {
                        kind: OutputPartKind::Text,
                        content: chunk.message.content,
                    },
                },
            )?;
            if chunk.done {
                self.push_event(
                    &mut events,
                    &mut validator,
                    request,
                    StreamEventPayload::Finish {
                        reason: FinishReason::Stop,
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
        let messages = request
            .messages
            .iter()
            .map(|message| OllamaMessage {
                role: match message.role {
                    provider_core::request::MessageRole::Assistant => "assistant".into(),
                    provider_core::request::MessageRole::System => "system".into(),
                    provider_core::request::MessageRole::User => "user".into(),
                    provider_core::request::MessageRole::Tool => "tool".into(),
                },
                content: message.content.clone(),
            })
            .collect();
        let body = OllamaRequest {
            model: request.model_id.as_str().into(),
            messages,
            stream,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.budget.max_tokens,
            },
        };
        let body = serde_json::to_vec(&body).map_err(|_| AdapterError::InvalidRequest)?;
        validate_request_body(&body)?;
        Ok(HttpRequest {
            method: "POST".into(),
            url: self.endpoint.path("api/chat"),
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
            provider_id: ProviderId::parse("ollama").expect("static provider id is valid"),
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
            provider_version: OLLAMA_DESCRIPTOR_VERSION.into(),
            latency_ms: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: Option<f32>,
    #[serde(rename = "num_predict")]
    num_predict: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: Option<String>,
    message: OllamaMessage,
    done: bool,
    #[serde(rename = "eval_count")]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OllamaStreamChunk {
    model: String,
    message: OllamaMessage,
    done: bool,
}

fn capabilities_for(provider_id: &ProviderId, model: OllamaModel) -> CapabilityReport {
    let report = CapabilityReport {
        schema_version: 1,
        provider_id: provider_id.clone(),
        model_id: ModelId::parse(model.as_str()).expect("static model id is valid"),
        version: OLLAMA_DESCRIPTOR_VERSION.into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([
            (ModelModality::Text, CapabilityState::Supported),
            (ModelModality::Image, CapabilityState::Unsupported),
            (ModelModality::Audio, CapabilityState::Unsupported),
            (ModelModality::Video, CapabilityState::Unsupported),
        ]),
        features: BTreeMap::from([
            (CapabilityFeature::Streaming, CapabilityState::Supported),
            (CapabilityFeature::ToolUse, CapabilityState::Unsupported),
            (CapabilityFeature::Vision, CapabilityState::Unsupported),
            (CapabilityFeature::AudioInput, CapabilityState::Unsupported),
        ]),
        limits: CapabilityLimits {
            max_context_tokens: Some(32_768),
            max_output_tokens: Some(2_048),
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
                ProviderId::parse("ollama").expect("static provider id is valid");
            ProviderDescriptorError::Adapter(AdapterError::Response(response))
        }
        other => ProviderDescriptorError::Adapter(other),
    }
}
