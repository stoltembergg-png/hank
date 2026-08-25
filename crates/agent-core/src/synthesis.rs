//! Bounded, provenance-aware synthesis without provider or persistence side effects.

use crate::ProjectId;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthesisSourceKind {
    Fact,
    Proposal,
    Instruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthesisMode {
    DeterministicFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SynthesisReason {
    #[error("result denied by policy")]
    DeniedByPolicy,
    #[error("result belongs to another project")]
    WrongProject,
    #[error("duplicate result")]
    Duplicate,
    #[error("result exceeds synthesis budget")]
    BudgetExceeded,
}

#[derive(Debug, Clone)]
pub struct SynthesisItem {
    pub result_id: uuid::Uuid,
    pub agent_id: uuid::Uuid,
    pub project_id: ProjectId,
    pub content: String,
    pub kind: SynthesisSourceKind,
    pub allowed: bool,
    pub denial: Option<SynthesisReason>,
}

impl SynthesisItem {
    pub fn accepted(
        result_id: uuid::Uuid,
        agent_id: uuid::Uuid,
        content: String,
        kind: SynthesisSourceKind,
    ) -> Self {
        Self {
            result_id,
            agent_id,
            project_id: ProjectId::new(),
            content,
            kind,
            allowed: true,
            denial: None,
        }
    }
    pub fn deny(&mut self, reason: SynthesisReason) {
        self.allowed = false;
        self.denial = Some(reason);
    }
}

#[derive(Debug, Clone)]
pub struct SynthesisInput {
    pub items: Vec<SynthesisItem>,
    pub max_output_bytes: usize,
}

impl SynthesisInput {
    pub fn new(items: Vec<SynthesisItem>) -> Self {
        Self {
            items,
            max_output_bytes: 4096,
        }
    }
    pub fn set_budget(&mut self, max_output_bytes: usize) {
        self.max_output_bytes = max_output_bytes;
    }
}

#[derive(Debug, Clone)]
pub struct SynthesisTrace {
    pub mode: SynthesisMode,
    pub included: Vec<uuid::Uuid>,
    pub excluded: Vec<(uuid::Uuid, SynthesisReason)>,
    pub closed: bool,
}

#[derive(Debug, Clone)]
pub struct SynthesisOutput {
    pub text: String,
    pub conflicts: Vec<Vec<uuid::Uuid>>,
    pub trace: SynthesisTrace,
}

#[derive(Debug, Clone)]
pub enum SynthesisOutcome {
    Completed(SynthesisOutput),
    Cancelled(SynthesisTrace),
    Failed(SynthesisTrace),
}

impl SynthesisOutcome {
    pub fn completed(self) -> Option<SynthesisOutput> {
        match self {
            Self::Completed(output) => Some(output),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SynthesisPolicy {
    project_id: ProjectId,
    group_id: uuid::Uuid,
    session_id: uuid::Uuid,
    max_output_bytes: usize,
}

impl SynthesisPolicy {
    pub fn new(
        project_id: uuid::Uuid,
        group_id: uuid::Uuid,
        session_id: uuid::Uuid,
        max_output_bytes: usize,
    ) -> Result<Self, SynthesisReason> {
        if max_output_bytes == 0 {
            return Err(SynthesisReason::BudgetExceeded);
        }
        Ok(Self {
            project_id: project_id.into(),
            group_id,
            session_id,
            max_output_bytes,
        })
    }
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn group_id(&self) -> uuid::Uuid {
        self.group_id
    }

    pub fn session_id(&self) -> uuid::Uuid {
        self.session_id
    }

    pub fn synthesize(&self, input: SynthesisInput) -> SynthesisOutcome {
        let limit = input.max_output_bytes.min(self.max_output_bytes);
        let mut seen = HashSet::new();
        let mut included = Vec::new();
        let mut excluded = Vec::new();
        let mut lines = Vec::new();
        let mut conflicts = Vec::new();
        let mut fact_ids = Vec::new();
        for mut item in input.items {
            let reason = if !item.allowed {
                item.denial.unwrap_or(SynthesisReason::DeniedByPolicy)
            } else if item.project_id != self.project_id {
                SynthesisReason::WrongProject
            } else if !seen.insert(item.result_id) {
                SynthesisReason::Duplicate
            } else {
                fact_ids.push(item.result_id);
                included.push(item.result_id);
                let kind = match item.kind {
                    SynthesisSourceKind::Fact => "fact",
                    SynthesisSourceKind::Proposal => "proposal",
                    SynthesisSourceKind::Instruction => "data",
                };
                lines.push(format!(
                    "[{kind}][source:{}] {}",
                    item.result_id, item.content
                ));
                continue;
            };
            item.deny(reason);
            excluded.push((item.result_id, reason));
        }
        if fact_ids.len() > 1 {
            conflicts.push(fact_ids);
        }
        let mut text = lines.join("\n");
        if text.len() > limit {
            text.truncate(limit);
        }
        SynthesisOutcome::Completed(SynthesisOutput {
            text,
            conflicts,
            trace: SynthesisTrace {
                mode: SynthesisMode::DeterministicFallback,
                included,
                excluded,
                closed: true,
            },
        })
    }
}
