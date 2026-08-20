use std::sync::{Arc, Mutex};
use std::time::Duration;

use provider_adapter_ollama::{
    OllamaModel, OllamaProvider, OllamaProviderDescriptor, ProviderDescriptorError,
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

    #[allow(dead_code)]
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
        provider_id: ProviderId::parse("ollama").unwrap(),
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

fn provider<T: HttpTransport>(transport: T) -> OllamaProvider<T> {
    OllamaProvider::new(
        EndpointPolicy::parse("https://mock-ollama.local").unwrap(),
        CredentialRef::parse("cred_fixture").unwrap(),
        transport,
        Duration::from_secs(10),
    )
    .unwrap()
}

#[test]
fn descriptor_exposes_ollama_models_and_explicit_capabilities() {
    let descriptor = OllamaProviderDescriptor::new();
    assert_eq!(descriptor.provider_id().as_str(), "ollama");
    assert_eq!(descriptor.version(), "ollama-descriptor-1");
    assert_eq!(descriptor.models().len(), 2);
    let model = descriptor.model(OllamaModel::Llama318b);
    assert_eq!(model.as_str(), "llama3.1:8b");
    let capabilities = descriptor.capabilities(OllamaModel::Llama318b).unwrap();
    assert_eq!(
        capabilities.feature_state(CapabilityFeature::Streaming),
        CapabilityState::Supported
    );
    assert_eq!(
        capabilities.modality_state(ModelModality::Text),
        CapabilityState::Supported
    );
    assert_eq!(
        capabilities.modality_state(ModelModality::Image),
        CapabilityState::Unsupported
    );
}

#[test]
fn descriptor_rejects_wrong_provider_unknown_model_and_unsupported_capability() {
    let descriptor = OllamaProviderDescriptor::new();
    let mut wrong = request("llama3.1:8b");
    wrong.provider_id = ProviderId::parse("other").unwrap();
    assert!(matches!(
        descriptor.validate_request(&wrong),
        Err(ProviderDescriptorError::ProviderMismatch)
    ));
    assert!(matches!(
        descriptor.validate_request(&request("unknown/model")),
        Err(ProviderDescriptorError::UnsupportedModel(_))
    ));

    let mut image = request("llama3.1:8b");
    image.modalities = std::collections::BTreeSet::from([ModelModality::Image]);
    image.capabilities.modalities = std::collections::BTreeSet::from([ModelModality::Image]);
    assert!(matches!(
        descriptor.validate_request(&image),
        Err(ProviderDescriptorError::UnsupportedCapability(_))
    ));
}

#[test]
fn complete_maps_ollama_response_and_preserves_identity() {
    let transport = MockTransport::response(
        br#"{"model":"llama3.1:8b","message":{"role":"assistant","content":"ok"},"done":true,"eval_count":5}"#,
    );
    let result = provider(transport)
        .complete(request("llama3.1:8b"), &CancellationToken::new())
        .unwrap();
    assert_eq!(result.provider_id.as_str(), "ollama");
    assert_eq!(result.model_id.as_str(), "llama3.1:8b");
    assert_eq!(result.parts.len(), 1);
    assert_eq!(result.parts[0].content, "ok");
}

#[test]
fn stream_maps_ollama_chunks_and_terminal_event() {
    let transport = MockTransport::response(
        br#"[{"model":"llama3.1:8b","message":{"role":"assistant","content":"chunk1"},"done":false},{"model":"llama3.1:8b","message":{"role":"assistant","content":"chunk2"},"done":true}]"#,
    );
    let events = provider(transport)
        .stream(request("llama3.2:3b"), &CancellationToken::new())
        .unwrap();
    match &events[0].payload {
        StreamEventPayload::Start {
            provider_id,
            model_id,
        } => {
            assert_eq!(provider_id.as_str(), "ollama");
            assert_eq!(model_id.as_str(), "llama3.2:3b");
        }
        payload => panic!("unexpected payload: {payload:?}"),
    }
    assert!(events.last().unwrap().is_terminal());
}

#[test]
fn endpoint_policy_rejects_invalid_and_malformed() {
    assert!(EndpointPolicy::parse("https://mock-ollama.local").is_ok());
    assert!(EndpointPolicy::parse("https://example.com").is_ok()); // format is valid
    assert!(EndpointPolicy::parse("http://remote:11434").is_err()); // http not allowed
    assert!(CredentialRef::parse("api_key=plaintext").is_err());
}
