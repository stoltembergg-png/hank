use futures_util::StreamExt;
use provider_core::{
    CancellationToken, CredentialRef, HealthStatus, MockProvider, ModelId, ModelProvider,
    ModelProviderError, ProviderId, ProviderRequest, StreamConfig,
};

fn request() -> ProviderRequest {
    ProviderRequest::new(
        "req-1",
        ModelId::parse("mock-model").unwrap(),
        CredentialRef::parse("cred_test_ref").unwrap(),
        "hello provider",
    )
    .unwrap()
}

#[tokio::test]
async fn mock_provider_compiles_against_object_safe_trait_and_supports_lifecycle() {
    let provider: Box<dyn ModelProvider> = Box::new(MockProvider::new(
        ProviderId::parse("mock-provider").unwrap(),
        "0.1",
    ));

    assert_eq!(provider.provider_id().as_str(), "mock-provider");
    assert!(provider.capabilities().supports_completion);
    assert!(provider.capabilities().supports_streaming);
    assert_eq!(provider.health().await.unwrap(), HealthStatus::Healthy);
    assert_eq!(provider.list_models().await.unwrap().len(), 1);

    let response = provider
        .complete(request(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.model_id.as_str(), "mock-model");
    assert!(response.text.contains("hello provider"));

    let mut stream = provider
        .stream(
            request(),
            CancellationToken::new(),
            StreamConfig::new(8).unwrap(),
        )
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    let terminal = stream.next().await.unwrap().unwrap();
    assert_eq!(first.sequence, 0);
    assert!(!first.terminal);
    assert!(terminal.terminal);
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn cancellation_is_explicit_and_does_not_call_after_cancel() {
    let provider = MockProvider::new(ProviderId::parse("mock-provider").unwrap(), "0.1");
    let token = CancellationToken::new();
    token.cancel();

    let error = provider.complete(request(), token).await.unwrap_err();
    assert_eq!(error, ModelProviderError::Cancelled);
}

#[test]
fn opaque_ids_and_credential_refs_validate_and_redact() {
    assert!(ProviderId::parse("provider.example").is_ok());
    assert!(ModelId::parse("model.example").is_ok());
    assert!(ProviderId::parse("https://provider.invalid").is_err());
    assert!(ModelId::parse("model://concrete-sdk").is_err());
    assert!(CredentialRef::parse("cred_project_1").is_ok());
    assert!(CredentialRef::parse("sk-live-secret-token").is_err());

    let error = CredentialRef::parse("api_key=secret").unwrap_err();
    let rendered = error.to_string();
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("api_key"));
}

#[test]
fn request_and_stream_config_are_bounded_and_roundtrip() {
    let request = request();
    let encoded = serde_json::to_value(&request).unwrap();
    let decoded: ProviderRequest = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);

    assert!(StreamConfig::new(1).is_ok());
    assert!(StreamConfig::new(1024).is_ok());
    assert!(StreamConfig::new(0).is_err());
    assert!(StreamConfig::new(1025).is_err());

    assert!(ProviderRequest::new(
        "req",
        ModelId::parse("model").unwrap(),
        CredentialRef::parse("cred_ref").unwrap(),
        "x".repeat(1_048_577),
    )
    .is_err());
}

#[test]
fn unsupported_operation_is_a_typed_non_secret_error() {
    let error = ModelProviderError::UnsupportedOperation("stream".into());
    assert!(error.to_string().contains("unsupported provider operation"));
    assert!(!error.to_string().contains("token"));
}
