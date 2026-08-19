//! Entidades Memory e políticas de memória de domínio.

use crate::ids::{AgentId, MemoryId, ProjectId, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tipo de memória
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Working,
    ShortTerm,
    LongTerm,
    Episodic,
    Semantic,
    Procedural,
}

/// Status da memória
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Candidate,
    Approved,
    Rejected,
    Archived,
}

/// Memória de domínio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub project_id: ProjectId,
    pub agent_id: Option<AgentId>,
    pub session_id: Option<SessionId>,
    pub memory_type: MemoryType,
    pub status: MemoryStatus,
    pub content: String,
    pub summary: Option<String>,
    pub importance: f32,
    pub tags: Vec<String>,
    pub provenance: MemoryProvenance,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accessed_at: Option<DateTime<Utc>>,
    pub access_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub source: ProvenanceSource,
    pub extractor: Option<String>,
    pub confidence: f32,
    pub original_context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceSource {
    UserInput,
    AgentOutput,
    ToolResult,
    SkillExecution,
    WorkflowNode,
    ExternalImport,
    Inferred,
}

impl Memory {
    pub fn new_candidate(
        project_id: ProjectId,
        content: String,
        memory_type: MemoryType,
        provenance: MemoryProvenance,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryId::new(),
            project_id,
            agent_id: None,
            session_id: None,
            memory_type,
            status: MemoryStatus::Candidate,
            content,
            summary: None,
            importance: 0.5,
            tags: Vec::new(),
            provenance,
            created_at: now,
            updated_at: now,
            accessed_at: None,
            access_count: 0,
        }
    }

    pub fn approve(&mut self, importance: f32, summary: Option<String>) {
        self.status = MemoryStatus::Approved;
        self.importance = importance.clamp(0.0, 1.0);
        self.summary = summary;
        self.updated_at = Utc::now();
    }

    pub fn reject(&mut self) {
        self.status = MemoryStatus::Rejected;
        self.updated_at = Utc::now();
    }

    pub fn access(&mut self) {
        self.accessed_at = Some(Utc::now());
        self.access_count += 1;
    }
}

/// Política de memória por agente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPolicy {
    pub max_working_memories: usize,
    pub max_short_term_memories: usize,
    pub max_long_term_memories: usize,
    pub importance_threshold: f32,
    pub retention_days: u32,
    pub auto_archive: bool,
}

impl Default for MemoryPolicy {
    fn default() -> Self {
        Self {
            max_working_memories: 10,
            max_short_term_memories: 100,
            max_long_term_memories: 10000,
            importance_threshold: 0.3,
            retention_days: 90,
            auto_archive: true,
        }
    }
}
