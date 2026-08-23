//! Deterministic, explainable memory importance scoring.

use crate::ids::ProjectId;
use crate::memory::ProvenanceSource;
use crate::taxonomy::MemoryKind;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ImportanceInput {
    pub project_id: ProjectId,
    pub kind: MemoryKind,
    pub source: ProvenanceSource,
    pub confidence: f32,
    pub recency_days: u32,
    pub repetition: u32,
    pub policy_version: String,
    pub trace_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ImportancePolicy {
    pub threshold: f32,
    pub max_recency_days: u32,
    pub max_repetition: u32,
}

impl Default for ImportancePolicy {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            max_recency_days: 365,
            max_repetition: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportanceScore {
    pub value: f32,
    pub eligible: bool,
    pub policy_version: String,
    pub trace_id: String,
    pub factors: Vec<String>,
    pub explanation: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ImportanceError {
    #[error("importance policy is invalid")]
    InvalidPolicy,
    #[error("importance identity is missing")]
    MissingIdentity,
    #[error("importance confidence is invalid")]
    InvalidConfidence,
}

pub struct MemoryImportanceScorer;

impl MemoryImportanceScorer {
    pub fn score(
        input: &ImportanceInput,
        policy: &ImportancePolicy,
    ) -> Result<ImportanceScore, ImportanceError> {
        if !policy.threshold.is_finite()
            || !(0.0..=1.0).contains(&policy.threshold)
            || policy.max_recency_days == 0
            || policy.max_repetition == 0
            || input.policy_version.is_empty()
            || input.trace_id.is_empty()
        {
            return Err(ImportanceError::InvalidPolicy);
        }
        if !input.confidence.is_finite() || !(0.0..=1.0).contains(&input.confidence) {
            return Err(ImportanceError::InvalidConfidence);
        }
        if input.project_id.to_string().is_empty() {
            return Err(ImportanceError::MissingIdentity);
        }

        let recency = 1.0
            - (input.recency_days.min(policy.max_recency_days) as f32
                / policy.max_recency_days as f32);
        let repetition = (input.repetition.min(policy.max_repetition) as f32
            / policy.max_repetition as f32)
            .sqrt();
        let value = (input.confidence * 0.6 + recency * 0.2 + repetition * 0.2).clamp(0.0, 1.0);
        let factors = vec![
            format!("kind:{:?}", input.kind),
            format!("source:{:?}", input.source),
            "content:excluded".into(),
        ];
        let explanation = vec![
            format!("confidence:{:.3}", input.confidence),
            format!("recency_days:{}", input.recency_days),
            format!("repetition:{}", input.repetition),
        ];
        Ok(ImportanceScore {
            value,
            eligible: value >= policy.threshold,
            policy_version: input.policy_version.clone(),
            trace_id: input.trace_id.clone(),
            factors,
            explanation,
        })
    }
}
