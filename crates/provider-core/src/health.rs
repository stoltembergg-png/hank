//! Provider health checks with bounded, project-scoped evidence.

use crate::credentials::{
    CredentialAccessContext, CredentialAccount, CredentialService, CredentialServiceError,
};
use crate::registry::{ProviderRegistry, RegistryError};
use crate::{
    CancellationToken, CredentialRef, HealthStatus as ProviderHealthStatus, ModelProvider,
    ModelProviderError, ProviderFuture, ProviderId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;

const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_INTERVAL_MS: u64 = 3_600_000;
const MAX_CACHE_AGE_MS: u64 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthCheckPolicy {
    pub timeout: Duration,
    pub min_interval_ms: u64,
    pub max_cache_age_ms: u64,
}

impl HealthCheckPolicy {
    pub fn new(
        timeout_ms: u64,
        min_interval_ms: u64,
        max_cache_age_ms: u64,
    ) -> Result<Self, HealthError> {
        if !(1..=MAX_TIMEOUT_MS).contains(&timeout_ms)
            || !(1..=MAX_INTERVAL_MS).contains(&min_interval_ms)
            || !(1..=MAX_CACHE_AGE_MS).contains(&max_cache_age_ms)
        {
            return Err(HealthError::InvalidRequest);
        }
        Ok(Self {
            timeout: Duration::from_millis(timeout_ms),
            min_interval_ms,
            max_cache_age_ms,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HealthRequest {
    pub access: CredentialAccessContext,
    pub account: CredentialAccount,
    pub now_ms: u64,
}

impl HealthRequest {
    pub fn new(
        access: CredentialAccessContext,
        account: CredentialAccount,
        now_ms: u64,
    ) -> Result<Self, HealthError> {
        if access.project_id != account.project_id {
            return Err(HealthError::Unauthorized);
        }
        Ok(Self {
            access,
            account,
            now_ms,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unconfigured,
    Disabled,
    RateLimited,
    QuotaExceeded,
    Timeout,
    Outage,
    InvalidCredential,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthReason {
    Healthy,
    ProviderDegraded,
    ProviderUnavailable,
    CredentialMissing,
    CredentialRevoked,
    CredentialServiceUnavailable,
    ProviderDisabled,
    ProviderNotFound,
    RateLimited,
    QuotaExceeded,
    Timeout,
    Outage,
    InvalidCredential,
    Unsupported,
    Internal,
}

impl HealthReason {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::ProviderDegraded => "provider_degraded",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::CredentialMissing => "credential_missing",
            Self::CredentialRevoked => "credential_revoked",
            Self::CredentialServiceUnavailable => "credential_service_unavailable",
            Self::ProviderDisabled => "provider_disabled",
            Self::ProviderNotFound => "provider_not_found",
            Self::RateLimited => "rate_limited",
            Self::QuotaExceeded => "quota_exceeded",
            Self::Timeout => "timeout",
            Self::Outage => "outage",
            Self::InvalidCredential => "invalid_credential",
            Self::Unsupported => "unsupported",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResult {
    pub check_id: String,
    pub provider_id: ProviderId,
    pub account_id: String,
    pub status: HealthStatus,
    pub reason: HealthReason,
    pub cache_hit: bool,
    pub latency_ms: u64,
    pub evidence_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HealthProbeError {
    #[error("provider rate limited health probe")]
    RateLimited,
    #[error("provider quota exceeded")]
    QuotaExceeded,
    #[error("provider health probe timed out")]
    Timeout,
    #[error("provider health probe outage")]
    Outage,
    #[error("provider credential rejected")]
    InvalidCredential,
    #[error("provider health operation unsupported")]
    Unsupported,
    #[error("provider health operation cancelled")]
    Cancelled,
    #[error("provider health probe failed internally")]
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HealthError {
    #[error("health request is invalid")]
    InvalidRequest,
    #[error("health access is unauthorized")]
    Unauthorized,
    #[error("health operation was cancelled")]
    Cancelled,
    #[error("health service state is unavailable")]
    Internal,
}

pub trait HealthProbe: Send + Sync {
    fn check(
        &self,
        provider: Arc<dyn ModelProvider>,
        credential_ref: CredentialRef,
        timeout: Duration,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProbeOutcome, HealthProbeError>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultHealthProbe;

impl HealthProbe for DefaultHealthProbe {
    fn check(
        &self,
        provider: Arc<dyn ModelProvider>,
        _credential_ref: CredentialRef,
        _timeout: Duration,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProbeOutcome, HealthProbeError>> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(HealthProbeError::Cancelled);
            }
            let future = provider.health();
            match future.await {
                Ok(ProviderHealthStatus::Healthy) => Ok(ProbeOutcome::Healthy),
                Ok(ProviderHealthStatus::Degraded) => Ok(ProbeOutcome::Degraded),
                Ok(ProviderHealthStatus::Unavailable) => Ok(ProbeOutcome::Unavailable),
                Err(ModelProviderError::Cancelled) => Err(HealthProbeError::Cancelled),
                Err(ModelProviderError::Unavailable) => Err(HealthProbeError::Outage),
                Err(ModelProviderError::UnsupportedOperation(_)) => {
                    Err(HealthProbeError::Unsupported)
                }
                Err(_) => Err(HealthProbeError::Internal),
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct CacheKey {
    project_id: String,
    provider_id: ProviderId,
    account_id: String,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    checked_at_ms: u64,
    result: HealthResult,
}

pub struct HealthService {
    registry: Arc<ProviderRegistry>,
    credentials: Arc<dyn CredentialService>,
    probe: Arc<dyn HealthProbe>,
    policy: HealthCheckPolicy,
    cache: Mutex<BTreeMap<CacheKey, CacheEntry>>,
}

impl HealthService {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        credentials: Arc<dyn CredentialService>,
        probe: Arc<dyn HealthProbe>,
        policy: HealthCheckPolicy,
    ) -> Result<Self, HealthError> {
        Ok(Self {
            registry,
            credentials,
            probe,
            policy,
            cache: Mutex::new(BTreeMap::new()),
        })
    }

    pub async fn check(&self, request: HealthRequest) -> Result<HealthResult, HealthError> {
        if request.access.cancellation.is_cancelled() {
            return Err(HealthError::Cancelled);
        }

        let cache_key = CacheKey {
            project_id: request.account.project_id.as_str().to_string(),
            provider_id: request.account.provider_id.clone(),
            account_id: request.account.account_id.as_str().to_string(),
        };

        let credential_ref = match self
            .credentials
            .resolve_ref(request.access.clone(), request.account.clone())
        {
            Ok(reference) => reference,
            Err(CredentialServiceError::Missing) => {
                return Ok(self.result(
                    &request,
                    HealthStatus::Unconfigured,
                    HealthReason::CredentialMissing,
                    false,
                ))
            }
            Err(CredentialServiceError::Revoked) => {
                return Ok(self.result(
                    &request,
                    HealthStatus::Unconfigured,
                    HealthReason::CredentialRevoked,
                    false,
                ))
            }
            Err(CredentialServiceError::Unavailable) => {
                return Ok(self.result(
                    &request,
                    HealthStatus::Unconfigured,
                    HealthReason::CredentialServiceUnavailable,
                    false,
                ))
            }
            Err(CredentialServiceError::Cancelled) => return Err(HealthError::Cancelled),
            Err(CredentialServiceError::Unauthorized) => return Err(HealthError::Unauthorized),
            Err(_) => {
                return Ok(self.result(
                    &request,
                    HealthStatus::InvalidCredential,
                    HealthReason::InvalidCredential,
                    false,
                ))
            }
        };

        let provider = match self.registry.get(&request.account.provider_id) {
            Ok(provider) => provider,
            Err(RegistryError::Disabled(_)) => {
                return Ok(self.result(
                    &request,
                    HealthStatus::Disabled,
                    HealthReason::ProviderDisabled,
                    false,
                ))
            }
            Err(RegistryError::NotFound(_)) => {
                return Ok(self.result(
                    &request,
                    HealthStatus::Unsupported,
                    HealthReason::ProviderNotFound,
                    false,
                ))
            }
            Err(_) => return Err(HealthError::Internal),
        };

        if let Some(cached) = self.cached(&cache_key, request.now_ms)? {
            return Ok(cached);
        }

        if request.access.cancellation.is_cancelled() {
            return Err(HealthError::Cancelled);
        }
        let probe_result = self.probe.check(
            provider,
            credential_ref,
            self.policy.timeout,
            request.access.cancellation.clone(),
        );
        let started = Instant::now();
        let (status, reason) = match probe_result.await {
            Ok(outcome) => map_outcome(outcome),
            Err(HealthProbeError::Cancelled) => return Err(HealthError::Cancelled),
            Err(error) => map_probe_error(error),
        };
        let mut result = self.result(&request, status, reason, false);
        result.latency_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        self.store(cache_key, request.now_ms, result.clone())?;
        Ok(result)
    }

    pub fn clear_cache(&self) -> Result<(), HealthError> {
        self.cache
            .lock()
            .map_err(|_| HealthError::Internal)?
            .clear();
        Ok(())
    }

    fn cached(&self, key: &CacheKey, now_ms: u64) -> Result<Option<HealthResult>, HealthError> {
        let mut cache = self.cache.lock().map_err(|_| HealthError::Internal)?;
        cache.retain(|_, entry| {
            now_ms.saturating_sub(entry.checked_at_ms) <= self.policy.max_cache_age_ms
        });
        let Some(entry) = cache.get(key) else {
            return Ok(None);
        };
        if now_ms.saturating_sub(entry.checked_at_ms) < self.policy.min_interval_ms {
            let mut result = entry.result.clone();
            result.cache_hit = true;
            return Ok(Some(result));
        }
        Ok(None)
    }

    fn store(
        &self,
        key: CacheKey,
        checked_at_ms: u64,
        result: HealthResult,
    ) -> Result<(), HealthError> {
        self.cache
            .lock()
            .map_err(|_| HealthError::Internal)?
            .insert(
                key,
                CacheEntry {
                    checked_at_ms,
                    result,
                },
            );
        Ok(())
    }

    fn result(
        &self,
        request: &HealthRequest,
        status: HealthStatus,
        reason: HealthReason,
        cache_hit: bool,
    ) -> HealthResult {
        HealthResult {
            check_id: format!(
                "health_{}_{}_{}",
                request.account.provider_id.as_str(),
                request.account.account_id.as_str(),
                request.now_ms
            ),
            provider_id: request.account.provider_id.clone(),
            account_id: request.account.account_id.as_str().to_string(),
            status,
            reason,
            cache_hit,
            latency_ms: 0,
            evidence_at_ms: request.now_ms,
        }
    }
}

fn map_outcome(outcome: ProbeOutcome) -> (HealthStatus, HealthReason) {
    match outcome {
        ProbeOutcome::Healthy => (HealthStatus::Healthy, HealthReason::Healthy),
        ProbeOutcome::Degraded => (HealthStatus::Unhealthy, HealthReason::ProviderDegraded),
        ProbeOutcome::Unavailable => (HealthStatus::Outage, HealthReason::ProviderUnavailable),
    }
}

fn map_probe_error(error: HealthProbeError) -> (HealthStatus, HealthReason) {
    match error {
        HealthProbeError::RateLimited => (HealthStatus::RateLimited, HealthReason::RateLimited),
        HealthProbeError::QuotaExceeded => {
            (HealthStatus::QuotaExceeded, HealthReason::QuotaExceeded)
        }
        HealthProbeError::Timeout => (HealthStatus::Timeout, HealthReason::Timeout),
        HealthProbeError::Outage => (HealthStatus::Outage, HealthReason::Outage),
        HealthProbeError::InvalidCredential => (
            HealthStatus::InvalidCredential,
            HealthReason::InvalidCredential,
        ),
        HealthProbeError::Unsupported => (HealthStatus::Unsupported, HealthReason::Unsupported),
        HealthProbeError::Cancelled => (HealthStatus::Outage, HealthReason::Internal),
        HealthProbeError::Internal => (HealthStatus::Outage, HealthReason::Internal),
    }
}
