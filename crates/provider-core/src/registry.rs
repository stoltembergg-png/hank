//! Provider registry for deterministic provider resolution.

use crate::{ModelProvider, ProviderId};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// Registry error types.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    #[error("provider not found: {0}")]
    NotFound(String),
    #[error("duplicate provider id: {0}")]
    DuplicateId(String),
    #[error("provider is disabled: {0}")]
    Disabled(String),
    #[error("no provider supports required capability: {0}")]
    CapabilityMismatch(String),
    #[error("invalid provider id: {0}")]
    InvalidId(String),
    #[error("registry is sealed")]
    Sealed,
}

/// Provider metadata stored in the registry.
#[derive(Clone)]
pub struct ProviderEntry {
    pub descriptor: ProviderDescriptor,
    pub adapter: Arc<dyn ModelProvider>,
    pub enabled: bool,
}

/// Provider descriptor with capabilities and version.
#[derive(Debug, Clone)]
pub struct ProviderDescriptor {
    pub provider_id: ProviderId,
    pub version: String,
    pub capabilities: crate::capabilities::CapabilityReport,
}

/// Thread-safe provider registry.
pub struct ProviderRegistry {
    providers: RwLock<BTreeMap<ProviderId, ProviderEntry>>,
    sealed: RwLock<bool>,
}

impl ProviderRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(BTreeMap::new()),
            sealed: RwLock::new(false),
        }
    }

    /// Registers a provider. Returns error if ID already exists or registry is sealed.
    pub fn register(&self, adapter: Arc<dyn ModelProvider>) -> Result<(), RegistryError> {
        let provider_id = adapter.provider_id().clone();
        let descriptor = ProviderDescriptor {
            provider_id: provider_id.clone(),
            version: adapter.version().to_string(),
            capabilities: adapter.capabilities(),
        };
        let entry = ProviderEntry {
            descriptor,
            adapter,
            enabled: true,
        };

        let mut providers = self.providers.write().map_err(|_| RegistryError::Sealed)?;
        let sealed = *self.sealed.read().map_err(|_| RegistryError::Sealed)?;

        if sealed {
            return Err(RegistryError::Sealed);
        }

        if providers.contains_key(&provider_id) {
            return Err(RegistryError::DuplicateId(provider_id.as_str().to_string()));
        }

        providers.insert(provider_id, entry);
        Ok(())
    }

    /// Enables or disables a provider.
    pub fn set_enabled(
        &self,
        provider_id: &ProviderId,
        enabled: bool,
    ) -> Result<(), RegistryError> {
        let mut providers = self.providers.write().map_err(|_| RegistryError::Sealed)?;
        if *self.sealed.read().map_err(|_| RegistryError::Sealed)? {
            return Err(RegistryError::Sealed);
        }
        let entry = providers
            .get_mut(provider_id)
            .ok_or_else(|| RegistryError::NotFound(provider_id.as_str().to_string()))?;
        entry.enabled = enabled;
        Ok(())
    }

    /// Checks if a provider is enabled.
    pub fn is_enabled(&self, provider_id: &ProviderId) -> Result<bool, RegistryError> {
        let providers = self.providers.read().map_err(|_| RegistryError::Sealed)?;
        let entry = providers
            .get(provider_id)
            .ok_or_else(|| RegistryError::NotFound(provider_id.as_str().to_string()))?;
        Ok(entry.enabled)
    }

    /// Gets a provider by ID, returns error if not found or disabled.
    pub fn get(&self, provider_id: &ProviderId) -> Result<Arc<dyn ModelProvider>, RegistryError> {
        let providers = self.providers.read().map_err(|_| RegistryError::Sealed)?;
        let entry = providers
            .get(provider_id)
            .ok_or_else(|| RegistryError::NotFound(provider_id.as_str().to_string()))?;
        if !entry.enabled {
            return Err(RegistryError::Disabled(provider_id.as_str().to_string()));
        }
        Ok(entry.adapter.clone())
    }

    /// Gets provider descriptor by ID.
    pub fn get_descriptor(
        &self,
        provider_id: &ProviderId,
    ) -> Result<ProviderDescriptor, RegistryError> {
        let providers = self.providers.read().map_err(|_| RegistryError::Sealed)?;
        let entry = providers
            .get(provider_id)
            .ok_or_else(|| RegistryError::NotFound(provider_id.as_str().to_string()))?;
        Ok(entry.descriptor.clone())
    }

    /// Finds a provider supporting the given capability requirement.
    pub fn find_by_capability(
        &self,
        modality: crate::capabilities::ModelModality,
        feature: Option<crate::capabilities::CapabilityFeature>,
    ) -> Result<Arc<dyn ModelProvider>, RegistryError> {
        let providers = self.providers.read().map_err(|_| RegistryError::Sealed)?;
        for entry in providers.values() {
            if !entry.enabled {
                continue;
            }
            let caps = &entry.descriptor.capabilities;
            if caps.modality_state(modality) != crate::capabilities::CapabilityState::Supported {
                continue;
            }
            if let Some(feature) = feature {
                if caps.feature_state(feature) != crate::capabilities::CapabilityState::Supported {
                    continue;
                }
            }
            return Ok(entry.adapter.clone());
        }
        Err(RegistryError::CapabilityMismatch(format!(
            "no enabled provider supports modality {modality:?} and feature {feature:?}"
        )))
    }

    /// Lists all registered provider IDs.
    pub fn list_providers(&self) -> Result<Vec<ProviderId>, RegistryError> {
        let providers = self.providers.read().map_err(|_| RegistryError::Sealed)?;
        Ok(providers.keys().cloned().collect())
    }

    /// Lists all enabled provider IDs.
    pub fn list_enabled_providers(&self) -> Result<Vec<ProviderId>, RegistryError> {
        let providers = self.providers.read().map_err(|_| RegistryError::Sealed)?;
        Ok(providers
            .iter()
            .filter(|(_, entry)| entry.enabled)
            .map(|(id, _)| id.clone())
            .collect())
    }

    /// Seals the registry, preventing further registrations.
    pub fn seal(&self) -> Result<(), RegistryError> {
        let mut sealed = self.sealed.write().map_err(|_| RegistryError::Sealed)?;
        *sealed = true;
        Ok(())
    }

    /// Returns true if the registry is sealed.
    pub fn is_sealed(&self) -> bool {
        self.sealed.read().map(|s| *s).unwrap_or(true)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
