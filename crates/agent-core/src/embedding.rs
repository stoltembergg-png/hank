//! Provider-agnostic embedding contract with deterministic offline mock.

use crate::ids::ProjectId;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingInput {
    pub reference: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub project_id: Option<ProjectId>,
    pub trace_id: String,
    pub model: String,
    pub model_version: String,
    pub dimensions: usize,
    pub inputs: Vec<EmbeddingInput>,
    pub budget_available: bool,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingPolicy {
    pub max_dimensions: usize,
    pub max_batch: usize,
    pub max_reference_bytes: usize,
}

impl Default for EmbeddingPolicy {
    fn default() -> Self {
        Self {
            max_dimensions: 4096,
            max_batch: 128,
            max_reference_bytes: 256,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingResponse {
    pub model: String,
    pub model_version: String,
    pub dimensions: usize,
    pub vectors: Vec<Vec<f32>>,
    pub trace_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EmbeddingError {
    #[error("embedding project is missing")]
    MissingProject,
    #[error("embedding trace is missing")]
    MissingTrace,
    #[error("embedding model identity is invalid")]
    InvalidModel,
    #[error("embedding dimensions are invalid")]
    InvalidDimensions,
    #[error("embedding batch is too large")]
    BatchTooLarge,
    #[error("embedding reference is invalid")]
    InvalidReference,
    #[error("embedding budget is unavailable")]
    BudgetUnavailable,
    #[error("embedding request was cancelled")]
    Cancelled,
}

pub struct MockEmbeddingProvider;

impl MockEmbeddingProvider {
    pub fn embed(
        request: &EmbeddingRequest,
        policy: &EmbeddingPolicy,
    ) -> Result<EmbeddingResponse, EmbeddingError> {
        if request.project_id.is_none() {
            return Err(EmbeddingError::MissingProject);
        }
        if request.trace_id.is_empty() {
            return Err(EmbeddingError::MissingTrace);
        }
        if request.model.is_empty() || request.model_version.is_empty() {
            return Err(EmbeddingError::InvalidModel);
        }
        if request.dimensions == 0 || request.dimensions > policy.max_dimensions {
            return Err(EmbeddingError::InvalidDimensions);
        }
        if request.inputs.is_empty() || request.inputs.len() > policy.max_batch {
            return Err(EmbeddingError::BatchTooLarge);
        }
        if request.inputs.iter().any(|input| {
            input.reference.is_empty()
                || input.reference.len() > policy.max_reference_bytes
                || input.reference.chars().any(char::is_control)
        }) {
            return Err(EmbeddingError::InvalidReference);
        }
        if !request.budget_available {
            return Err(EmbeddingError::BudgetUnavailable);
        }
        if request.cancelled {
            return Err(EmbeddingError::Cancelled);
        }
        let vectors = request
            .inputs
            .iter()
            .map(|input| vector(&input.reference, request.dimensions))
            .collect();
        Ok(EmbeddingResponse {
            model: request.model.clone(),
            model_version: request.model_version.clone(),
            dimensions: request.dimensions,
            vectors,
            trace_id: request.trace_id.clone(),
        })
    }
}

fn vector(reference: &str, dimensions: usize) -> Vec<f32> {
    let mut seed = 2_166_136_261u32;
    for byte in reference.as_bytes() {
        seed ^= u32::from(*byte);
        seed = seed.wrapping_mul(16_777_619);
    }
    (0..dimensions)
        .map(|index| {
            seed = seed.wrapping_add(index as u32).rotate_left(5);
            (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}
