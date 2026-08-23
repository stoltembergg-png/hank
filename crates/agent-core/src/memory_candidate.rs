//! Deterministic, data-only memory candidate extraction.

use crate::ids::MemoryId;
use crate::ids::{ProjectId, SessionId};
use crate::memory::ProvenanceSource;
use crate::taxonomy::{MemoryKind, MemoryTaxonomy, TaxonomyError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRequest {
    pub project_id: Option<ProjectId>,
    pub session_id: Option<SessionId>,
    pub source_message_id: String,
    pub kind: MemoryKind,
    pub content: String,
    pub source: ProvenanceSource,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub id: MemoryId,
    pub project_id: Option<ProjectId>,
    pub session_id: Option<SessionId>,
    pub source_message_id: String,
    pub kind: MemoryKind,
    pub content: String,
    pub source: ProvenanceSource,
    pub confidence: f32,
    pub status: CandidateStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CandidateError {
    #[error("candidate project is required")]
    MissingProject,
    #[error("candidate source message is required")]
    MissingSource,
    #[error("candidate confidence is invalid")]
    InvalidConfidence,
    #[error("candidate content is invalid")]
    InvalidContent,
    #[error("candidate content is not trusted")]
    UntrustedContent,
}

pub struct MemoryCandidateExtractor;

impl MemoryCandidateExtractor {
    pub fn extract(request: CandidateRequest) -> Result<MemoryCandidate, CandidateError> {
        let project_id = request.project_id.ok_or(CandidateError::MissingProject)?;
        if request.source_message_id.trim().is_empty() || request.source_message_id.len() > 128 {
            return Err(CandidateError::MissingSource);
        }
        if !request.confidence.is_finite() || !(0.0..=1.0).contains(&request.confidence) {
            return Err(CandidateError::InvalidConfidence);
        }
        if request.content.trim().is_empty() || request.content.len() > 16 * 1024 {
            return Err(CandidateError::InvalidContent);
        }
        MemoryTaxonomy::validate(request.kind, &request.content, request.source).map_err(
            |error| match error {
                TaxonomyError::InstructionClaim | TaxonomyError::SecretLikeContent => {
                    CandidateError::UntrustedContent
                }
                _ => CandidateError::InvalidContent,
            },
        )?;
        Ok(MemoryCandidate {
            id: MemoryId::new(),
            project_id: Some(project_id),
            session_id: request.session_id,
            source_message_id: request.source_message_id,
            kind: request.kind,
            content: request.content,
            source: request.source,
            confidence: request.confidence,
            status: CandidateStatus::Pending,
        })
    }
}
