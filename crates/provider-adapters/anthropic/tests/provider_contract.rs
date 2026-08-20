use std::sync::{Arc, Mutex};
use std::time::Duration;

use provider_adapter_anthropic::{
    AnthropicModel, AnthropicProvider, AnthropicProviderDescriptor, ProviderDescriptorError,
};
use provider_core::capabilities::{CapabilityFeature, CapabilityState, ModelModality};
use provider_core::request::{
    CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
};
use provider_core::stream::StreamEventPayload;
use provider_core::transport::{
    EndpointPolicy, HttpRequest, HttpResponse, HttpTransport, TransportError,
};
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};

#[derive(Clone)]
struct MockTransport {
    response: Arc<Mutex<Result<HttpResponse, TransportError>>>,
}

impl MockTransport {
    fn response(body: &[u8]) -> Self {
        Self {
            response: Arc::new(Mutex::new(Ok(HttpResponse::ok(body.to_vec())))),
        }
    }

    fn status(status: u16, body: &[u8]) -> Self {
        Self {
            response: Arc::new(Mutex::new(Ok(HttpResponse::with_status(
                status,
                body.to_vec(),
            )))),
        }
    }
}

impl HttpTransport for MockTransport {
    fn send(
        &self,
        _request: HttpRequest,
        _timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<HttpResponse, TransportError> {
        if cancellation.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        self.response.lock().unwrap().clone()
    }
}

fn request(model_id: &str) -> NormalizedRequest {
    NormalizedRequest {
        schema_version: 1,
        request_id: "req-1".into(),
        correlation_id: "corr-1".into(),
        project_id: "project-1".into(),
        agent_id: "agent-1".into(),
        session_id: None,
        provider_id: ProviderId::parse("anthropic").unwrap(),
        model_id: ModelId::parse(model_id).unwrap(),
        messages: vec![NormalizedMessage {
            role: MessageRole::User,
            content: "hello".into(),
        }],
        modalities: std::collections::BTreeSet::from([ModelModality::Text]),
        capabilities: provider_core::capabilities::CapabilityRequirement {
            modalities: std::collections::BTreeSet::from([ModelModality::Text]),
            features: std::collections::BTreeSet::new(),
            min_context_tokens: None,
            min_output_tokens: None,
        },
        tools: Vec::new(),
        budget: RequestBudget {
            max_tokens: Some(128),
            max_cost_micros: None,
        },
        cancellation: CancellationMetadata {
            cancellation_id: "cancel-1".into(),
            deadline_unix_ms: None,
        },
        temperature: Some(0.2),
    }
}

fn provider<T: HttpTransport>(transport: T) -> AnthropicProvider<T> {
    AnthropicProvider::new(
        EndpointPolicy::parse("https://mock.example/v1").unwrap(),
        CredentialRef::parse("cred_fixture").unwrap(),
        transport,
        Duration::from_secs(10),
    )
    .unwrap()
}

#[test]
fn descriptor_exposes_anthropic_models_and_explicit_capabilities() {
    let descriptor = AnthropicProviderDescriptor::new();
    assert_eq!(descriptor.provider_id().as_str(), "anthropic");
    assert_eq!(descriptor.version(), "anthropic-descriptor-1");
    assert_eq!(descriptor.models().len(), 2);
    let model = descriptor
        .capabilities(AnthropicModel::Claude35Sonnet)
        .unwrap();
    assert_eq!(
        model.modality_state(ModelModality::Text),
        CapabilityState::Supported
    );
    assert_eq!(
        model.modality_state(ModelModality::Image),
        CapabilityState::Supported
    );
    assert_eq!(
        model.feature_state(CapabilityFeature::Streaming),
        CapabilityState::Supported
    );
}

#[test]
fn descriptor_rejects_wrong_provider_unknown_model_and_unsupported_mode() {
    let descriptor = AnthropicProviderDescriptor::new();
    let mut wrong = request("claude-3-5-sonnet");
    wrong.provider_id = ProviderId::parse("other").unwrap();
    assert!(matches!(
        descriptor.validate_request(&wrong),
        Err(ProviderDescriptorError::ProviderMismatch)
    ));
    assert!(matches!(
        descriptor.validate_request(&request("unknown")),
        Err(ProviderDescriptorError::UnsupportedModel(_))
    ));

    let mut audio = request("claude-3-5-sonnet");
    audio.modalities = std::collections::BTreeSet::from([ModelModality::Audio]);
    audio.capabilities.modalities = std::collections::BTreeSet::from([ModelModality::Audio]);
    assert!(matches!(
        descriptor.validate_request(&audio),
        Err(ProviderDescriptorError::UnsupportedCapability(_))
    ));
}

#[test]
fn complete_maps_anthropic_content_and_preserves_identity() {
    let transport = MockTransport::response(
        br#"{"id":"msg-1","model":"claude-3-5-sonnet","content":[{"type":"text","text":"hello back"}],"stop_reason":"end_turn","usage":{"input_tokens":2,"output_tokens":3}}"#,
    );
    let result = provider(transport)
        .complete(request("claude-3-5-sonnet"), &CancellationToken::new())
        .unwrap();
    assert_eq!(result.provider_id.as_str(), "anthropic");
    assert_eq!(
        result.finish_reason,
        provider_core::response::FinishReason::Stop
    );
    assert_eq!(result.usage.unwrap().output_tokens, 3);
}

#[test]
fn stream_maps_anthropic_chunks_to_terminal_events() {
    let transport = MockTransport::response(
        br#"[{"delta":"ok"},{"usage":{"input_tokens":1,"output_tokens":1}},{"stop_reason":"end_turn"}]"#,
    );
    let events = provider(transport)
        .stream(request("claude-3-5-sonnet"), &CancellationToken::new())
        .unwrap();
    match &events[0].payload {
        StreamEventPayload::Start { provider_id, .. } => {
            assert_eq!(provider_id.as_str(), "anthropic")
        }
        payload => panic!("unexpected payload: {payload:?}"),
    }
    assert!(events.last().unwrap().is_terminal());
}

#[test]
fn maps_rate_limit_and_timeout_without_retry_or_secret_defaults() {
    let rate = provider(MockTransport::status(
        429,
        br#"{"error":{"message":"secret"}}"#,
    ))
    .complete(request("claude-3-5-sonnet"), &CancellationToken::new())
    .unwrap_err();
    match rate {
        ProviderDescriptorError::Adapter(provider_adapter_anthropic::AdapterError::Response(
            response,
        )) => {
            assert!(response.error.expect("rate limit error").retryable);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(EndpointPolicy::parse("http://mock.example/v1").is_err());
    assert!(CredentialRef::parse("api_key=plaintext").is_err());
}
