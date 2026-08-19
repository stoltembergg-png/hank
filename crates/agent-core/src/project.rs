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
pub const MAX_FOLDER_NAME_LEN: usize = 128;
pub const MAX_FOLDER_PATH_LEN: usize = 1024;
pub const MAX_REPO_NAME_LEN: usize = 128;
pub const MAX_REPO_URL_LEN: usize = 1024;
pub const MAX_REPO_BRANCH_LEN: usize = 256;
pub const MAX_REPO_WORKTREE_LEN: usize = 1024;

/// Estado do ciclo de vida do projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Paused,
    Archived,
}

/// Configuração de configurações do projeto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSettings {
    pub default_budget: BudgetPolicy,
    pub default_agent_policy: AgentPolicyConfig,
    pub instruction_hierarchy: InstructionHierarchy,
    pub allowed_capabilities: crate::capability::CapabilitySet,
    pub retention_days: u32,
    pub auto_archive_idle_days: Option<u32>,
    pub telemetry_enabled: bool,
    pub max_active_agents: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            default_budget: BudgetPolicy::default(),
            default_agent_policy: AgentPolicyConfig::default(),
            instruction_hierarchy: InstructionHierarchy::default(),
            allowed_capabilities: crate::capability::CapabilitySet::new(),
            retention_days: 90,
            auto_archive_idle_days: None,
            telemetry_enabled: false,
            max_active_agents: 5,
        }
    }
}

impl ProjectSettings {
    /// Valida as restrições e limites das configurações do projeto.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.retention_days == 0 || self.retention_days > 3650 {
            return Err(DomainError::Validation(
                "retention_days deve estar no intervalo de 1 a 3650 dias".into(),
            ));
        }

        if let Some(idle) = self.auto_archive_idle_days {
            if idle == 0 || idle > 365 {
                return Err(DomainError::Validation(
                    "auto_archive_idle_days deve estar no intervalo de 1 a 365 dias".into(),
                ));
            }
        }

        if self.max_active_agents == 0 || self.max_active_agents > 50 {
            return Err(DomainError::Validation(
                "max_active_agents deve estar no intervalo de 1 a 50".into(),
            ));
        }

        Ok(())
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

impl ProjectFolder {
    /// Cria uma nova pasta validada para vinculação ao projeto.
    pub fn create(name: impl Into<String>, path: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into().trim().to_string();
        let path = path.into().trim().to_string();

        if name.is_empty() {
            return Err(DomainError::Validation(
                "nome da pasta não pode ser vazio".into(),
            ));
        }
        if name.len() > MAX_FOLDER_NAME_LEN {
            return Err(DomainError::Validation(format!(
                "nome da pasta excede limite de {} caracteres",
                MAX_FOLDER_NAME_LEN
            )));
        }
        if name.chars().any(|c| c.is_control()) {
            return Err(DomainError::Validation(
                "nome da pasta contém caracteres de controle".into(),
            ));
        }

        if path.is_empty() {
            return Err(DomainError::Validation(
                "caminho da pasta não pode ser vazio".into(),
            ));
        }
        if path.len() > MAX_FOLDER_PATH_LEN {
            return Err(DomainError::Validation(format!(
                "caminho da pasta excede limite de {} caracteres",
                MAX_FOLDER_PATH_LEN
            )));
        }
        if path.chars().any(|c| c.is_control()) {
            return Err(DomainError::Validation(
                "caminho da pasta contém caracteres de controle".into(),
            ));
        }
        if path.contains("..") {
            return Err(DomainError::Validation(
                "caminho da pasta não pode conter path traversal (..)".into(),
            ));
        }

        Ok(Self {
            id: format!("fld-{}", uuid::Uuid::new_v4()),
            name,
            path,
            created_at: Utc::now(),
        })
    }
}

/// Repositório de código vinculado ao escopo do projeto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectGitRepo {
    pub id: String,
    pub name: String,
    pub url: String,
    pub branch: String,
    pub worktree_path: Option<String>,
    pub added_at: DateTime<Utc>,
}

