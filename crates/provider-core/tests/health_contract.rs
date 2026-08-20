use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService, ProjectScopeId,
};
use provider_core::health::{
    DefaultHealthProbe, HealthCheckPolicy, HealthError, HealthProbe, HealthProbeError,
    HealthRequest, HealthService, HealthStatus,
};
use provider_core::registry::ProviderRegistry;
use provider_core::{CancellationToken, CredentialRef, MockProvider, ProviderFuture, ProviderId};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
struct StaticProbe {
    outcome: Arc<Mutex<Result<provider_core::health::ProbeOutcome, HealthProbeError>>>,
    calls: Arc<Mutex<usize>>,
}

impl StaticProbe {
    fn new(outcome: Result<provider_core::health::ProbeOutcome, HealthProbeError>) -> Self {
        Self {
            outcome: Arc::new(Mutex::new(outcome)),
            calls: Arc::new(Mutex::new(0)),
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }
}

impl HealthProbe for StaticProbe {
    fn check(
        &self,
        _provider: Arc<dyn provider_core::ModelProvider>,
        _credential_ref: CredentialRef,
        _timeout: Duration,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<provider_core::health::ProbeOutcome, HealthProbeError>> {
        let outcome = *self.outcome.lock().unwrap();
        *self.calls.lock().unwrap() += 1;
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(HealthProbeError::Cancelled);
            }
            outcome
        })
    }
}

fn account() -> CredentialAccount {
    CredentialAccount::new(
        ProjectScopeId::parse("project_1").unwrap(),
        ProviderId::parse("mock-provider").unwrap(),
        AccountId::parse("account_1").unwrap(),
    )
    .unwrap()
}

fn context() -> CredentialAccessContext {
    CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        CancellationToken::new(),
    )
    .unwrap()
}

fn registry() -> Arc<ProviderRegistry> {
    let registry = Arc::new(ProviderRegistry::new());
    registry
        .register(Arc::new(MockProvider::new(
            ProviderId::parse("mock-provider").unwrap(),
            "0.1",
        )))
        .unwrap();
    registry
}

fn credentials() -> Arc<provider_core::credentials::InMemoryCredentialService> {
    let credentials = Arc::new(provider_core::credentials::InMemoryCredentialService::new());
    credentials
        .connect(
            context(),
            account(),
            CredentialRef::parse("cred_mock_1").unwrap(),
        )
        .unwrap();
    credentials
}

fn policy() -> HealthCheckPolicy {
    HealthCheckPolicy::new(5_000, 100, 10_000).unwrap()
}

fn request(now_ms: u64) -> HealthRequest {
    HealthRequest::new(context(), account(), now_ms).unwrap()
}

fn service(probe: Arc<dyn HealthProbe>) -> HealthService {
    HealthService::new(registry(), credentials(), probe, policy()).unwrap()
}

#[tokio::test]
async fn healthy_status_has_evidence_without_secret_material() {
    let probe = Arc::new(StaticProbe::new(Ok(
        provider_core::health::ProbeOutcome::Healthy,
    )));
    let result = service(probe).check(request(1_000)).await.unwrap();
    assert_eq!(result.status, HealthStatus::Healthy);
    assert_eq!(result.reason.code(), "healthy");
    assert!(!result.cache_hit);
    assert!(format!("{result:?}").contains("mock-provider"));
    assert!(!format!("{result:?}").contains("cred_mock_1"));
    assert!(result.evidence_at_ms >= 1_000);
}

#[tokio::test]
async fn missing_and_revoked_credentials_never_claim_healthy() {
    let probe = Arc::new(StaticProbe::new(Ok(
        provider_core::health::ProbeOutcome::Healthy,
    )));
    let service_without_credential = HealthService::new(
        registry(),
        Arc::new(provider_core::credentials::InMemoryCredentialService::new()),
        probe.clone(),
        policy(),
    )
    .unwrap();
    let missing = service_without_credential
        .check(request(1_000))
        .await
        .unwrap();
    assert_eq!(missing.status, HealthStatus::Unconfigured);
    assert_eq!(missing.reason.code(), "credential_missing");
    assert_eq!(probe.calls(), 0);

    let revoked_credentials = credentials();
    revoked_credentials
        .disconnect(context(), account())
        .unwrap();
    let revoked_service = HealthService::new(
        registry(),
        revoked_credentials,
        Arc::new(StaticProbe::new(Ok(
            provider_core::health::ProbeOutcome::Healthy,
        ))),
        policy(),
    )
    .unwrap();
    let revoked = revoked_service.check(request(1_001)).await.unwrap();
    assert_eq!(revoked.status, HealthStatus::Unconfigured);
    assert_eq!(revoked.reason.code(), "credential_revoked");
}

