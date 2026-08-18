//! Entidade Project e invariantes de domínio.

use crate::ids::ProjectId;
use crate::policy::{AgentPolicyConfig, BudgetPolicy, InstructionHierarchy};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Estado do projeto
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Archived,
    Paused,
}

/// Configuração de um projeto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: ProjectSettings,
    pub folders: Vec<ProjectFolder>,
    pub repositories: Vec<ProjectRepository>,
    pub agents: HashSet<crate::ids::AgentId>,
    pub skills: HashSet<crate::ids::SkillId>,
    pub workflows: HashSet<crate::ids::WorkflowId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub default_budget: BudgetPolicy,
    pub default_agent_policy: AgentPolicyConfig,
    pub instruction_hierarchy: InstructionHierarchy,
    pub allowed_capabilities: crate::capability::CapabilitySet,
    pub retention_days: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            default_budget: BudgetPolicy::default(),
            default_agent_policy: AgentPolicyConfig::default(),
            instruction_hierarchy: InstructionHierarchy::default(),
            allowed_capabilities: crate::capability::CapabilitySet::new(),
            retention_days: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRepository {
    pub id: String,
    pub name: String,
    pub url: String,
    pub branch: String,
    pub worktree_path: Option<String>,
    pub added_at: DateTime<Utc>,
}

impl Project {
    pub fn new(name: String, owner: String) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            name,
            description: None,
            status: ProjectStatus::Active,
            owner,
            created_at: now,
            updated_at: now,
            settings: ProjectSettings::default(),
            folders: Vec::new(),
            repositories: Vec::new(),
            agents: HashSet::new(),
            skills: HashSet::new(),
            workflows: HashSet::new(),
        }
    }

    pub fn archive(&mut self) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Archived".into(),
            });
        }
        self.status = ProjectStatus::Archived;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn add_agent(&mut self, agent_id: crate::ids::AgentId) {
        self.agents.insert(agent_id);
        self.updated_at = Utc::now();
    }

    pub fn remove_agent(&mut self, agent_id: &crate::ids::AgentId) -> bool {
        let removed = self.agents.remove(agent_id);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Duplicate entity: {0}")]
    Duplicate(String),
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
}
