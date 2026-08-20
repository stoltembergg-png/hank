//! Isolated OpenAI-compatible adapter.
//!
//! This crate owns protocol mapping only. Transport, credential resolution,
//! persistence, UI, retries, and tool execution stay outside provider-core.

use provider_core::request::{MessageRole, NormalizedRequest};
use provider_core::response::{
    FinishReason, NormalizedResponse, OutputPart, OutputPartKind, ProviderErrorCode,
    ProviderErrorInfo, ResponseStatus, Usage,
};
use provider_core::stream::{
    StreamEvent, StreamEventPayload, StreamValidationError, StreamValidator,
};
use provider_core::transport::validate_request_body;
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use thiserror::Error;

pub use provider_core::transport::{
    EndpointError, EndpointPolicy, HttpRequest, HttpResponse, HttpTransport, TransportError,
};

const MAX_RESPONSE_BYTES: usize = 2_097_152;
const OPENAI_PROVIDER_VERSION: &str = "openai-compatible-1";

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("invalid endpoint: {0}")]
    Endpoint(#[from] EndpointError),
    #[error("invalid credential reference")]
    Credential,
    #[error("invalid normalized request")]
    InvalidRequest,
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("malformed provider response")]
    MalformedResponse,
    #[error("normalized response: {0:?}")]
    Response(Box<NormalizedResponse>),
    #[error("stream validation error: {0}")]
    Stream(#[from] StreamValidationError),
    #[error("provider stream ended without a terminal event")]
    IncompleteStream,
}

pub struct OpenAiCompatibleAdapter<T> {
    endpoint: EndpointPolicy,
    credential_ref: CredentialRef,
    transport: T,
    timeout: Duration,
}

impl<T: HttpTransport> OpenAiCompatibleAdapter<T> {
    pub fn new(
        endpoint: EndpointPolicy,
        credential_ref: CredentialRef,
        transport: T,
        timeout: Duration,
    ) -> Result<Self, AdapterError> {
        if timeout.is_zero() {
            return Err(AdapterError::Transport(TransportError::Timeout));
        }
        Ok(Self {
            endpoint,
            credential_ref,
            transport,
            timeout,
        })
    }