#[tokio::test]
async fn probe_categories_map_to_stable_non_healthy_statuses() {
    let cases = [
        (
            HealthProbeError::RateLimited,
            HealthStatus::RateLimited,
            "rate_limited",
        ),
        (
            HealthProbeError::QuotaExceeded,
            HealthStatus::QuotaExceeded,
            "quota_exceeded",
        ),
        (HealthProbeError::Timeout, HealthStatus::Timeout, "timeout"),
        (HealthProbeError::Outage, HealthStatus::Outage, "outage"),
        (
            HealthProbeError::InvalidCredential,
            HealthStatus::InvalidCredential,
            "invalid_credential",
        ),
        (
            HealthProbeError::Unsupported,
            HealthStatus::Unsupported,
            "unsupported",
        ),
    ];
    for (error, status, code) in cases {
        let result = service(Arc::new(StaticProbe::new(Err(error))))
            .check(request(2_000))
            .await
            .unwrap();
        assert_eq!(result.status, status);
        assert_eq!(result.reason.code(), code);
        assert_ne!(result.status, HealthStatus::Healthy);
    }
}

#[tokio::test]
async fn disabled_provider_cannot_be_reported_healthy() {
    let registry = registry();
    registry
        .set_enabled(&ProviderId::parse("mock-provider").unwrap(), false)
        .unwrap();
    let service = HealthService::new(
        registry,
        credentials(),
        Arc::new(StaticProbe::new(Ok(
            provider_core::health::ProbeOutcome::Healthy,
        ))),
        policy(),
    )
    .unwrap();
    let result = service.check(request(3_000)).await.unwrap();
    assert_eq!(result.status, HealthStatus::Disabled);
    assert_eq!(result.reason.code(), "provider_disabled");
}

#[tokio::test]
async fn debounce_returns_cached_evidence_without_probe_storm() {
    let probe = Arc::new(StaticProbe::new(Ok(
        provider_core::health::ProbeOutcome::Healthy,
    )));
    let service = service(probe.clone());
    let first = service.check(request(4_000)).await.unwrap();
    let second = service.check(request(4_050)).await.unwrap();
    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert_eq!(probe.calls(), 1);

    let refreshed = service.check(request(4_101)).await.unwrap();
    assert!(!refreshed.cache_hit);
    assert_eq!(probe.calls(), 2);
}

#[tokio::test]
async fn cancellation_is_explicit_and_does_not_call_probe() {
    let probe = Arc::new(StaticProbe::new(Ok(
        provider_core::health::ProbeOutcome::Healthy,
    )));
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let access = CredentialAccessContext::new(
        ProjectScopeId::parse("project_1").unwrap(),
        "agent_1".into(),
        cancelled,
    )
    .unwrap();
    let request = HealthRequest::new(access, account(), 5_000).unwrap();
    let result = service(probe.clone()).check(request).await;
    assert!(matches!(result, Err(HealthError::Cancelled)));
    assert_eq!(probe.calls(), 0);
}

#[tokio::test]
async fn policy_and_request_bounds_are_fail_closed() {
    assert!(HealthCheckPolicy::new(0, 100, 10_000).is_err());
    assert!(HealthCheckPolicy::new(5_000, 0, 10_000).is_err());
    assert!(HealthCheckPolicy::new(5_000, 100, 0).is_err());
    assert!(HealthRequest::new(context(), account(), u64::MAX).is_ok());
}

#[tokio::test]
async fn default_probe_uses_provider_health_contract() {
    let service = HealthService::new(
        registry(),
        credentials(),
        Arc::new(DefaultHealthProbe),
        policy(),
    )
    .unwrap();
    let result = service.check(request(6_000)).await.unwrap();
    assert_eq!(result.status, HealthStatus::Healthy);
}
