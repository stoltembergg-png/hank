//! Stable memory taxonomy independent from storage and retrieval.

use crate::memory::ProvenanceSource;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Fact,
    Preference,
    Decision,
    Lesson,
    ProjectContext,
    TechnicalContext,
    Failure,
    SuccessfulPattern,
}

impl MemoryKind {
    pub fn parse(value: &str) -> Result<Self, TaxonomyError> {
        match value {
            "fact" => Ok(Self::Fact),
            "preference" => Ok(Self::Preference),
            "decision" => Ok(Self::Decision),
            "lesson" => Ok(Self::Lesson),
            "project_context" => Ok(Self::ProjectContext),
            "technical_context" => Ok(Self::TechnicalContext),
            "failure" => Ok(Self::Failure),
            "successful_pattern" => Ok(Self::SuccessfulPattern),
            _ => Err(TaxonomyError::UnknownType),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaxonomyHints {
    pub retention_days: u32,
    pub minimum_importance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxonomyVersion;

impl TaxonomyVersion {
    pub const CURRENT: Self = Self;

    pub const fn as_str(self) -> &'static str {
        "1"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TaxonomyError {
    #[error("unknown memory type")]
    UnknownType,
    #[error("memory content is empty")]
    EmptyContent,
    #[error("memory content contains an instruction hierarchy claim")]
    InstructionClaim,
    #[error("memory content resembles a secret")]
    SecretLikeContent,
}

pub struct MemoryTaxonomy;

impl MemoryTaxonomy {
    pub fn hints(kind: MemoryKind) -> TaxonomyHints {
        match kind {
            MemoryKind::Fact | MemoryKind::TechnicalContext => TaxonomyHints {
                retention_days: 365,
                minimum_importance: 0.2,
            },
            MemoryKind::Preference | MemoryKind::Decision => TaxonomyHints {
                retention_days: 180,
                minimum_importance: 0.4,
            },
            MemoryKind::Lesson | MemoryKind::SuccessfulPattern => TaxonomyHints {
                retention_days: 365,
                minimum_importance: 0.5,
            },
            MemoryKind::ProjectContext => TaxonomyHints {
                retention_days: 90,
                minimum_importance: 0.3,
            },
            MemoryKind::Failure => TaxonomyHints {
                retention_days: 30,
                minimum_importance: 0.3,
            },
        }
    }

    pub fn validate(
        _kind: MemoryKind,
        content: &str,
        _source: ProvenanceSource,
    ) -> Result<(), TaxonomyError> {
        if content.trim().is_empty() {
            return Err(TaxonomyError::EmptyContent);
        }
        let lowered = content.to_ascii_lowercase();
        if lowered.contains("<system>")
            || lowered.contains("<developer>")
            || lowered.contains("ignore previous instructions")
        {
            return Err(TaxonomyError::InstructionClaim);
        }
        if lowered.contains("api_key=")
            || lowered.contains("authorization: bearer ")
            || lowered.contains("password=")
        {
            return Err(TaxonomyError::SecretLikeContent);
        }
        Ok(())
    }
}
