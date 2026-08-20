//! Bounded model discovery through registry and credential services.

use crate::capabilities::{
    CapabilityError, CapabilityReport, CapabilityRequirement, CapabilitySource,
};
use crate::credentials::{
    CredentialAccessContext, CredentialAccount, CredentialService, CredentialServiceError,
};
use crate::registry::{ProviderRegistry, RegistryError};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

const MAX_PAGE_SIZE: usize = 64;
const MAX_PAGE_INDEX: usize = 1_000_000;
const MAX_CACHE_TTL_MS: u64 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryCachePolicy {
    ttl_ms: u64,
}

impl DiscoveryCachePolicy {
    pub fn new(ttl_ms: u64) -> Result<Self, DiscoveryError> {
        if !(1..=MAX_CACHE_TTL_MS).contains(&ttl_ms) {
            return Err(DiscoveryError::InvalidRequest);
        }
        Ok(Self { ttl_ms })
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveryRequest {
    pub access: CredentialAccessContext,
    pub account: CredentialAccount,
    pub page: usize,
    pub page_size: usize,
    pub now_ms: u64,
    pub requirements: CapabilityRequirement,
}

impl DiscoveryRequest {
    pub fn new(
        access: CredentialAccessContext,
        account: CredentialAccount,
        page: usize,
        page_size: usize,
        now_ms: u64,
        requirements: CapabilityRequirement,
    ) -> Result<Self, DiscoveryError> {
        if page > MAX_PAGE_INDEX || !(1..=MAX_PAGE_SIZE).contains(&page_size) {
            return Err(DiscoveryError::InvalidRequest);
        }
        Ok(Self {
            access,
            account,
            page,
            page_size,
            now_ms,
            requirements,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedModelRecord {
    pub provider_id: crate::ProviderId,
    pub model_id: crate::ModelId,
    pub display_name: String,
    pub capabilities: CapabilityReport,
    pub source: CapabilitySource,
    pub credential_ref_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryResult {
    pub models: Vec<NormalizedModelRecord>,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiscoveryError {
    #[error("discovery request is invalid or exceeds bounds")]
    InvalidRequest,
    #[error("discovery operation was cancelled")]
    Cancelled,
    #[error("discovery project access is unauthorized")]
    Unauthorized,
    #[error("credential is missing for discovery")]
    CredentialMissing,
    #[error("credential is revoked for discovery")]
    CredentialRevoked,
    #[error("credential service is unavailable")]
    CredentialUnavailable,
    #[error("provider is not registered")]
    ProviderNotFound,
    #[error("provider is disabled")]
    ProviderDisabled,
    #[error("model capability requirement mismatch: {0}")]
    CapabilityMismatch(#[from] CapabilityError),
    #[error("discovery cache state is unavailable")]
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct CacheKey {
    project_id: String,
    provider_id: String,
    account_id: String,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    expires_at_ms: u64,
    models: Vec<NormalizedModelRecord>,
}

pub struct ModelDiscoveryService {
    registry: Arc<ProviderRegistry>,
    credentials: Arc<dyn CredentialService>,
    cache_policy: DiscoveryCachePolicy,
    cache: Mutex<BTreeMap<CacheKey, CacheEntry>>,
}

impl ModelDiscoveryService {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        credentials: Arc<dyn CredentialService>,
        cache_policy: DiscoveryCachePolicy,
    ) -> Self {
        Self {
            registry,
            credentials,
            cache_policy,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn discover(&self, request: DiscoveryRequest) -> Result<DiscoveryResult, DiscoveryError> {
        if request.access.cancellation.is_cancelled() {
            return Err(DiscoveryError::Cancelled);
        }
        if request.access.project_id != request.account.project_id {
            return Err(DiscoveryError::Unauthorized);
        }
        self.credentials
            .resolve_ref(request.access.clone(), request.account.clone())
            .map_err(map_credential_error)?;
        let provider_id = request.account.provider_id.clone();
        self.registry
            .get(&provider_id)
            .map_err(map_registry_error)?;
        let descriptor = self
            .registry
            .get_descriptor(&provider_id)
            .map_err(map_registry_error)?;
        descriptor
            .capabilities
            .check_compatibility(&request.requirements)?;

        let key = CacheKey {
            project_id: request.account.project_id.as_str().to_string(),
            provider_id: provider_id.as_str().to_string(),
            account_id: request.account.account_id.as_str().to_string(),
        };
        let use_cache = request.requirements == CapabilityRequirement::default();
        let models = if use_cache {
            self.cached_or_store(&key, request.now_ms, &descriptor.capabilities)?
        } else {
            vec![record_from_report(
                &descriptor.capabilities,
                CapabilitySource::Provider,
            )]
        };
        let total = models.len();
        let start = request
            .page
            .checked_mul(request.page_size)
            .ok_or(DiscoveryError::InvalidRequest)?;
        let page = models
            .into_iter()
            .skip(start)
            .take(request.page_size)
            .collect();
        Ok(DiscoveryResult {
            models: page,
            total,
        })
    }

    pub fn clear_cache(&self) -> Result<(), DiscoveryError> {
        self.cache
            .lock()
            .map_err(|_| DiscoveryError::Internal)?
            .clear();
        Ok(())
    }

    fn cached_or_store(
        &self,
        key: &CacheKey,
        now_ms: u64,
        report: &CapabilityReport,
    ) -> Result<Vec<NormalizedModelRecord>, DiscoveryError> {
        let mut cache = self.cache.lock().map_err(|_| DiscoveryError::Internal)?;
        cache.retain(|_, entry| entry.expires_at_ms > now_ms);
        if let Some(entry) = cache.get(key) {
            return Ok(entry
                .models
                .iter()
                .cloned()
                .map(|mut model| {
                    model.source = CapabilitySource::Cache;
                    model.capabilities.source = CapabilitySource::Cache;
                    model
                })
                .collect());
        }
        let models = vec![record_from_report(report, CapabilitySource::Provider)];
        cache.insert(
            key.clone(),
            CacheEntry {
                expires_at_ms: now_ms.saturating_add(self.cache_policy.ttl_ms),
                models: models.clone(),
            },
        );
        Ok(models)
    }
}

fn record_from_report(
    report: &CapabilityReport,
    source: CapabilitySource,
) -> NormalizedModelRecord {
    let mut capabilities = report.clone();
    capabilities.source = source;
    NormalizedModelRecord {
        provider_id: report.provider_id.clone(),
        model_id: report.model_id.clone(),
        display_name: format!(
            "{} / {}",
            report.provider_id.as_str(),
            report.model_id.as_str()
        ),
        capabilities,
        source,
        credential_ref_available: true,
    }
}

fn map_credential_error(error: CredentialServiceError) -> DiscoveryError {
    match error {
        CredentialServiceError::Missing => DiscoveryError::CredentialMissing,
        CredentialServiceError::Revoked => DiscoveryError::CredentialRevoked,
        CredentialServiceError::Unauthorized => DiscoveryError::Unauthorized,
        CredentialServiceError::Cancelled => DiscoveryError::Cancelled,
        CredentialServiceError::Unavailable => DiscoveryError::CredentialUnavailable,
        _ => DiscoveryError::CredentialUnavailable,
    }
}

fn map_registry_error(error: RegistryError) -> DiscoveryError {
    match error {
        RegistryError::NotFound(_) => DiscoveryError::ProviderNotFound,
        RegistryError::Disabled(_) => DiscoveryError::ProviderDisabled,
        _ => DiscoveryError::Internal,
    }
}
