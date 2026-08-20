//! Isolated OpenRouter route descriptor over the compatible adapter.

pub use provider_adapter_openai_compatible::AdapterError;
use provider_adapter_openai_compatible::{EndpointPolicy, HttpTransport, OpenAiCompatibleAdapter};
use provider_core::capabilities::{
    CapabilityError, CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::request::NormalizedRequest;
use provider_core::response::NormalizedResponse;
use provider_core::stream::{StreamEvent, StreamEventPayload};
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use thiserror::Error;

const OPENROUTER_VERSION: &str = "openrouter-descriptor-1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenRouterModel {
    OpenAiGpt4oMini,
    AnthropicClaude35Sonnet,
}

impl OpenRouterModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiGpt4oMini => "openai/gpt-4o-mini",
            Self::AnthropicClaude35Sonnet => "anthropic/claude-3-5-sonnet",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMetadata {
    pub logical_model: ModelId,
    pub upstream_provider: String,
    pub upstream_model: ModelId,
    pub route_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterModelDescriptor {
    pub route: RouteMetadata,
    pub capabilities: CapabilityReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterProviderDescriptor {
    provider_id: ProviderId,
    version: String,
    models: Vec<OpenRouterModelDescriptor>,
}

#[derive(Debug, Error)]
pub enum ProviderDescriptorError {
    #[error("normalized request targets another provider")]
    ProviderMismatch,
    #[error("OpenRouter route is not declared: {0}")]
    UnsupportedRoute(String),
    #[error("normalized request capability is unsupported: {0:?}")]
    UnsupportedCapability(CapabilityError),
    #[error("normalized request is invalid")]
    InvalidRequest,
    #[error("OpenRouter adapter error: {0}")]
    Adapter(#[from] AdapterError),
}

impl OpenRouterProviderDescriptor {
    pub fn new() -> Self {
        let provider_id = ProviderId::parse("openrouter").expect("static provider id is valid");
        let models = [
            (OpenRouterModel::OpenAiGpt4oMini, "openai", "gpt-4o-mini"),
            (
                OpenRouterModel::AnthropicClaude35Sonnet,
                "anthropic",
                "claude-3-5-sonnet",
            ),
        ]
        .into_iter()
        .map(|(model, upstream_provider, upstream_model)| {
            let logical_model = ModelId::parse(model.as_str()).expect("static route is valid");
            OpenRouterModelDescriptor {
                route: RouteMetadata {
                    logical_model: logical_model.clone(),
                    upstream_provider: upstream_provider.into(),
                    upstream_model: ModelId::parse(upstream_model).expect("static model is valid"),
                    route_label: "direct".into(),
                },
                capabilities: capabilities_for(&provider_id, logical_model),
            }
        })
        .collect();
        Self {
            provider_id,
            version: OPENROUTER_VERSION.into(),
            models,
        }
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn models(&self) -> &[OpenRouterModelDescriptor] {
        &self.models
    }

    pub fn route(&self, model: OpenRouterModel) -> Result<&RouteMetadata, ProviderDescriptorError> {
        self.models
            .iter()
            .find(|descriptor| descriptor.route.logical_model.as_str() == model.as_str())
            .map(|descriptor| &descriptor.route)
            .ok_or_else(|| ProviderDescriptorError::UnsupportedRoute(model.as_str().into()))
    }

    pub fn capabilities(
        &self,
        model: OpenRouterModel,
    ) -> Result<&CapabilityReport, ProviderDescriptorError> {
        self.models
            .iter()
            .find(|descriptor| descriptor.route.logical_model.as_str() == model.as_str())
            .map(|descriptor| &descriptor.capabilities)
            .ok_or_else(|| ProviderDescriptorError::UnsupportedRoute(model.as_str().into()))
    }

    fn route_for_request(
        &self,
        request: &NormalizedRequest,
    ) -> Result<&RouteMetadata, ProviderDescriptorError> {
        self.models
            .iter()
            .find(|descriptor| descriptor.route.logical_model == request.model_id)
            .map(|descriptor| &descriptor.route)
            .ok_or_else(|| {
                ProviderDescriptorError::UnsupportedRoute(request.model_id.as_str().into())
            })
    }

    pub fn validate_request(
        &self,
        request: &NormalizedRequest,
    ) -> Result<(), ProviderDescriptorError> {
        if request.provider_id != self.provider_id {
            return Err(ProviderDescriptorError::ProviderMismatch);
        }
        request
            .validate()
            .map_err(|_| ProviderDescriptorError::InvalidRequest)?;
        let descriptor = self
            .models
            .iter()
            .find(|descriptor| descriptor.route.logical_model == request.model_id)
            .ok_or_else(|| {
                ProviderDescriptorError::UnsupportedRoute(request.model_id.as_str().into())
            })?;
        descriptor
            .capabilities
            .check_compatibility(&request.capabilities)
            .map_err(ProviderDescriptorError::UnsupportedCapability)
    }
}

impl Default for OpenRouterProviderDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OpenRouterProvider<T> {
    descriptor: OpenRouterProviderDescriptor,
    adapter: OpenAiCompatibleAdapter<T>,
}

impl<T: HttpTransport> OpenRouterProvider<T> {
    pub fn new(
        endpoint: EndpointPolicy,
        credential_ref: CredentialRef,
        transport: T,
        timeout: Duration,
    ) -> Result<Self, ProviderDescriptorError> {
        Ok(Self {
            descriptor: OpenRouterProviderDescriptor::new(),
            adapter: OpenAiCompatibleAdapter::new(endpoint, credential_ref, transport, timeout)?,
        })
    }

    pub fn descriptor(&self) -> &OpenRouterProviderDescriptor {
        &self.descriptor
    }

    pub fn complete(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<NormalizedResponse, ProviderDescriptorError> {
        self.descriptor.validate_request(&request)?;
        let route = self.descriptor.route_for_request(&request)?.clone();
        let mut upstream_request = request.clone();
        upstream_request.model_id = route.upstream_model;
        self.adapter
            .complete(upstream_request, cancellation)
            .map(|mut response| {
                response.provider_id = self.descriptor.provider_id.clone();
                response.model_id = route.logical_model.clone();
                response
            })
            .map_err(rewrite_adapter_error)
    }

    pub fn stream(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<Vec<StreamEvent>, ProviderDescriptorError> {
        self.descriptor.validate_request(&request)?;
        let route = self.descriptor.route_for_request(&request)?.clone();
        let mut upstream_request = request.clone();
        upstream_request.model_id = route.upstream_model;
        self.adapter
            .stream(upstream_request, cancellation)
            .map(|mut events| {
                for event in &mut events {
                    if let StreamEventPayload::Start {
                        provider_id,
                        model_id,
                    } = &mut event.payload
                    {
                        *provider_id = self.descriptor.provider_id.clone();
                        *model_id = route.logical_model.clone();
                    }
                }
                events
            })
            .map_err(rewrite_adapter_error)
    }
}

fn rewrite_adapter_error(error: AdapterError) -> ProviderDescriptorError {
    match error {
        AdapterError::Response(mut response) => {
            response.provider_id =
                ProviderId::parse("openrouter").expect("static provider id is valid");
            ProviderDescriptorError::Adapter(AdapterError::Response(response))
        }
        other => ProviderDescriptorError::Adapter(other),
    }
}

fn capabilities_for(provider_id: &ProviderId, model: ModelId) -> CapabilityReport {
    let report = CapabilityReport {
        schema_version: 1,
        provider_id: provider_id.clone(),
        model_id: model,
        version: OPENROUTER_VERSION.into(),
        source: CapabilitySource::Provider,
        modalities: BTreeMap::from([
            (ModelModality::Text, CapabilityState::Supported),
            (ModelModality::Image, CapabilityState::Supported),
            (ModelModality::Audio, CapabilityState::Unsupported),
            (ModelModality::Video, CapabilityState::Unsupported),
        ]),
        features: BTreeMap::from([
            (CapabilityFeature::Streaming, CapabilityState::Supported),
            (CapabilityFeature::ToolUse, CapabilityState::Supported),
            (CapabilityFeature::Vision, CapabilityState::Supported),
            (CapabilityFeature::AudioInput, CapabilityState::Unsupported),
        ]),
        limits: CapabilityLimits {
            max_context_tokens: Some(128_000),
            max_output_tokens: Some(16_384),
        },
    };
    report
        .validate()
        .expect("static capability report is valid");
    report
}
