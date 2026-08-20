//! OpenAI provider descriptor over the OpenAI-compatible adapter.
//!
//! This crate declares provider identity, planning-time model mappings, and
//! capabilities. It does not perform discovery or store credentials.

use provider_adapter_openai_compatible::{
    AdapterError, EndpointPolicy, HttpTransport, OpenAiCompatibleAdapter,
};
use provider_core::capabilities::{
    CapabilityError, CapabilityFeature, CapabilityLimits, CapabilityReport, CapabilitySource,
    CapabilityState, ModelModality,
};
use provider_core::request::NormalizedRequest;
use provider_core::response::NormalizedResponse;
use provider_core::stream::{StreamEvent, StreamEventPayload};
use provider_core::{CancellationToken, CredentialRef, ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const OPENAI_PROVIDER_VERSION: &str = "openai-descriptor-1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OpenAiModel {
    Gpt4oMini,
    Gpt4o,
}

impl OpenAiModel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gpt4oMini => "gpt-4o-mini",
            Self::Gpt4o => "gpt-4o",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiModelDescriptor {
    pub model_id: ModelId,
    pub capabilities: CapabilityReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiProviderDescriptor {
    provider_id: ProviderId,
    version: String,
    models: Vec<OpenAiModelDescriptor>,
}

#[derive(Debug, Error)]
pub enum ProviderDescriptorError {
    #[error("normalized request targets another provider")]
    ProviderMismatch,
    #[error("OpenAI model is not declared: {0}")]
    UnsupportedModel(String),
    #[error("normalized request capability is unsupported: {0:?}")]
    UnsupportedCapability(CapabilityError),
    #[error("normalized request is invalid")]
    InvalidRequest,
    #[error("OpenAI adapter error: {0}")]
    Adapter(#[from] AdapterError),
}

impl OpenAiProviderDescriptor {
    pub fn new() -> Self {
        let provider_id = ProviderId::parse("openai").expect("static provider id is valid");
        let models = [(OpenAiModel::Gpt4oMini, false), (OpenAiModel::Gpt4o, true)]
            .into_iter()
            .map(|(model, vision)| OpenAiModelDescriptor {
                model_id: ModelId::parse(model.as_str()).expect("static model id is valid"),
                capabilities: capabilities_for(&provider_id, model, vision),
            })
            .collect();
        Self {
            provider_id,
            version: OPENAI_PROVIDER_VERSION.into(),
            models,
        }
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn models(&self) -> &[OpenAiModelDescriptor] {
        &self.models
    }

    pub fn model(&self, model: OpenAiModel) -> ModelId {
        ModelId::parse(model.as_str()).expect("static model id is valid")
    }

    pub fn capabilities(
        &self,
        model: OpenAiModel,
    ) -> Result<&CapabilityReport, ProviderDescriptorError> {
        self.models
            .iter()
            .find(|descriptor| descriptor.model_id.as_str() == model.as_str())
            .map(|descriptor| &descriptor.capabilities)
            .ok_or_else(|| ProviderDescriptorError::UnsupportedModel(model.as_str().into()))
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
        let capabilities = self
            .models
            .iter()
            .find(|descriptor| descriptor.model_id == request.model_id)
            .ok_or_else(|| {
                ProviderDescriptorError::UnsupportedModel(request.model_id.as_str().into())
            })?
            .capabilities
            .clone();
        capabilities
            .check_compatibility(&request.capabilities)
            .map_err(ProviderDescriptorError::UnsupportedCapability)
    }
}

impl Default for OpenAiProviderDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OpenAiProvider<T> {
    descriptor: OpenAiProviderDescriptor,
    adapter: OpenAiCompatibleAdapter<T>,
}

impl<T: HttpTransport> OpenAiProvider<T> {
    pub fn new(
        endpoint: EndpointPolicy,
        credential_ref: CredentialRef,
        transport: T,
        timeout: Duration,
    ) -> Result<Self, ProviderDescriptorError> {
        Ok(Self {
            descriptor: OpenAiProviderDescriptor::new(),
            adapter: OpenAiCompatibleAdapter::new(endpoint, credential_ref, transport, timeout)?,
        })
    }

    pub fn descriptor(&self) -> &OpenAiProviderDescriptor {
        &self.descriptor
    }

    pub fn complete(
        &self,
        request: NormalizedRequest,
        cancellation: &CancellationToken,
    ) -> Result<NormalizedResponse, ProviderDescriptorError> {
        self.descriptor.validate_request(&request)?;
        self.adapter
            .complete(request, cancellation)
            .map(|mut response| {
                response.provider_id = self.descriptor.provider_id.clone();
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
        self.adapter
            .stream(request, cancellation)
            .map(|mut events| {
                for event in &mut events {
                    if let StreamEventPayload::Start { provider_id, .. } = &mut event.payload {
                        *provider_id = self.descriptor.provider_id.clone();
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
                ProviderId::parse("openai").expect("static provider id is valid");
            ProviderDescriptorError::Adapter(AdapterError::Response(response))
        }
        other => ProviderDescriptorError::Adapter(other),
    }
}

fn capabilities_for(
    provider_id: &ProviderId,
    model: OpenAiModel,
    vision: bool,
) -> CapabilityReport {
    let mut modalities = std::collections::BTreeMap::from([
        (ModelModality::Text, CapabilityState::Supported),
        (
            ModelModality::Image,
            if vision {
                CapabilityState::Supported
            } else {
                CapabilityState::Unsupported
            },
        ),
        (ModelModality::Audio, CapabilityState::Unsupported),
        (ModelModality::Video, CapabilityState::Unsupported),
    ]);
    let features = std::collections::BTreeMap::from([
        (CapabilityFeature::Streaming, CapabilityState::Supported),
        (CapabilityFeature::ToolUse, CapabilityState::Supported),
        (
            CapabilityFeature::Vision,
            if vision {
                CapabilityState::Supported
            } else {
                CapabilityState::Unsupported
            },
        ),
        (CapabilityFeature::AudioInput, CapabilityState::Unsupported),
    ]);
    let report = CapabilityReport {
        schema_version: 1,
        provider_id: provider_id.clone(),
        model_id: ModelId::parse(model.as_str()).expect("static model id is valid"),
        version: OPENAI_PROVIDER_VERSION.into(),
        source: CapabilitySource::Provider,
        modalities: std::mem::take(&mut modalities),
        features,
        limits: CapabilityLimits {
            max_context_tokens: Some(match model {
                OpenAiModel::Gpt4oMini => 128_000,
                OpenAiModel::Gpt4o => 128_000,
            }),
            max_output_tokens: Some(match model {
                OpenAiModel::Gpt4oMini => 16_384,
                OpenAiModel::Gpt4o => 16_384,
            }),
        },
    };
    report
        .validate()
        .expect("static capability report is valid");
    report
}
