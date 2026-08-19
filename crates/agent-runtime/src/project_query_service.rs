//! Serviço de consulta (query) e listagem paginada para Projetos.
//!
//! Conforme PR-030 e princípios de isolamento e fronteira da Application API.

use agent_core::error::DomainError;
use agent_core::ids::ProjectId;
use agent_core::project::{Project, ProjectRepository, ProjectStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const DEFAULT_PAGE_LIMIT: usize = 20;
pub const MAX_PAGE_LIMIT: usize = 100;

/// DTO de entrada para listagem paginada de projetos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsInput {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status_filter: Option<ProjectStatus>,
    pub correlation_id: Option<String>,
}

impl Default for ListProjectsInput {
    fn default() -> Self {
        Self {
            limit: Some(DEFAULT_PAGE_LIMIT),
            offset: Some(0),
            status_filter: None,
            correlation_id: None,
        }
    }
}

/// DTO resumido para exibição em listas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub agent_count: usize,
}

impl From<&Project> for ProjectSummary {
    fn from(p: &Project) -> Self {
        Self {
            id: p.id,
            name: p.name.clone(),
            description: p.description.clone(),
            status: p.status,
            owner: p.owner.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
            agent_count: p.agents.len(),
        }
    }
}

/// DTO de saída com lista paginada de projetos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsOutput {
    pub items: Vec<ProjectSummary>,
    pub limit: usize,
    pub offset: usize,
    pub correlation_id: Option<String>,
}

/// Serviço de aplicação para leitura e consulta de projetos.
pub struct ListProjectsService<R: ProjectRepository> {
    repository: Arc<R>,
}

impl<R: ProjectRepository> ListProjectsService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Retorna lista paginada de projetos com limites estritos (max 100 por página).
    pub async fn list(&self, input: ListProjectsInput) -> Result<ListProjectsOutput, DomainError> {
        let limit = input
            .limit
            .unwrap_or(DEFAULT_PAGE_LIMIT)
            .clamp(1, MAX_PAGE_LIMIT);
        let offset = input.offset.unwrap_or(0);

        let projects = self.repository.list(limit, offset).await?;

        let filtered_items: Vec<ProjectSummary> = projects
            .into_iter()
            .filter(|p| {
                if let Some(status) = input.status_filter {
                    p.status == status
                } else {
                    true
                }
            })
            .map(|p| ProjectSummary::from(&p))
            .collect();

        Ok(ListProjectsOutput {
            items: filtered_items,
            limit,
            offset,
            correlation_id: input.correlation_id,
        })
    }

    /// Busca um projeto detalhado por ID.
    pub async fn get_by_id(&self, id: &ProjectId) -> Result<Option<Project>, DomainError> {
        self.repository.get_by_id(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use crate::project_repo::SqliteProjectRepository;
    use crate::sqlite::SqliteStorage;

    async fn setup_service() -> ListProjectsService<SqliteProjectRepository> {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        ListProjectsService::new(repo)
    }

    #[tokio::test]
    async fn list_empty_returns_empty_page() {
        let service = setup_service().await;
        let output = service.list(ListProjectsInput::default()).await.unwrap();
        assert!(output.items.is_empty());
        assert_eq!(output.limit, DEFAULT_PAGE_LIMIT);
        assert_eq!(output.offset, 0);
    }

    #[tokio::test]
    async fn list_paginated_projects() {
        let service = setup_service().await;

        for i in 1..=5 {
            let p = Project::create(format!("Project {}", i), "gabriel", None).unwrap();
            service.repository.save(&p).await.unwrap();
        }

        let p1 = service
            .list(ListProjectsInput {
                limit: Some(2),
                offset: Some(0),
                status_filter: None,
                correlation_id: Some("corr-1".into()),
            })
            .await
            .unwrap();

        assert_eq!(p1.items.len(), 2);
        assert_eq!(p1.correlation_id.as_deref(), Some("corr-1"));

        let p2 = service
            .list(ListProjectsInput {
                limit: Some(2),
                offset: Some(2),
                status_filter: None,
                correlation_id: None,
            })
            .await
            .unwrap();

        assert_eq!(p2.items.len(), 2);
    }

    #[tokio::test]
    async fn get_by_id_returns_exact_project() {
        let service = setup_service().await;
        let project =
            Project::create("Target Project", "gabriel", Some("Detailed desc".into())).unwrap();
        service.repository.save(&project).await.unwrap();

        let found = service
            .get_by_id(&project.id)
            .await
            .unwrap()
            .expect("projeto deve existir");
        assert_eq!(found.id, project.id);
        assert_eq!(found.name, "Target Project");
        assert_eq!(found.description, Some("Detailed desc".into()));

        let not_found = service.get_by_id(&ProjectId::new()).await.unwrap();
        assert!(not_found.is_none());
    }
}
