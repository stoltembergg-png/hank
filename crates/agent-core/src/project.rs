//! Entidade Project e invariantes de domínio.
//!
//! Conforme PR-027 e regras de fronteira arquitetural (AI-001, AI-003, D-001).
//! O Project atua como aggregate root e fronteira estrita de isolamento
//! para agentes, sessões, memória, skills e workflows.

use crate::error::DomainError;
use crate::ids::ProjectId;
use crate::policy::{AgentPolicyConfig, BudgetPolicy, InstructionHierarchy};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Limites de tamanho de campos para validação de integridade.
pub const MAX_PROJECT_NAME_LEN: usize = 128;
pub const MAX_PROJECT_DESCRIPTION_LEN: usize = 1024;
pub const MAX_PROJECT_OWNER_LEN: usize = 128;

/// Estado do ciclo de vida do projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Paused,
    Archived,
}

/// Configuração de configurações do projeto.
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

/// Diretório monitorado pertencente ao escopo do projeto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
}

/// Repositório de código vinculado ao escopo do projeto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRepository {
    pub id: String,
    pub name: String,
    pub url: String,
    pub branch: String,
    pub worktree_path: Option<String>,
    pub added_at: DateTime<Utc>,
}

/// Entidade Aggregate Root: Project.
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

impl Project {
    /// Cria um novo projeto ativo após validação estrita de invariants.
    pub fn create(
        name: impl Into<String>,
        owner: impl Into<String>,
        description: Option<String>,
    ) -> Result<Self, DomainError> {
        let name = name.into().trim().to_string();
        let owner = owner.into().trim().to_string();

        Self::validate_name(&name)?;
        Self::validate_owner(&owner)?;

        if let Some(ref desc) = description {
            Self::validate_description(desc)?;
        }

        let now = Utc::now();
        Ok(Self {
            id: ProjectId::new(),
            name,
            description,
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
        })
    }

    /// Construtor simplificado (retrocompatível com testes existentes).
    pub fn new(name: String, owner: String) -> Self {
        Self::create(name, owner, None).unwrap_or_else(|_| {
            let now = Utc::now();
            Self {
                id: ProjectId::new(),
                name: "default".into(),
                description: None,
                status: ProjectStatus::Active,
                owner: "owner".into(),
                created_at: now,
                updated_at: now,
                settings: ProjectSettings::default(),
                folders: Vec::new(),
                repositories: Vec::new(),
                agents: HashSet::new(),
                skills: HashSet::new(),
                workflows: HashSet::new(),
            }
        })
    }

    /// Valida o nome do projeto.
    fn validate_name(name: &str) -> Result<(), DomainError> {
        if name.is_empty() {
            return Err(DomainError::Validation(
                "o nome do projeto não pode ser vazio".into(),
            ));
        }
        if name.len() > MAX_PROJECT_NAME_LEN {
            return Err(DomainError::Validation(format!(
                "o nome do projeto excede o limite de {} caracteres",
                MAX_PROJECT_NAME_LEN
            )));
        }
        if name.chars().any(char::is_control) {
            return Err(DomainError::Validation(
                "o nome do projeto contém caracteres de controle inválidos".into(),
            ));
        }
        Ok(())
    }

    /// Valida o proprietário do projeto.
    fn validate_owner(owner: &str) -> Result<(), DomainError> {
        if owner.is_empty() {
            return Err(DomainError::Validation(
                "o proprietário do projeto não pode ser vazio".into(),
            ));
        }
        if owner.len() > MAX_PROJECT_OWNER_LEN {
            return Err(DomainError::Validation(format!(
                "o proprietário do projeto excede o limite de {} caracteres",
                MAX_PROJECT_OWNER_LEN
            )));
        }
        if owner.chars().any(char::is_control) {
            return Err(DomainError::Validation(
                "o proprietário contém caracteres de controle inválidos".into(),
            ));
        }
        Ok(())
    }

    /// Valida a descrição do projeto.
    fn validate_description(desc: &str) -> Result<(), DomainError> {
        if desc.len() > MAX_PROJECT_DESCRIPTION_LEN {
            return Err(DomainError::Validation(format!(
                "a descrição do projeto excede o limite de {} caracteres",
                MAX_PROJECT_DESCRIPTION_LEN
            )));
        }
        Ok(())
    }