impl ProjectGitRepo {
    /// Cria e valida um registro de repositório Git vinculado ao projeto.
    pub fn create(
        name: impl Into<String>,
        url: impl Into<String>,
        branch: impl Into<String>,
        worktree_path: Option<String>,
    ) -> Result<Self, DomainError> {
        let name = name.into().trim().to_string();
        let url = url.into().trim().to_string();
        let branch = branch.into().trim().to_string();

        if name.is_empty() {
            return Err(DomainError::Validation(
                "nome do repositório não pode ser vazio".into(),
            ));
        }
        if name.len() > MAX_REPO_NAME_LEN {
            return Err(DomainError::Validation(format!(
                "nome do repositório excede limite de {} caracteres",
                MAX_REPO_NAME_LEN
            )));
        }
        if name.chars().any(|c| c.is_control()) {
            return Err(DomainError::Validation(
                "nome do repositório contém caracteres de controle".into(),
            ));
        }

        if url.is_empty() {
            return Err(DomainError::Validation(
                "url do repositório não pode ser vazia".into(),
            ));
        }
        if url.len() > MAX_REPO_URL_LEN {
            return Err(DomainError::Validation(format!(
                "url do repositório excede limite de {} caracteres",
                MAX_REPO_URL_LEN
            )));
        }
        if url.chars().any(|c| c.is_control()) {
            return Err(DomainError::Validation(
                "url do repositório contém caracteres de controle".into(),
            ));
        }
        if url.contains('@') && (url.starts_with("http://") || url.starts_with("https://")) {
            return Err(DomainError::Validation(
                "url do repositório não pode conter credenciais embutidas".into(),
            ));
        }

        if branch.is_empty() {
            return Err(DomainError::Validation(
                "branch padrão não pode ser vazia".into(),
            ));
        }
        if branch.len() > MAX_REPO_BRANCH_LEN {
            return Err(DomainError::Validation(format!(
                "branch excede limite de {} caracteres",
                MAX_REPO_BRANCH_LEN
            )));
        }
        if branch.chars().any(|c| c.is_control()) {
            return Err(DomainError::Validation(
                "branch contém caracteres de controle".into(),
            ));
        }

        let validated_worktree = if let Some(wt) = worktree_path {
            let trimmed = wt.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                if trimmed.len() > MAX_REPO_WORKTREE_LEN {
                    return Err(DomainError::Validation(format!(
                        "worktree_path excede limite de {} caracteres",
                        MAX_REPO_WORKTREE_LEN
                    )));
                }
                if trimmed.chars().any(|c| c.is_control()) {
                    return Err(DomainError::Validation(
                        "worktree_path contém caracteres de controle".into(),
                    ));
                }
                if trimmed.contains("..") {
                    return Err(DomainError::Validation(
                        "worktree_path não pode conter path traversal (..)".into(),
                    ));
                }
                Some(trimmed)
            }
        } else {
            None
        };

