//! Stable, provider-neutral model capability schema.

use crate::{ModelId, ModelProviderError, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const MODEL_CAPABILITY_SCHEMA_VERSION: u32 = 1;
const MAX_CAPABILITY_VERSION_LEN: usize = 64;
const MAX_MODALITIES: usize = 4;
const MAX_FEATURES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelModality {
    Text,
    Image,
    Audio,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityFeature {
    Streaming,
    ToolUse,
    Vision,
    AudioInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Supported,
    Unsupported,
    Unknown,
}

impl CapabilityState {
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Supported)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySource {
    Provider,
    Cache,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityLimits {
    pub max_context_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub schema_version: u32,
    pub provider_id: ProviderId,
    pub model_id: ModelId,
    pub version: String,
    pub source: CapabilitySource,
    pub modalities: BTreeMap<ModelModality, CapabilityState>,
    pub features: BTreeMap<CapabilityFeature, CapabilityState>,
    pub limits: CapabilityLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityRequirement {
    pub modalities: BTreeSet<ModelModality>,
    pub features: BTreeSet<CapabilityFeature>,
    pub min_context_tokens: Option<u32>,
    pub min_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CapabilityError {
    #[error("unsupported modality: {0:?}")]
    UnsupportedModality(ModelModality),
    #[error("unknown modality: {0:?}")]
    UnknownModality(ModelModality),
    #[error("unsupported feature: {0:?}")]
    UnsupportedFeature(CapabilityFeature),
    #[error("unknown feature: {0:?}")]
    UnknownFeature(CapabilityFeature),
    #[error("insufficient context limit: required {required}, available {available:?}")]
    InsufficientContext {
        required: u32,
        available: Option<u32>,
    },
    #[error("insufficient output limit: required {required}, available {available:?}")]
    InsufficientOutput {
        required: u32,
        available: Option<u32>,
    },
}

impl CapabilityReport {
    pub fn validate(&self) -> Result<(), ModelProviderError> {
        if self.schema_version != MODEL_CAPABILITY_SCHEMA_VERSION
            || self.version.trim().is_empty()
            || self.version.len() > MAX_CAPABILITY_VERSION_LEN
            || self.version.chars().any(char::is_control)
        {
            return Err(ModelProviderError::InvalidRequest);
        }
        if self.modalities.len() > MAX_MODALITIES || self.features.len() > MAX_FEATURES {
            return Err(ModelProviderError::InvalidRequest);
        }
        if self
            .limits
            .max_context_tokens
            .is_some_and(|value| !(1..=2_000_000).contains(&value))
            || self
                .limits
                .max_output_tokens
                .is_some_and(|value| !(1..=1_000_000).contains(&value))
        {
            return Err(ModelProviderError::InvalidRequest);
        }
        Ok(())
    }

    pub fn modality_state(&self, modality: ModelModality) -> CapabilityState {
        self.modalities
            .get(&modality)
            .copied()
            .unwrap_or(CapabilityState::Unknown)
    }

    pub fn feature_state(&self, feature: CapabilityFeature) -> CapabilityState {
        self.features
            .get(&feature)
            .copied()
            .unwrap_or(CapabilityState::Unknown)
    }

    pub fn supports_modality(&self, modality: ModelModality) -> bool {
        self.modality_state(modality).is_supported()
    }

    pub fn supports_feature(&self, feature: CapabilityFeature) -> bool {
        self.feature_state(feature).is_supported()
    }

    pub fn check_compatibility(
        &self,
        requirement: &CapabilityRequirement,
    ) -> Result<(), CapabilityError> {
        for modality in &requirement.modalities {
            match self.modality_state(*modality) {
                CapabilityState::Supported => {}
                CapabilityState::Unsupported => {
                    return Err(CapabilityError::UnsupportedModality(*modality))
                }
                CapabilityState::Unknown => {
                    return Err(CapabilityError::UnknownModality(*modality))
                }
            }
        }
        for feature in &requirement.features {
            match self.feature_state(*feature) {
                CapabilityState::Supported => {}
                CapabilityState::Unsupported => {
                    return Err(CapabilityError::UnsupportedFeature(*feature))
                }
                CapabilityState::Unknown => return Err(CapabilityError::UnknownFeature(*feature)),
            }
        }
        if let Some(required) = requirement.min_context_tokens {
            if self.limits.max_context_tokens.unwrap_or_default() < required {
                return Err(CapabilityError::InsufficientContext {
                    required,
                    available: self.limits.max_context_tokens,
                });
            }
        }
        if let Some(required) = requirement.min_output_tokens {
            if self.limits.max_output_tokens.unwrap_or_default() < required {
                return Err(CapabilityError::InsufficientOutput {
                    required,
                    available: self.limits.max_output_tokens,
                });
            }
        }
        Ok(())
    }
}
