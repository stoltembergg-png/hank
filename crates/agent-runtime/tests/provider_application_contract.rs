use agent_runtime::provider_service::{
    InvocationError, InvocationRequest, ProviderApplicationService,
};
use provider_core::capabilities::{
    CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilityRequirement, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService, ProjectScopeId,
};
use provider_core::fallback::{FallbackCandidate, FallbackPolicy};
use provider_core::health::HealthStatus;
use provider_core::request::{
    CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
};
use provider_core::{
    CancellationToken, CredentialRef, FinishReason, MockProvider, ModelId, ModelProvider,
    ModelProviderError, ProviderFuture, ProviderId, ProviderRequest, ProviderResponse,
    ProviderStream, StreamConfig,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

struct FailingProvider {
    inner: MockProvider,
}

impl FailingProvider {
    fn new() -> Self {
        Self {
            inner: MockProvider::new(ProviderId::parse("failing-provider").unwrap(), "0.1"),
        }
    }
}

impl ModelProvider for FailingProvider {
    fn provider_id(&self) -> &ProviderId {
        self.inner.provider_id()
    }

    fn version(&self) -> &str {
        self.inner.version()
    }

    fn capabilities(&self) -> CapabilityReport {
        self.inner.capabilities()
    }

    fn complete(
        &self,
        _request: ProviderRequest,
        _cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProviderResponse, ModelProviderError>> {
        Box::pin(async { Err(ModelProviderError::Unavailable) })
    }

    fn stream(
        &self,
        _request: ProviderRequest,
        _cancellation: CancellationToken,
        _config: StreamConfig,
    ) -> Result<ProviderStream<'_>, ModelProviderError> {
        Err(ModelProviderError::Unavailable)
    }

    fn list_models(
        &self,
    ) -> ProviderFuture<'_, Result<Vec<provider_core::ModelDescriptor>, ModelProviderError>> {
        self.inner.list_models()
    }

    fn health(
        &self,
    ) -> ProviderFuture<'_, Result<provider_core::HealthStatus, ModelProviderError>> {
        self.inner.health()
    }
}

fn account(provider: &str, account: &str) -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse("project_1").unwrap(),
        ProviderId::parse(provider).unwrap(),
        AccountId::parse(account).unwrap(),
    )
    .unwrap()
}

fn access(token: CancellationToken) -> CredentialAccessContext {
    CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        token,
    )
    .unwrap()
}

fn report(provider: &str, model: &str) -> CapabilityReport {
    CapabilityReport {
        schema_version: 1,
        provider_id: ProviderId::parse(provider).unwrap(),
        model_id: ModelId::parse(model).unwrap(),
        version: "1".into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([(ModelModality::Text, CapabilityState::Supported)]),
        features: BTreeMap::from([(CapabilityFeature::Streaming, CapabilityState::Supported)]),
        limits: CapabilityLimits {
            max_context_tokens: Some(32_768),
            max_output_tokens: Some(8_192),
        },
    }
}

fn request(provider: &str, stream: bool, _cancellation: CancellationToken) -> NormalizedRequest {
    NormalizedRequest {
        schema_version: 1,
        request_id: "request_1".into(),
        correlation_id: "correlation_1".into(),
        project_id: "project_1".into(),
        agent_id: "agent_1".into(),
        session_id: Some("session_1".into()),
        provider_id: ProviderId::parse(provider).unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        messages: vec![NormalizedMessage {
            role: MessageRole::User,
            content: "hello".into(),
        }],
        modalities: BTreeSet::from([ModelModality::Text]),
        capabilities: CapabilityRequirement {
            modalities: BTreeSet::from([ModelModality::Text]),
            features: if stream {
                BTreeSet::from([CapabilityFeature::Streaming])
            } else {
                BTreeSet::new()
            },
            min_context_tokens: Some(4_096),
            min_output_tokens: Some(256),
        },
        tools: vec![],
        budget: RequestBudget {
            max_tokens: Some(512),
            max_cost_micros: Some(1_000),
        },
        cancellation: CancellationMetadata {
            cancellation_id: "cancel_1".into(),
            deadline_unix_ms: Some(2_000_000_000_000),
        },
        temperature: Some(0.2),
    }
}

