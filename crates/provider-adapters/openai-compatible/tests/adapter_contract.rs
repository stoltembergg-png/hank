use std::sync::{Arc, Mutex};
use std::time::Duration;

use provider_adapter_openai_compatible::{
    EndpointPolicy, HttpRequest, HttpResponse, HttpTransport, OpenAiCompatibleAdapter,
    TransportError,
};
use provider_core::capabilities::CapabilityRequirement;
use provider_core::request::{
    CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
};
use provider_core::response::{FinishReason, ResponseStatus};
use provider_core::stream::StreamEventPayload;
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};

#[derive(Clone)]
struct MockTransport {
    response: Arc<Mutex<Result<HttpResponse, TransportError>>>,
    requests: Arc<Mutex<Vec<HttpRequest>>>,
}

impl MockTransport {
    fn response(body: &str) -> Self {
        Self {
            response: Arc::new(Mutex::new(Ok(HttpResponse::ok(body.as_bytes().to_vec())))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn response_status(status: u16, body: &[u8]) -> Self {
        Self {
            response: Arc::new(Mutex::new(Ok(HttpResponse::with_status(
                status,
                body.to_vec(),
            )))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn error(error: TransportError) -> Self {
        Self {
            response: Arc::new(Mutex::new(Err(error))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl HttpTransport for MockTransport {
    fn send(
        &self,
        request: HttpRequest,
        _timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<HttpResponse, TransportError> {
        self.requests.lock().unwrap().push(request);
        if cancellation.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        self.response.lock().unwrap().clone()
    }
}

fn request() -> NormalizedRequest {
    NormalizedRequest {
        schema_version: 1,
        request_id: "req-1".into(),
        correlation_id: "corr-1".into(),
        project_id: "project-1".into(),
        agent_id: "agent-1".into(),
        session_id: Some("session-1".into()),
        provider_id: ProviderId::parse("openai-compatible").unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        messages: vec![NormalizedMessage {
            role: MessageRole::User,
            content: "hello".into(),
        }],
        modalities: std::collections::BTreeSet::from([
            provider_core::capabilities::ModelModality::Text,
        ]),
        capabilities: CapabilityRequirement {
            modalities: std::collections::BTreeSet::from([
                provider_core::capabilities::ModelModality::Text,
            ]),
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

fn adapter(transport: MockTransport) -> OpenAiCompatibleAdapter<MockTransport> {
    OpenAiCompatibleAdapter::new(
        EndpointPolicy::parse("https://mock.example/v1").unwrap(),
        CredentialRef::parse("cred_fixture").unwrap(),
        transport,
        Duration::from_secs(10),
    )
    .unwrap()
}

#[test]
fn complete_maps_mock_response_and_preserves_correlation_without_secret_body() {
    let transport = MockTransport::response(
        r#"{"id":"cmpl-1","model":"mock-model","choices":[{"message":{"content":"hello back"},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":3}}"#,
    );
    let requests = transport.requests.clone();
    let result = adapter(transport)
        .complete(request(), &CancellationToken::new())
        .unwrap();

    assert_eq!(result.status, ResponseStatus::Complete);
    assert_eq!(result.finish_reason, FinishReason::Stop);
    assert_eq!(result.request_id, "req-1");
    assert_eq!(result.correlation_id, "corr-1");
    assert_eq!(result.usage.unwrap().output_tokens, 3);
    let sent = requests.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert!(!String::from_utf8_lossy(&sent[0].body).contains("cred_fixture"));
    assert!(!format!("{:?}", sent[0]).contains("cred_fixture"));
}

#[test]
fn stream_maps_chunks_to_ordered_terminal_events() {
    let transport = MockTransport::response(
        r#"[{"delta":"hel"},{"delta":"lo","usage":{"prompt_tokens":2,"completion_tokens":2}},{"finish_reason":"stop"}]"#,
    );
    let events = adapter(transport)
        .stream(request(), &CancellationToken::new())
        .unwrap();
    assert!(matches!(
        events.first().unwrap().payload,
        StreamEventPayload::Start { .. }
    ));
    assert!(events
        .iter()
        .any(|event| matches!(event.payload, StreamEventPayload::Usage { .. })));
    assert!(matches!(
        events.last().unwrap().payload,
        StreamEventPayload::Finish {
            reason: FinishReason::Stop
        }
    ));
    assert!(events.last().unwrap().is_terminal());
}

#[test]
fn maps_rate_limit_timeout_and_cancel_without_retry_side_effects() {
    let rate_limited = MockTransport::response_status(429, br#"{"error":{"message":"slow down"}}"#);
    let error = adapter(rate_limited)
        .complete(request(), &CancellationToken::new())
        .unwrap_err();
    match error {
        provider_adapter_openai_compatible::AdapterError::Response(response) => {
            assert_eq!(response.status, ResponseStatus::Error);
            assert!(response.error.expect("rate limit error").retryable);
        }
        other => panic!("unexpected error: {other:?}"),
    }

    let timeout = adapter(MockTransport::error(TransportError::Timeout))
        .complete(request(), &CancellationToken::new())
        .unwrap_err();
    assert!(matches!(
        timeout,
        provider_adapter_openai_compatible::AdapterError::Transport(TransportError::Timeout)
    ));

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = adapter(MockTransport::error(TransportError::Cancelled))
        .complete(request(), &cancellation)
        .unwrap_err();
    assert!(matches!(
        cancelled,
        provider_adapter_openai_compatible::AdapterError::Transport(TransportError::Cancelled)
    ));
}

#[test]
fn malformed_response_and_incomplete_stream_fail_closed() {
    let malformed = adapter(MockTransport::response("not-json"))
        .complete(request(), &CancellationToken::new())
        .unwrap_err();
    assert!(matches!(
        malformed,
        provider_adapter_openai_compatible::AdapterError::MalformedResponse
    ));

    let incomplete = adapter(MockTransport::response(r#"[{"delta":"partial"}]"#))
        .stream(request(), &CancellationToken::new())
        .unwrap_err();
    assert!(matches!(
        incomplete,
        provider_adapter_openai_compatible::AdapterError::IncompleteStream
    ));
}

#[test]
fn endpoint_credential_and_size_limits_are_explicit() {
    assert!(EndpointPolicy::parse("http://mock.example/v1").is_err());
    assert!(EndpointPolicy::parse("https://user:password@mock.example/v1").is_err());
    assert!(CredentialRef::parse("api_key=secret").is_err());

    let oversized = MockTransport::response(&"x".repeat(2_097_153));
    let error = adapter(oversized)
        .complete(request(), &CancellationToken::new())
        .unwrap_err();
    assert!(matches!(
        error,
        provider_adapter_openai_compatible::AdapterError::Transport(
            TransportError::ResponseTooLarge
        )
    ));
}

#[test]
fn unsupported_http_status_is_typed_and_provider_payload_is_not_exposed() {
    let transport =
        MockTransport::response_status(500, br#"{"error":{"message":"api_key=secret"}}"#);
    let error = adapter(transport)
        .complete(request(), &CancellationToken::new())
        .unwrap_err();
    match error {
        provider_adapter_openai_compatible::AdapterError::Response(response) => {
            let info = response.error.unwrap();
            assert_eq!(
                info.code,
                provider_core::response::ProviderErrorCode::ProviderUnavailable
            );
            assert!(!info.message.contains("api_key"));
            assert!(!format!("{:?}", info).contains("secret"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
