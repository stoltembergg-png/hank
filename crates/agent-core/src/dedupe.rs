//! Deterministic pre-persistence deduplication without semantic/vector matching.

use crate::ids::ProjectId;
use crate::taxonomy::MemoryKind;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedupeInput {
    pub project_id: ProjectId,
    pub agent_id: Option<String>,
    pub kind: MemoryKind,
    pub canonical_key: String,
    pub content: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedupeEntry {
    pub id: String,
    pub project_id: ProjectId,
    pub agent_id: Option<String>,
    pub kind: MemoryKind,
    pub canonical_key: String,
    pub normalized_content: String,
}

impl DedupeEntry {
    pub fn from_input(id: String, input: &DedupeInput) -> Self {
        Self {
            id,
            project_id: input.project_id,
            agent_id: input.agent_id.clone(),
            kind: input.kind,
            canonical_key: normalize(&input.canonical_key),
            normalized_content: normalize(&input.content),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupeDecision {
    New,
    Duplicate { existing_id: String },
    Conflict { existing_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DedupeError {
    #[error("dedupe entry is invalid")]
    InvalidEntry,
    #[error("dedupe identity is duplicated")]
    DuplicateIdentity,
    #[error("dedupe entry is not found")]
    NotFound,
}

#[derive(Debug, Default)]
pub struct DedupeIndex {
    entries: Vec<DedupeEntry>,
}

impl DedupeIndex {
    pub fn commit(&mut self, entry: DedupeEntry) -> Result<(), DedupeError> {
        if entry.id.is_empty()
            || entry.id.len() > 128
            || entry.canonical_key.is_empty()
            || entry.normalized_content.is_empty()
            || entry.normalized_content.len() > 16 * 1024
        {
            return Err(DedupeError::InvalidEntry);
        }
        if self.entries.iter().any(|existing| existing.id == entry.id) {
            return Err(DedupeError::DuplicateIdentity);
        }
        self.entries.push(entry);
        Ok(())
    }

    pub fn decide(&self, input: &DedupeInput) -> Result<DedupeDecision, DedupeError> {
        if input.canonical_key.trim().is_empty()
            || input.content.trim().is_empty()
            || input.content.len() > 16 * 1024
            || input.trace_id.is_empty()
        {
            return Err(DedupeError::InvalidEntry);
        }
        let key = normalize(&input.canonical_key);
        let content = normalize(&input.content);
        for entry in &self.entries {
            if entry.project_id != input.project_id
                || entry.agent_id != input.agent_id
                || entry.kind != input.kind
                || entry.canonical_key != key
            {
                continue;
            }
            return if entry.normalized_content == content {
                Ok(DedupeDecision::Duplicate {
                    existing_id: entry.id.clone(),
                })
            } else {
                Ok(DedupeDecision::Conflict {
                    existing_id: entry.id.clone(),
                })
            };
        }
        Ok(DedupeDecision::New)
    }

    pub fn rollback(&mut self, id: &str) -> Result<(), DedupeError> {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        if self.entries.len() == before {
            return Err(DedupeError::NotFound);
        }
        Ok(())
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