fn service_with_mock() -> (
    ProviderApplicationService,
    Arc<provider_core::credentials::InMemoryCredentialService>,
) {
    let registry = Arc::new(provider_core::registry::ProviderRegistry::new());
    registry
        .register(Arc::new(MockProvider::new(
            ProviderId::parse("mock-provider").unwrap(),
            "0.1",
        )))
        .unwrap();
    let credentials = Arc::new(provider_core::credentials::InMemoryCredentialService::new());
    credentials
        .connect(
            access(CancellationToken::new()),
            account("mock-provider", "account_mock"),
            CredentialRef::parse("cred_mock_1").unwrap(),
        )
        .unwrap();
    let service = ProviderApplicationService::new(
        registry,
        credentials.clone(),
        FallbackPolicy::new(2, 1_000, 1_000).unwrap(),
    );
    (service, credentials)
}

#[tokio::test]
async fn complete_uses_only_application_service_and_returns_neutral_result() {
    let (service, _) = service_with_mock();
    let request = InvocationRequest::new(
        request("mock-provider", false, CancellationToken::new()),
        account("mock-provider", "account_mock"),
        access(CancellationToken::new()),
        vec![],
    )
    .unwrap();
    let result = service.complete(request).await.unwrap();
    assert_eq!(result.provider_id.as_str(), "mock-provider");
    assert_eq!(result.model_id.as_str(), "mock-model");
    assert_eq!(result.text, "mock response: hello");
    assert_eq!(result.finish_reason, FinishReason::Stop);
    assert_eq!(result.attempt_number, 1);
    assert!(!format!("{result:?}").contains("cred_mock_1"));
}

#[tokio::test]
async fn missing_credential_and_capability_mismatch_fail_before_provider_call() {
    let (service, credentials) = service_with_mock();
    credentials
        .disconnect(
            access(CancellationToken::new()),
            account("mock-provider", "account_mock"),
        )
        .unwrap();
    let request = InvocationRequest::new(
        request("mock-provider", false, CancellationToken::new()),
        account("mock-provider", "account_mock"),
        access(CancellationToken::new()),
        vec![],
    )
    .unwrap();
    assert!(matches!(
        service.complete(request).await,
        Err(InvocationError::Credential(_))
    ));
}

#[tokio::test]
async fn cancellation_is_explicit_and_does_not_invoke_provider() {
    let (service, _) = service_with_mock();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let request = InvocationRequest::new(
        request("mock-provider", false, cancellation.clone()),
        account("mock-provider", "account_mock"),
        access(cancellation),
        vec![],
    )
    .unwrap();
    assert!(matches!(
        service.complete(request).await,
        Err(InvocationError::Cancelled)
    ));
}

#[tokio::test]
async fn stream_returns_attempt_identity_and_terminal_event() {
    let (service, _) = service_with_mock();
    let request = InvocationRequest::new(
        request("mock-provider", true, CancellationToken::new()),
        account("mock-provider", "account_mock"),
        access(CancellationToken::new()),
        vec![],
    )
    .unwrap();
    let events = service.stream(request).await.unwrap();
    assert_eq!(events.len(), 2);
    assert!(events
        .iter()
        .all(|event| event.attempt_id == "request_1:attempt_1"));
    assert!(events.last().unwrap().terminal);
}

#[tokio::test]
async fn unavailable_provider_uses_one_eligible_fallback_attempt() {
    let registry = Arc::new(provider_core::registry::ProviderRegistry::new());
    registry.register(Arc::new(FailingProvider::new())).unwrap();
    registry
        .register(Arc::new(MockProvider::new(
            ProviderId::parse("mock-provider").unwrap(),
            "0.1",
        )))
        .unwrap();
    let credentials = Arc::new(provider_core::credentials::InMemoryCredentialService::new());
    credentials
        .connect(
            access(CancellationToken::new()),
            account("failing-provider", "account_failing"),
            CredentialRef::parse("cred_fail_1").unwrap(),
        )
        .unwrap();
    credentials
        .connect(
            access(CancellationToken::new()),
            account("mock-provider", "account_mock"),
            CredentialRef::parse("cred_mock_1").unwrap(),
        )
        .unwrap();
    let fallback = FallbackCandidate::new(
        account("mock-provider", "account_mock"),
        ModelId::parse("mock-model").unwrap(),
        report("mock-provider", "mock-model"),
        HealthStatus::Healthy,
        512,
        100,
    )
    .unwrap();
    let service = ProviderApplicationService::new(
        registry,
        credentials,
        FallbackPolicy::new(2, 1_000, 1_000).unwrap(),
    );
    let request = InvocationRequest::new(
        request("failing-provider", false, CancellationToken::new()),
        account("failing-provider", "account_failing"),
        access(CancellationToken::new()),
        vec![fallback],
    )
    .unwrap();
    let result = service.complete(request).await.unwrap();
    assert_eq!(result.provider_id.as_str(), "mock-provider");
    assert_eq!(result.attempt_number, 2);
    assert_eq!(result.attempt_id, "request_1:attempt_2");
}
