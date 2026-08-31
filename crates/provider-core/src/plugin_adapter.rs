use crate::capabilities::CapabilityReport;
use crate::{
    CancellationToken, HealthStatus, ModelProvider, ModelProviderError, ProviderFuture, ProviderId,
    ProviderRequest, ProviderResponse, ProviderStream, StreamConfig,
};
use std::sync::Arc;

const MAX_IDENTITY: usize = 256;

pub struct ProviderPluginAdapter {
    inner: Arc<dyn ModelProvider>,
    plugin_id: String,
    plugin_digest: String,
    approved: bool,
}

impl ProviderPluginAdapter {
    pub fn new(
        inner: Arc<dyn ModelProvider>,
        plugin_id: &str,
        plugin_digest: &str,
        approved: bool,
    ) -> Result<Self, ModelProviderError> {
        if plugin_id.is_empty()
            || plugin_digest.is_empty()
            || plugin_id.len() > MAX_IDENTITY
            || plugin_digest.len() > MAX_IDENTITY
        {
            return Err(ModelProviderError::InvalidProviderId);
        }
        Ok(Self {
            inner,
            plugin_id: plugin_id.into(),
            plugin_digest: plugin_digest.into(),
            approved,
        })
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn plugin_digest(&self) -> &str {
        &self.plugin_digest
    }

    fn allowed(&self) -> bool {
        self.approved
    }
}

impl ModelProvider for ProviderPluginAdapter {
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
        request: ProviderRequest,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProviderResponse, ModelProviderError>> {
        if !self.allowed() {
            return Box::pin(async { Err(ModelProviderError::Unavailable) });
        }
        self.inner.complete(request, cancellation)
    }

    fn stream(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
        config: StreamConfig,
    ) -> Result<ProviderStream<'_>, ModelProviderError> {
        if !self.allowed() {
            return Err(ModelProviderError::Unavailable);
        }
        self.inner.stream(request, cancellation, config)
    }

    fn list_models(
        &self,
    ) -> ProviderFuture<'_, Result<Vec<crate::ModelDescriptor>, ModelProviderError>> {
        if !self.allowed() {
            return Box::pin(async { Err(ModelProviderError::Unavailable) });
        }
        self.inner.list_models()
    }

    fn health(&self) -> ProviderFuture<'_, Result<HealthStatus, ModelProviderError>> {
        if !self.allowed() {
            return Box::pin(async { Err(ModelProviderError::Unavailable) });
        }
        self.inner.health()
    }
}
