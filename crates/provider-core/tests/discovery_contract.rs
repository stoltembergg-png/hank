use provider_core::capabilities::{
    CapabilityFeature, CapabilityRequirement, CapabilitySource, ModelModality,
};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService, ProjectScopeId,
};
use provider_core::discovery::{
    DiscoveryCachePolicy, DiscoveryError, DiscoveryRequest, ModelDiscoveryService,
};
use provider_core::registry::ProviderRegistry;
use provider_core::{CancellationToken, CredentialRef, MockProvider, ProviderId};
use std::collections::BTreeSet;
use std::sync::Arc;

fn account() -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse("project_1").unwrap(),
        ProviderId::parse("mock-provider").unwrap(),
        AccountId::parse("account_1").unwrap(),
    )
    .unwrap()
}

fn access() -> CredentialAccessContext {
    CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        CancellationToken::new(),
    )
    .unwrap()
}

fn service() -> ModelDiscoveryService {
    let registry = Arc::new(ProviderRegistry::new());
    registry
        .register(Arc::new(MockProvider::new(
            ProviderId::parse("mock-provider").unwrap(),
            "0.1",
        )))
        .unwrap();
    let credentials = Arc::new(provider_core::credentials::InMemoryCredentialService::new());
    credentials
        .connect(
            access(),
            account(),
            CredentialRef::parse("cred_mock_1").unwrap(),
        )
        .unwrap();
    ModelDiscoveryService::new(
        registry,
        credentials,
        DiscoveryCachePolicy::new(60_000).unwrap(),
    )
}

fn request() -> DiscoveryRequest {
    DiscoveryRequest::new(
        access(),
        account(),
        0,
        10,
        1_000,
        CapabilityRequirement::default(),
    )
    .unwrap()
}

#[test]
fn discovery_returns_normalized_model_and_capabilities() {
    let result = service().discover(request()).unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.models[0].provider_id.as_str(), "mock-provider");
    assert_eq!(result.models[0].model_id.as_str(), "mock-model");
    assert_eq!(result.models[0].source, CapabilitySource::Provider);
    assert!(result.models[0].credential_ref_available);
    assert!(result.models[0]
        .capabilities
        .feature_state(CapabilityFeature::Streaming)
        .is_supported());
}

#[test]
fn discovery_rejects_missing_or_revoked_credentials() {
    let registry = Arc::new(ProviderRegistry::new());
    registry
        .register(Arc::new(MockProvider::new(
            ProviderId::parse("mock-provider").unwrap(),
            "0.1",
        )))
        .unwrap();
    let credentials = Arc::new(provider_core::credentials::InMemoryCredentialService::new());
    let service = ModelDiscoveryService::new(
        registry,
        credentials,
        DiscoveryCachePolicy::new(60_000).unwrap(),
    );
    assert!(matches!(
        service.discover(request()),
        Err(DiscoveryError::CredentialMissing)
    ));
}

#[test]
fn discovery_enforces_capability_requirements_before_returning_models() {
    let mut req = request();
    req.requirements.modalities = BTreeSet::from([ModelModality::Image]);
    assert!(matches!(
        service().discover(req),
        Err(DiscoveryError::CapabilityMismatch(_))
    ));
}

#[test]
fn discovery_paginates_bounded_results_and_rejects_invalid_page_size() {
    let mut req = request();
    req.page_size = 1;
    let first = service().discover(req.clone()).unwrap();
    assert_eq!(first.models.len(), 1);
    req.page = 1;
    assert!(service().discover(req).unwrap().models.is_empty());
    assert!(DiscoveryRequest::new(
        access(),
        account(),
        0,
        0,
        1_000,
        CapabilityRequirement::default()
    )
    .is_err());
    assert!(DiscoveryRequest::new(
        access(),
        account(),
        0,
        65,
        1_000,
        CapabilityRequirement::default()
    )
    .is_err());
}

#[test]
fn discovery_cache_is_explicit_and_invalidatable() {
    let service = service();
    let first = service.discover(request()).unwrap();
    assert_eq!(first.models[0].source, CapabilitySource::Provider);
    let cached = service.discover(request()).unwrap();
    assert_eq!(cached.models[0].source, CapabilitySource::Cache);
    service.clear_cache().unwrap();
    let refreshed = service.discover(request()).unwrap();
    assert_eq!(refreshed.models[0].source, CapabilitySource::Provider);
}

#[test]
fn discovery_rejects_disabled_provider_and_cancelled_request() {
    let registry = Arc::new(ProviderRegistry::new());
    let provider_id = ProviderId::parse("mock-provider").unwrap();
    registry
        .register(Arc::new(MockProvider::new(provider_id.clone(), "0.1")))
        .unwrap();
    registry.set_enabled(&provider_id, false).unwrap();
    let credentials = Arc::new(provider_core::credentials::InMemoryCredentialService::new());
    credentials
        .connect(
            access(),
            account(),
            CredentialRef::parse("cred_mock_1").unwrap(),
        )
        .unwrap();
    let service = ModelDiscoveryService::new(
        registry,
        credentials,
        DiscoveryCachePolicy::new(60_000).unwrap(),
    );
    assert!(matches!(
        service.discover(request()),
        Err(DiscoveryError::ProviderDisabled)
    ));

    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let context = CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        cancelled,
    )
    .unwrap();
    let mut req = request();
    req.access = context;
    assert!(matches!(
        service.discover(req),
        Err(DiscoveryError::Cancelled)
    ));
}

#[test]
fn discovery_does_not_expose_credential_material_or_provider_payload() {
    let result = service().discover(request()).unwrap();
    let debug = format!("{result:?}");
    assert!(!debug.contains("cred_mock_1"));
    assert!(!debug.contains("api_key"));
    assert!(!debug.contains("secret"));
}
