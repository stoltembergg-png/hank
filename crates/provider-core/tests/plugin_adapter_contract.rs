use futures_util::StreamExt;
use provider_core::{
    CancellationToken, CredentialRef, MockProvider, ModelId, ModelProvider, ProviderId,
    ProviderPluginAdapter, ProviderRequest, StreamConfig,
};
use std::sync::Arc;

fn request() -> ProviderRequest {
    ProviderRequest::new(
        "request-1",
        ModelId::parse("mock-model").unwrap(),
        CredentialRef::parse("cred_provider_1").unwrap(),
        "hello",
    )
    .unwrap()
}

fn adapter(approved: bool) -> ProviderPluginAdapter {
    let provider = Arc::new(MockProvider::new(
        ProviderId::parse("plugin-provider").unwrap(),
        "1.0.0",
    ));
    ProviderPluginAdapter::new(provider, "plugin-a", "digest-1", approved).unwrap()
}

#[tokio::test]
// @spec:AC-1397
async fn approved_plugin_preserves_normalized_provider_contract() {
    let adapter = adapter(true);
    assert_eq!(adapter.plugin_id(), "plugin-a");
    assert_eq!(adapter.plugin_digest(), "digest-1");
    assert_eq!(adapter.version(), "1.0.0");
    let response = adapter
        .complete(request(), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(response.text, "mock response: hello");
    let mut stream = adapter
        .stream(
            request(),
            CancellationToken::new(),
            StreamConfig::new(2).unwrap(),
        )
        .unwrap();
    assert!(stream.next().await.is_some());
}

#[tokio::test]
// @spec:AC-1398
async fn unapproved_plugin_denies_calls_without_external_effects() {
    let adapter = adapter(false);
    let error = adapter
        .complete(request(), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error, provider_core::ModelProviderError::Unavailable);
    assert!(matches!(
        adapter.stream(
            request(),
            CancellationToken::new(),
            StreamConfig::new(2).unwrap()
        ),
        Err(provider_core::ModelProviderError::Unavailable)
    ));
}