    /// Transição de estado para Arquivado (terminal).
    pub fn archive(&mut self) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "archived".into(),
            });
        }
        self.status = ProjectStatus::Archived;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Transição de estado para Pausado.
    pub fn pause(&mut self) -> Result<(), DomainError> {
        match self.status {
            ProjectStatus::Active => {
                self.status = ProjectStatus::Paused;
                self.updated_at = Utc::now();
                Ok(())
            }
            ProjectStatus::Paused => Ok(()),
            ProjectStatus::Archived => Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "paused".into(),
            }),
        }
    }

    /// Retoma o projeto para Ativo.
    pub fn resume(&mut self) -> Result<(), DomainError> {
        match self.status {
            ProjectStatus::Paused => {
                self.status = ProjectStatus::Active;
                self.updated_at = Utc::now();
                Ok(())
            }
            ProjectStatus::Active => Ok(()),
            ProjectStatus::Archived => Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "active".into(),
            }),
        }
    }

    /// Vincula um agente ao projeto.
    pub fn add_agent(&mut self, agent_id: crate::ids::AgentId) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::Validation(
                "não é possível adicionar agentes a um projeto arquivado".into(),
            ));
        }
        self.agents.insert(agent_id);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove um agente vinculado ao projeto.
    pub fn remove_agent(&mut self, agent_id: &crate::ids::AgentId) -> bool {
        let removed = self.agents.remove(agent_id);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::AgentId;

    #[test]
    fn create_valid_project_succeeds() {
        let project =
            Project::create("Hank Workspace", "gabriel", Some("Dev workspace".into())).unwrap();
        assert_eq!(project.name, "Hank Workspace");
        assert_eq!(project.owner, "gabriel");
        assert_eq!(project.status, ProjectStatus::Active);
        assert_eq!(project.description, Some("Dev workspace".into()));
        assert!(project.id.to_string().starts_with("proj-"));
    }

    #[test]
    fn create_project_with_empty_or_whitespace_name_fails() {
        let err1 = Project::create("", "owner", None).unwrap_err();
        let err2 = Project::create("   ", "owner", None).unwrap_err();
        assert!(matches!(err1, DomainError::Validation(_)));
        assert!(matches!(err2, DomainError::Validation(_)));
    }

    #[test]
    fn create_project_with_overlong_name_fails() {
        let long_name = "a".repeat(MAX_PROJECT_NAME_LEN + 1);
        let err = Project::create(long_name, "owner", None).unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn create_project_with_empty_owner_fails() {
        let err = Project::create("Hank", "", None).unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn lifecycle_transitions_active_paused_active() {
        let mut project = Project::create("Hank", "gabriel", None).unwrap();
        assert_eq!(project.status, ProjectStatus::Active);

        project.pause().unwrap();
        assert_eq!(project.status, ProjectStatus::Paused);

        project.resume().unwrap();
        assert_eq!(project.status, ProjectStatus::Active);
    }

    #[test]
    fn archive_is_terminal_state() {
        let mut project = Project::create("Hank", "gabriel", None).unwrap();
        project.archive().unwrap();
        assert_eq!(project.status, ProjectStatus::Archived);

        // Não pode arquivar novamente
        assert!(project.archive().is_err());
        // Não pode pausar
        assert!(project.pause().is_err());
        // Não pode retomar
        assert!(project.resume().is_err());
        // Não pode adicionar agentes
        assert!(project.add_agent(AgentId::new()).is_err());
    }

    #[test]
    fn project_serde_roundtrip() {
        let project =
            Project::create("Hank Serde", "tester", Some("Testing serde".into())).unwrap();
        let json = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project.id, deserialized.id);
        assert_eq!(project.name, deserialized.name);
        assert_eq!(project.status, deserialized.status);
    }

    #[test]
    fn agent_association_and_removal() {
        let mut project = Project::create("Hank", "gabriel", None).unwrap();
        let agent = AgentId::new();

        assert!(!project.agents.contains(&agent));
        project.add_agent(agent).unwrap();
        assert!(project.agents.contains(&agent));

        assert!(project.remove_agent(&agent));
        assert!(!project.agents.contains(&agent));
        assert!(!project.remove_agent(&agent));
    }
}
