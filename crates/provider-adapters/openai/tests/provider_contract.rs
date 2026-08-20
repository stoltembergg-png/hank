use std::sync::{Arc, Mutex};
use std::time::Duration;

use provider_adapter_openai::{
    OpenAiModel, OpenAiProvider, OpenAiProviderDescriptor, ProviderDescriptorError,
};
use provider_adapter_openai_compatible::{
    EndpointPolicy, HttpRequest, HttpResponse, HttpTransport, TransportError,
};
use provider_core::capabilities::{CapabilityFeature, CapabilityState, ModelModality};
use provider_core::request::{
    CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
};
use provider_core::stream::StreamEventPayload;
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};

#[derive(Clone)]
struct MockTransport {
    body: Arc<Mutex<Vec<u8>>>,
}

impl MockTransport {
    fn new(body: &[u8]) -> Self {
        Self {
            body: Arc::new(Mutex::new(body.to_vec())),
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
        Ok(HttpResponse::ok(self.body.lock().unwrap().clone()))
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
        provider_id: ProviderId::parse("openai").unwrap(),
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

fn provider<T: HttpTransport>(transport: T) -> OpenAiProvider<T> {
    OpenAiProvider::new(
        EndpointPolicy::parse("https://mock.example/v1").unwrap(),
        CredentialRef::parse("cred_fixture").unwrap(),
        transport,
        Duration::from_secs(10),
    )
    .unwrap()
}

#[test]
fn descriptor_exposes_deterministic_models_and_capabilities() {
    let descriptor = OpenAiProviderDescriptor::new();
    assert_eq!(descriptor.provider_id().as_str(), "openai");
    assert_eq!(descriptor.version(), "openai-descriptor-1");
    assert_eq!(descriptor.models().len(), 2);
    assert_eq!(
        descriptor.model(OpenAiModel::Gpt4oMini).as_str(),
        "gpt-4o-mini"
    );

    let mini = descriptor.capabilities(OpenAiModel::Gpt4oMini).unwrap();
    assert_eq!(
        mini.modality_state(ModelModality::Text),
        CapabilityState::Supported
    );
    assert_eq!(
        mini.modality_state(ModelModality::Image),
        CapabilityState::Unsupported
    );
    assert_eq!(
        mini.feature_state(CapabilityFeature::Streaming),
        CapabilityState::Supported
    );
}

#[test]
fn descriptor_rejects_wrong_provider_unknown_model_and_unsupported_capability() {
    let descriptor = OpenAiProviderDescriptor::new();
    let mut wrong_provider = request("gpt-4o-mini");
    wrong_provider.provider_id = ProviderId::parse("other").unwrap();
    assert!(matches!(
        descriptor.validate_request(&wrong_provider),
        Err(ProviderDescriptorError::ProviderMismatch)
    ));

    assert!(matches!(
        descriptor.validate_request(&request("unknown")),
        Err(ProviderDescriptorError::UnsupportedModel(_))
    ));

    let mut image = request("gpt-4o-mini");
    image.modalities = std::collections::BTreeSet::from([ModelModality::Image]);
    image.capabilities.modalities = std::collections::BTreeSet::from([ModelModality::Image]);
    assert!(matches!(
        descriptor.validate_request(&image),
        Err(ProviderDescriptorError::UnsupportedCapability(_))
    ));
}

#[test]
fn provider_wrapper_maps_complete_and_rewrites_provider_identity() {
    let transport = MockTransport::new(
        br#"{"id":"cmpl-1","model":"gpt-4o-mini","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#,
    );
    let result = provider(transport)
        .complete(request("gpt-4o-mini"), &CancellationToken::new())
        .unwrap();
    assert_eq!(result.provider_id.as_str(), "openai");
    assert_eq!(result.model_id.as_str(), "gpt-4o-mini");
}

#[test]
fn provider_wrapper_maps_stream_and_preserves_openai_identity() {
    let transport = MockTransport::new(br#"[{"delta":"ok"},{"finish_reason":"stop"}]"#);
    let events = provider(transport)
        .stream(request("gpt-4o"), &CancellationToken::new())
        .unwrap();
    match &events[0].payload {
        StreamEventPayload::Start { provider_id, .. } => assert_eq!(provider_id.as_str(), "openai"),
        payload => panic!("unexpected first payload: {payload:?}"),
    }
    assert!(events.last().unwrap().is_terminal());
}

#[test]
fn provider_descriptor_rejects_invalid_endpoint_and_credential_without_defaults() {
    assert!(EndpointPolicy::parse("http://api.openai.example/v1").is_err());
    assert!(CredentialRef::parse("api_key=plaintext").is_err());
    let descriptor = OpenAiProviderDescriptor::new();
    assert!(descriptor
        .capabilities(OpenAiModel::Gpt4o)
        .unwrap()
        .features
        .values()
        .all(|state| *state != CapabilityState::Unknown));
}