        Ok(Self {
            id: format!("repo-{}", uuid::Uuid::new_v4()),
            name,
            url,
            branch,
            worktree_path: validated_worktree,
            added_at: Utc::now(),
        })
    }
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
    pub repositories: Vec<ProjectGitRepo>,
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

    /// Adiciona uma pasta validada ao escopo do projeto.
    pub fn add_folder(&mut self, folder: ProjectFolder) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "folder_added".into(),
            });
        }

        if self
            .folders
            .iter()
            .any(|f| f.path == folder.path || f.id == folder.id)
        {
            return Err(DomainError::Duplicate(format!(
                "pasta já cadastrada neste projeto: {}",
                folder.path
            )));
        }

        self.folders.push(folder);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove uma pasta vinculada ao projeto.
    pub fn remove_folder(&mut self, folder_id: &str) -> bool {
        let initial_len = self.folders.len();
        self.folders.retain(|f| f.id != folder_id);
        let removed = self.folders.len() < initial_len;
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Adiciona um repositório Git validado ao escopo do projeto.
    pub fn add_repository(&mut self, repo: ProjectGitRepo) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "repository_added".into(),
            });
        }

        if self
            .repositories
            .iter()
            .any(|r| r.url == repo.url || r.id == repo.id)
        {
            return Err(DomainError::Duplicate(format!(
                "repositório já cadastrado neste projeto: {}",
                repo.url
            )));
        }

        self.repositories.push(repo);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove um repositório Git vinculado ao projeto.
    pub fn remove_repository(&mut self, repo_id: &str) -> bool {
        let initial_len = self.repositories.len();
        self.repositories.retain(|r| r.id != repo_id);
        let removed = self.repositories.len() < initial_len;
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Vincula um agente ao projeto se ainda não associado.
    pub fn add_agent(&mut self, agent_id: crate::ids::AgentId) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "agent_added".into(),
            });
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

    /// Atualiza as configurações do projeto após validação estrita.
    pub fn update_settings(&mut self, settings: ProjectSettings) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "settings_updated".into(),
            });
        }
        settings.validate()?;
        self.settings = settings;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// Port de persistência para o aggregate Project (DIP / Clean Architecture).
