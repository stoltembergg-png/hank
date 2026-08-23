//! Policy-first, read-only selection of memory candidates for context.

use super::{ContextEntry, ContextSourceKind};
use agent_core::{AgentId, MemoryStatus, ProjectId, ProvenanceSource};
use provider_core::CancellationToken;
use std::collections::BTreeSet;
use thiserror::Error;

const MAX_CANDIDATES: usize = 128;
const MAX_MEMORY_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct MemoryContextCandidate {
    pub memory_id: String,
    pub project_id: ProjectId,
    pub agent_id: Option<AgentId>,
    pub status: MemoryStatus,
    pub content: String,
    pub estimated_tokens: u32,
    pub confidence: f32,
    pub importance: f32,
    pub recency_rank: u32,
    pub provenance: ProvenanceSource,
    pub duplicate_key: Option<String>,
    pub policy_allowed: bool,
    pub capability_allowed: bool,
}

#[derive(Debug, Clone)]
pub struct MemorySelectionRequest {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub candidates: Vec<MemoryContextCandidate>,
    pub max_tokens: u32,
    pub trace_id: String,
    pub cancellation: CancellationToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOmissionReason {
    ProjectScope,
    AgentScope,
    Status,
    Policy,
    Capability,
    Invalid,
    HostileContent,
    Duplicate,
    Budget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryOmission {
    pub memory_id: String,
    pub reason: MemoryOmissionReason,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedMemory {
    pub memory_id: String,
    pub score: f32,
    pub provenance: ProvenanceSource,
    pub context: ContextEntry,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemorySelectionResult {
    pub selected: Vec<SelectedMemory>,
    pub omitted: Vec<MemoryOmission>,
    pub consumed_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MemorySelectionError {
    #[error("memory selection request is invalid")]
    Invalid,
    #[error("memory selection was cancelled")]
    Cancelled,
}

pub struct MemorySelector;

impl MemorySelector {
    pub fn select(
        request: MemorySelectionRequest,
    ) -> Result<MemorySelectionResult, MemorySelectionError> {
        if request.cancellation.is_cancelled() {
            return Err(MemorySelectionError::Cancelled);
        }
        if request.trace_id.is_empty()
            || request.trace_id.len() > 128
            || request.trace_id.chars().any(char::is_control)
            || request.max_tokens == 0
            || request.candidates.len() > MAX_CANDIDATES
        {
            return Err(MemorySelectionError::Invalid);
        }

        let mut omitted = Vec::new();
        let mut allowed = Vec::new();
        for candidate in request.candidates {
            if request.cancellation.is_cancelled() {
                return Err(MemorySelectionError::Cancelled);
            }
            let reason = if candidate.project_id != request.project_id {
                Some(MemoryOmissionReason::ProjectScope)
            } else if candidate.agent_id.is_some_and(|id| id != request.agent_id) {
                Some(MemoryOmissionReason::AgentScope)
            } else if candidate.status != MemoryStatus::Approved {
                Some(MemoryOmissionReason::Status)
            } else if !candidate.policy_allowed {
                Some(MemoryOmissionReason::Policy)
            } else if !candidate.capability_allowed {
                Some(MemoryOmissionReason::Capability)
            } else if !valid_candidate(&candidate) {
                Some(MemoryOmissionReason::Invalid)
            } else if hostile(&candidate.content) {
                Some(MemoryOmissionReason::HostileContent)
            } else {
                None
            };
            if let Some(reason) = reason {
                omitted.push(MemoryOmission {
                    memory_id: candidate.memory_id,
                    reason,
                });
            } else {
                allowed.push(candidate);
            }
        }

        allowed.sort_by(|left, right| {
            score(right)
                .total_cmp(&score(left))
                .then_with(|| left.memory_id.cmp(&right.memory_id))
        });
        let mut seen = BTreeSet::new();
        let mut selected = Vec::new();
        let mut consumed_tokens = 0u32;
        for candidate in allowed {
            let duplicate = candidate
                .duplicate_key
                .as_ref()
                .is_some_and(|key| !seen.insert(key.clone()));
            if duplicate {
                omitted.push(MemoryOmission {
                    memory_id: candidate.memory_id,
                    reason: MemoryOmissionReason::Duplicate,
                });
                continue;
            }
            if consumed_tokens.saturating_add(candidate.estimated_tokens) > request.max_tokens {
                omitted.push(MemoryOmission {
                    memory_id: candidate.memory_id,
                    reason: MemoryOmissionReason::Budget,
                });
                continue;
            }
            consumed_tokens = consumed_tokens.saturating_add(candidate.estimated_tokens);
            selected.push(SelectedMemory {
                memory_id: candidate.memory_id.clone(),
                score: score(&candidate),
                provenance: candidate.provenance,
                context: ContextEntry {
                    source_id: candidate.memory_id,
                    kind: ContextSourceKind::Memory,
                    content: candidate.content,
                    untrusted: true,
                    tool_executable: false,
                },
            });
        }
        Ok(MemorySelectionResult {
            selected,
            omitted,
            consumed_tokens,
        })
    }
}

fn valid_candidate(candidate: &MemoryContextCandidate) -> bool {
    !candidate.memory_id.is_empty()
        && candidate.memory_id.len() <= 128
        && candidate.content.len() <= MAX_MEMORY_BYTES
        && candidate.estimated_tokens > 0
        && candidate.confidence.is_finite()
        && candidate.importance.is_finite()
        && (0.0..=1.0).contains(&candidate.confidence)
        && (0.0..=1.0).contains(&candidate.importance)
        && candidate.recency_rank > 0
}

fn hostile(content: &str) -> bool {
    let normalized = content.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "<system>",
        "<developer>",
        "api_key",
        "authorization:",
        "password",
        "secret",
        "bearer",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn score(candidate: &MemoryContextCandidate) -> f32 {
    let recency = 1.0 / candidate.recency_rank as f32;
    (candidate.importance * 0.6 + candidate.confidence * 0.3 + recency * 0.1).clamp(0.0, 1.0)
}
