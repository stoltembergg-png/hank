use std::sync::{Arc, Mutex};
use std::time::Duration;

use provider_adapter_openrouter::{
    OpenRouterModel, OpenRouterProvider, OpenRouterProviderDescriptor, ProviderDescriptorError,
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
        provider_id: ProviderId::parse("openrouter").unwrap(),
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

fn provider<T: HttpTransport>(transport: T) -> OpenRouterProvider<T> {
    OpenRouterProvider::new(
        EndpointPolicy::parse("https://mock.example/api/v1").unwrap(),
        CredentialRef::parse("cred_fixture").unwrap(),
        transport,
        Duration::from_secs(10),
    )
    .unwrap()
}

#[test]
fn descriptor_exposes_route_identity_and_capabilities_without_fallbacks() {
    let descriptor = OpenRouterProviderDescriptor::new();
    assert_eq!(descriptor.provider_id().as_str(), "openrouter");
    assert_eq!(descriptor.version(), "openrouter-descriptor-1");
    assert_eq!(descriptor.models().len(), 2);
    let route = descriptor.route(OpenRouterModel::OpenAiGpt4oMini).unwrap();
    assert_eq!(route.upstream_provider, "openai");
    assert_eq!(route.upstream_model.as_str(), "gpt-4o-mini");
    assert_eq!(route.route_label, "direct");
    let capabilities = descriptor
        .capabilities(OpenRouterModel::OpenAiGpt4oMini)
        .unwrap();
    assert_eq!(
        capabilities.feature_state(CapabilityFeature::Streaming),
        CapabilityState::Supported
    );
    assert_eq!(
        capabilities.modality_state(ModelModality::Text),
        CapabilityState::Supported
    );
}

#[test]
fn descriptor_rejects_wrong_provider_unknown_route_and_unsupported_capability() {
    let descriptor = OpenRouterProviderDescriptor::new();
    let mut wrong = request("openai/gpt-4o-mini");
    wrong.provider_id = ProviderId::parse("other").unwrap();
    assert!(matches!(
        descriptor.validate_request(&wrong),
        Err(ProviderDescriptorError::ProviderMismatch)
    ));
    assert!(matches!(
        descriptor.validate_request(&request("unknown/model")),
        Err(ProviderDescriptorError::UnsupportedRoute(_))
    ));

    let mut image = request("openai/gpt-4o-mini");
    image.modalities = std::collections::BTreeSet::from([ModelModality::Audio]);
    image.capabilities.modalities = std::collections::BTreeSet::from([ModelModality::Audio]);
    assert!(matches!(
        descriptor.validate_request(&image),
        Err(ProviderDescriptorError::UnsupportedCapability(_))
    ));
}

#[test]
fn complete_preserves_logical_route_identity_and_maps_upstream_response() {
    let transport = MockTransport::response(
        br#"{"id":"cmpl-1","model":"gpt-4o-mini","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#,
    );
    let result = provider(transport)
        .complete(request("openai/gpt-4o-mini"), &CancellationToken::new())
        .unwrap();
    assert_eq!(result.provider_id.as_str(), "openrouter");
    assert_eq!(result.model_id.as_str(), "openai/gpt-4o-mini");
}

#[test]
fn stream_preserves_route_identity_and_terminal_event() {
    let transport = MockTransport::response(br#"[{"delta":"ok"},{"finish_reason":"stop"}]"#);
    let events = provider(transport)
        .stream(
            request("anthropic/claude-3-5-sonnet"),
            &CancellationToken::new(),
        )
        .unwrap();
    match &events[0].payload {
        StreamEventPayload::Start {
            provider_id,
            model_id,
        } => {
            assert_eq!(provider_id.as_str(), "openrouter");
            assert_eq!(model_id.as_str(), "anthropic/claude-3-5-sonnet");
        }
        payload => panic!("unexpected payload: {payload:?}"),
    }
    assert!(events.last().unwrap().is_terminal());
}

#[test]
fn upstream_errors_are_explicit_retryable_and_no_secret_route_is_injected() {
    let rate = provider(MockTransport::status(
        429,
        br#"{"error":{"message":"secret"}}"#,
    ))
    .complete(request("openai/gpt-4o-mini"), &CancellationToken::new())
    .unwrap_err();
    match rate {
        ProviderDescriptorError::Adapter(provider_adapter_openrouter::AdapterError::Response(
            response,
        )) => {
            assert!(response.error.expect("rate limit error").retryable);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(EndpointPolicy::parse("http://mock.example/api/v1").is_err());
    assert!(CredentialRef::parse("api_key=plaintext").is_err());
}