    pub fn complete(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<NormalizedResponse, AdapterError> {
        request
            .validate()
            .map_err(|_| AdapterError::InvalidRequest)?;
        let http_request = self.build_request(&request, false)?;
        let response = self
            .transport
            .send(http_request, self.timeout, cancellation)?;
        self.map_complete_response(&request, response)
    }

    pub fn stream(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<Vec<StreamEvent>, AdapterError> {
        request
            .validate()
            .map_err(|_| AdapterError::InvalidRequest)?;
        let http_request = self.build_request(&request, true)?;
        let response = self
            .transport
            .send(http_request, self.timeout, cancellation)?;
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(TransportError::ResponseTooLarge.into());
        }
        if !(200..300).contains(&response.status) {
            return Err(AdapterError::Response(Box::new(
                self.error_response(&request, response.status),
            )));
        }
        let chunks: Vec<OpenAiStreamChunk> =
            serde_json::from_slice(&response.body).map_err(|_| AdapterError::MalformedResponse)?;
        let stream_id = request.request_id.clone();
        let mut validator = StreamValidator::new(&stream_id, 1)?;
        let mut events = Vec::new();
        self.push_stream_event(
            &mut events,
            &mut validator,
            &request,
            StreamEventPayload::Start {
                provider_id: ProviderId::parse("openai-compatible")
                    .expect("static provider id is valid"),
                model_id: request.model_id.clone(),
            },
        )?;

        for chunk in chunks {
            if cancellation.is_cancelled() {
                self.push_stream_event(
                    &mut events,
                    &mut validator,
                    &request,
                    StreamEventPayload::Cancel {
                        reason: "cancelled".into(),
                    },
                )?;
                return Ok(events);
            }
            if let Some(delta) = chunk.delta {
                self.push_stream_event(
                    &mut events,
                    &mut validator,
                    &request,
                    StreamEventPayload::Delta {
                        part: OutputPart {
                            kind: OutputPartKind::Text,
                            content: delta,
                        },
                    },
                )?;
            }
            if let Some(tool) = chunk.tool_request {
                self.push_stream_event(
                    &mut events,
                    &mut validator,
                    &request,
                    StreamEventPayload::ToolRequest {
                        tool_id: tool.tool_id,
                        capability_fingerprint: tool.capability_fingerprint,
                        context: tool.context,
                    },
                )?;
            }
            if let Some(usage) = chunk.usage {
                self.push_stream_event(
                    &mut events,
                    &mut validator,
                    &request,
                    StreamEventPayload::Usage {
                        usage: map_usage(usage),
                    },
                )?;
            }
            if chunk.error.is_some() {
                self.push_stream_event(
                    &mut events,
                    &mut validator,
                    &request,
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
                self.push_stream_event(
                    &mut events,
                    &mut validator,
                    &request,
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

    fn push_stream_event(
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
        let body = OpenAiChatRequest {
            model: request.model_id.as_str().to_owned(),
            messages: request
                .messages
                .iter()
                .map(|message| OpenAiMessage {
                    role: map_role(message.role),
                    content: message.content.clone(),
                })
                .collect(),
            max_tokens: request.budget.max_tokens,
            temperature: request.temperature,
            stream,
            tools: request
                .tools
                .iter()
                .map(|tool| OpenAiTool {
                    type_name: "function",
                    name: tool.tool_id.clone(),
                    capability_fingerprint: tool.capability_fingerprint.clone(),
                })
                .collect(),
        };
        let body = serde_json::to_vec(&body).map_err(|_| AdapterError::InvalidRequest)?;
        validate_request_body(&body)?;
        let mut headers = BTreeMap::new();
        headers.insert("content-type".into(), "application/json".into());
        Ok(HttpRequest {
            method: "POST".into(),
            url: self.endpoint.path("chat/completions"),
            headers,
            body,
            credential_ref: self.credential_ref.clone(),
        })
    }

    fn map_complete_response(
        &self,
        request: &NormalizedRequest,
        response: HttpResponse,
    ) -> Result<NormalizedResponse, AdapterError> {
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(TransportError::ResponseTooLarge.into());
        }
        if !(200..300).contains(&response.status) {
            return Err(AdapterError::Response(Box::new(
                self.error_response(request, response.status),
            )));
        }
        let parsed: OpenAiCompletionResponse =
            serde_json::from_slice(&response.body).map_err(|_| AdapterError::MalformedResponse)?;
        let choice = parsed
            .choices
            .first()
            .ok_or(AdapterError::MalformedResponse)?;
        let content = choice
            .message
            .content
            .clone()
            .ok_or(AdapterError::MalformedResponse)?;
        let result = NormalizedResponse {
            schema_version: provider_core::response::NORMALIZED_RESPONSE_SCHEMA_VERSION,
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            provider_id: ProviderId::parse("openai-compatible")
                .expect("static provider id is valid"),
            model_id: ModelId::parse(parsed.model).map_err(|_| AdapterError::MalformedResponse)?,
            status: ResponseStatus::Complete,
            finish_reason: map_finish_reason(choice.finish_reason.as_deref().unwrap_or("unknown")),
            parts: vec![OutputPart {
                kind: OutputPartKind::Text,
                content,
            }],
            usage: parsed.usage.map(map_usage),
            cost: None,
            error: None,
            provider_version: OPENAI_PROVIDER_VERSION.into(),
            latency_ms: None,
        };
        result
            .validate()
            .map_err(|_| AdapterError::MalformedResponse)?;
        Ok(result)
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
            provider_id: ProviderId::parse("openai-compatible")
                .expect("static provider id is valid"),
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
            provider_version: OPENAI_PROVIDER_VERSION.into(),
            latency_ms: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: bool,
    tools: Vec<OpenAiTool>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    type_name: &'static str,
    name: String,
    capability_fingerprint: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiCompletionResponse {
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageResponse {
    content: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    delta: Option<String>,
    finish_reason: Option<String>,
    usage: Option<OpenAiUsage>,
    tool_request: Option<OpenAiToolRequest>,
    error: Option<OpenAiChunkError>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolRequest {
    tool_id: String,
    capability_fingerprint: String,
    context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChunkError {
    #[allow(dead_code)]
    message: Option<String>,
}

fn map_role(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn map_usage(usage: OpenAiUsage) -> Usage {
    Usage {
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
    }
}

fn map_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "content_filter" => FinishReason::ContentFilter,
        "tool_calls" | "function_call" => FinishReason::ToolCall,
        "cancelled" | "canceled" => FinishReason::Cancelled,
        "error" => FinishReason::Error,
        _ => FinishReason::Unknown,
    }
}
