//! Entidade Agent e configuração de domínio.

use crate::ids::{AgentId, ProjectId, SkillId};
use crate::policy::AgentPolicyConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Estado do agente
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Inactive,
    Suspended,
}

/// Personalidade do agente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub name: String,
    pub description: Option<String>,
    pub traits: Vec<String>,
    pub communication_style: CommunicationStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationStyle {
    Formal,
    Casual,
    Technical,
    Concise,
    Verbose,
}

impl Default for Personality {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            description: None,
            traits: vec!["helpful".into(), "accurate".into()],
            communication_style: CommunicationStyle::Technical,
        }
    }
}

/// Agente de domínio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: AgentStatus,
    pub personality: Personality,
    pub policy: AgentPolicyConfig,
    pub skills: HashSet<SkillId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Agent {
    pub fn new(project_id: ProjectId, name: String, policy: AgentPolicyConfig) -> Self {
        let now = Utc::now();
        Self {
            id: AgentId::new(),
            project_id,
            name,
            description: None,
            status: AgentStatus::Active,
            personality: Personality::default(),
            policy,
            skills: HashSet::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn activate(&mut self) -> Result<(), crate::error::DomainError> {
        if self.status == AgentStatus::Active {
            return Err(crate::error::DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Active".into(),
            });
        }
        self.status = AgentStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn suspend(&mut self, _reason: String) {
        self.status = AgentStatus::Suspended;
        self.updated_at = Utc::now();
    }

    pub fn add_skill(&mut self, skill_id: SkillId) {
        self.skills.insert(skill_id);
        self.updated_at = Utc::now();
    }

    pub fn remove_skill(&mut self, skill_id: &SkillId) -> bool {
        let removed = self.skills.remove(skill_id);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }
}