pub trait ProjectRepository: Send + Sync {
    /// Salva um novo projeto no armazenamento persistente.
    fn save(
        &self,
        project: &Project,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    /// Busca um projeto pelo ID tipado.
    fn get_by_id(
        &self,
        id: &ProjectId,
    ) -> impl std::future::Future<Output = Result<Option<Project>, DomainError>> + Send;

    /// Lista projetos paginados.
    fn list(
        &self,
        limit: usize,
        offset: usize,
    ) -> impl std::future::Future<Output = Result<Vec<Project>, DomainError>> + Send;

    /// Atualiza um projeto existente.
    fn update(
        &self,
        project: &Project,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    /// Remove um projeto pelo ID.
    fn delete(
        &self,
        id: &ProjectId,
    ) -> impl std::future::Future<Output = Result<bool, DomainError>> + Send;

    /// Atualiza exclusivamente as configurações de um projeto.
    fn update_settings(
        &self,
        project_id: &ProjectId,
        settings: &ProjectSettings,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    /// Busca exclusivamente as configurações de um projeto.
    fn get_settings(
        &self,
        project_id: &ProjectId,
    ) -> impl std::future::Future<Output = Result<Option<ProjectSettings>, DomainError>> + Send;

    /// Adiciona uma pasta ao escopo do projeto.
    fn add_folder(
        &self,
        project_id: &ProjectId,
        folder: &ProjectFolder,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    /// Lista pastas vinculadas a um projeto.
    fn list_folders(
        &self,
        project_id: &ProjectId,
    ) -> impl std::future::Future<Output = Result<Vec<ProjectFolder>, DomainError>> + Send;

    /// Remove uma pasta de um projeto.
    fn remove_folder(
        &self,
        project_id: &ProjectId,
        folder_id: &str,
    ) -> impl std::future::Future<Output = Result<bool, DomainError>> + Send;

    /// Adiciona um repositório Git ao escopo do projeto.
    fn add_git_repo(
        &self,
        project_id: &ProjectId,
        repo: &ProjectGitRepo,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send;

    /// Lista repositórios Git vinculados a um projeto.
    fn list_git_repos(
        &self,
        project_id: &ProjectId,
    ) -> impl std::future::Future<Output = Result<Vec<ProjectGitRepo>, DomainError>> + Send;

    /// Remove um repositório Git de um projeto.
    fn remove_git_repo(
        &self,
        project_id: &ProjectId,
        repo_id: &str,
    ) -> impl std::future::Future<Output = Result<bool, DomainError>> + Send;
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

    #[test]
    fn folder_creation_and_validation() {
        let valid = ProjectFolder::create("src", "C:/dev/src").unwrap();
        assert_eq!(valid.name, "src");
        assert_eq!(valid.path, "C:/dev/src");
        assert!(valid.id.starts_with("fld-"));

        assert!(ProjectFolder::create("", "C:/dev").is_err());
        assert!(ProjectFolder::create("src", "").is_err());
        assert!(ProjectFolder::create("src", "C:/dev/../secret").is_err());
    }

    #[test]
    fn folder_association_duplicate_and_removal() {
        let mut project = Project::create("Hank", "gabriel", None).unwrap();
        let folder1 = ProjectFolder::create("root", "C:/dev/root").unwrap();
        let folder_id = folder1.id.clone();

        project.add_folder(folder1).unwrap();
        assert_eq!(project.folders.len(), 1);

        // Duplicata por caminho deve falhar
        let folder_dup = ProjectFolder::create("root2", "C:/dev/root").unwrap();
        assert!(project.add_folder(folder_dup).is_err());

        // Remoção
        assert!(project.remove_folder(&folder_id));
        assert_eq!(project.folders.len(), 0);
        assert!(!project.remove_folder(&folder_id));
    }

    #[test]
    fn git_repo_creation_and_validation() {
        let valid = ProjectGitRepo::create(
            "hank-repo",
            "https://github.com/stoltembergg-png/hank.git",
            "main",
            Some("C:/worktrees/hank".into()),
        )
        .unwrap();
        assert_eq!(valid.name, "hank-repo");
        assert_eq!(valid.branch, "main");
        assert!(valid.id.starts_with("repo-"));

        // Nome vazio
        assert!(ProjectGitRepo::create("", "https://github.com/hank.git", "main", None).is_err());
        // URL com credenciais embutidas
        assert!(ProjectGitRepo::create(
            "repo",
            "https://user:pass@github.com/hank.git",
            "main",
            None
        )
        .is_err());
        // Worktree com traversal
        assert!(ProjectGitRepo::create(
            "repo",
            "https://github.com/hank.git",
            "main",
            Some("C:/wt/../secret".into())
        )
        .is_err());
    }

    #[test]
    fn git_repo_association_duplicate_and_removal() {
        let mut project = Project::create("Hank", "gabriel", None).unwrap();
        let repo1 =
            ProjectGitRepo::create("hank-repo", "https://github.com/hank.git", "main", None)
                .unwrap();
        let repo_id = repo1.id.clone();

        project.add_repository(repo1).unwrap();
        assert_eq!(project.repositories.len(), 1);

        // Duplicata por URL deve falhar
        let repo_dup =
            ProjectGitRepo::create("hank-repo2", "https://github.com/hank.git", "main", None)
                .unwrap();
        assert!(project.add_repository(repo_dup).is_err());

        // Remoção
        assert!(project.remove_repository(&repo_id));
        assert_eq!(project.repositories.len(), 0);
        assert!(!project.remove_repository(&repo_id));
    }

    #[test]
    fn settings_validation_and_update() {
        let mut project = Project::create("Hank", "gabriel", None).unwrap();
        let mut settings = ProjectSettings::default();
        settings.retention_days = 180;
        settings.max_active_agents = 10;
        settings.auto_archive_idle_days = Some(30);
        settings.telemetry_enabled = true;

        project.update_settings(settings.clone()).unwrap();
        assert_eq!(project.settings.retention_days, 180);
        assert_eq!(project.settings.max_active_agents, 10);
        assert_eq!(project.settings.auto_archive_idle_days, Some(30));
        assert!(project.settings.telemetry_enabled);

        // Limites inválidos
        let mut invalid_retention = settings.clone();
        invalid_retention.retention_days = 0;
        assert!(project.update_settings(invalid_retention).is_err());

        let mut invalid_agents = settings.clone();
        invalid_agents.max_active_agents = 51;
        assert!(project.update_settings(invalid_agents).is_err());

        let mut invalid_idle = settings;
        invalid_idle.auto_archive_idle_days = Some(366);
        assert!(project.update_settings(invalid_idle).is_err());
    }
}
