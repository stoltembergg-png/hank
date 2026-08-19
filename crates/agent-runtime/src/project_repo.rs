//! Implementação SQLite do ProjectRepository.
//!
//! Conforme PR-028 e regras de persistência segura e isolamento.

use agent_core::error::DomainError;
use agent_core::ids::ProjectId;
use agent_core::project::{Project, ProjectRepository, ProjectSettings, ProjectStatus};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashSet;
use std::str::FromStr;

/// Repositório SQLite para a entidade Project.
#[derive(Clone)]
pub struct SqliteProjectRepository {
    pool: Pool<Sqlite>,
}

impl SqliteProjectRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

impl ProjectRepository for SqliteProjectRepository {
    async fn save(&self, project: &Project) -> Result<(), DomainError> {
        let settings_json = serde_json::to_string(&project.settings).map_err(DomainError::Serialization)?;

        let status_str = match project.status {
            ProjectStatus::Active => "active",
            ProjectStatus::Paused => "paused",
            ProjectStatus::Archived => "archived",
        };

        let result = sqlx::query(
            "INSERT INTO projects (id, name, description, status, owner, created_at, updated_at, settings) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(project.id.to_string())
        .bind(&project.name)
        .bind(&project.description)
        .bind(status_str)
        .bind(&project.owner)
        .bind(project.created_at.to_rfc3339())
        .bind(project.updated_at.to_rfc3339())
        .bind(settings_json)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(DomainError::Duplicate(format!("projeto já existe: {}", project.id)))
            }
            Err(e) => Err(DomainError::InvariantViolation(format!("erro no banco: {}", e))),
        }
    }

    async fn get_by_id(&self, id: &ProjectId) -> Result<Option<Project>, DomainError> {
        let row = sqlx::query(
            "SELECT id, name, description, status, owner, created_at, updated_at, settings \
             FROM projects WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::InvariantViolation(format!("erro ao buscar projeto: {}", e)))?;

        match row {
            Some(r) => {
                let id_str: String = r.get("id");
                let name: String = r.get("name");
                let description: Option<String> = r.get("description");
                let status_str: String = r.get("status");
                let owner: String = r.get("owner");
                let created_at_str: String = r.get("created_at");
                let updated_at_str: String = r.get("updated_at");
                let settings_json: String = r.get("settings");

                let id = ProjectId::from_str(&id_str)
                    .map_err(|e| DomainError::Validation(format!("id inválido: {}", e)))?;

                let status = match status_str.as_str() {
                    "active" => ProjectStatus::Active,
                    "paused" => ProjectStatus::Paused,
                    "archived" => ProjectStatus::Archived,
                    other => return Err(DomainError::Validation(format!("status desconhecido: {}", other))),
                };

                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| DomainError::Validation(format!("data created_at inválida: {}", e)))?
                    .with_timezone(&Utc);

                let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| DomainError::Validation(format!("data updated_at inválida: {}", e)))?
                    .with_timezone(&Utc);

                let settings: ProjectSettings = serde_json::from_str(&settings_json)
                    .map_err(DomainError::Serialization)?;

                Ok(Some(Project {
                    id,
                    name,
                    description,
                    status,
                    owner,
                    created_at,
                    updated_at,
                    settings,
                    folders: Vec::new(),
                    repositories: Vec::new(),
                    agents: HashSet::new(),
                    skills: HashSet::new(),
                    workflows: HashSet::new(),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<Project>, DomainError> {
        let bounded_limit = limit.min(100) as i64;
        let offset_i64 = offset as i64;

        let rows = sqlx::query(
            "SELECT id, name, description, status, owner, created_at, updated_at, settings \
             FROM projects ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(bounded_limit)
        .bind(offset_i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InvariantViolation(format!("erro ao listar projetos: {}", e)))?;

        let mut projects = Vec::with_capacity(rows.len());
        for r in rows {
            let id_str: String = r.get("id");
            let name: String = r.get("name");
            let description: Option<String> = r.get("description");
            let status_str: String = r.get("status");
            let owner: String = r.get("owner");
            let created_at_str: String = r.get("created_at");
            let updated_at_str: String = r.get("updated_at");
            let settings_json: String = r.get("settings");

            let id = ProjectId::from_str(&id_str)
                .map_err(|e| DomainError::Validation(format!("id inválido: {}", e)))?;

            let status = match status_str.as_str() {
                "active" => ProjectStatus::Active,
                "paused" => ProjectStatus::Paused,
                "archived" => ProjectStatus::Archived,
                other => return Err(DomainError::Validation(format!("status desconhecido: {}", other))),
            };

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DomainError::Validation(format!("data created_at inválida: {}", e)))?
                .with_timezone(&Utc);

            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| DomainError::Validation(format!("data updated_at inválida: {}", e)))?
                .with_timezone(&Utc);

            let settings: ProjectSettings = serde_json::from_str(&settings_json)
                .map_err(DomainError::Serialization)?;

            projects.push(Project {
                id,
                name,
                description,
                status,
                owner,
                created_at,
                updated_at,
                settings,
                folders: Vec::new(),
                repositories: Vec::new(),
                agents: HashSet::new(),
                skills: HashSet::new(),
                workflows: HashSet::new(),
            });
        }

        Ok(projects)
    }

    async fn update(&self, project: &Project) -> Result<(), DomainError> {
        let settings_json = serde_json::to_string(&project.settings).map_err(DomainError::Serialization)?;

        let status_str = match project.status {
            ProjectStatus::Active => "active",
            ProjectStatus::Paused => "paused",
            ProjectStatus::Archived => "archived",
        };

        let result = sqlx::query(
            "UPDATE projects SET name = ?, description = ?, status = ?, owner = ?, updated_at = ?, settings = ? \
             WHERE id = ?",
        )
        .bind(&project.name)
        .bind(&project.description)
        .bind(status_str)
        .bind(&project.owner)
        .bind(project.updated_at.to_rfc3339())
        .bind(settings_json)
        .bind(project.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::InvariantViolation(format!("erro ao atualizar projeto: {}", e)))?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound(format!("projeto não encontrado: {}", project.id)))
        } else {
            Ok(())
        }
    }

    async fn delete(&self, id: &ProjectId) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::InvariantViolation(format!("erro ao deletar projeto: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use crate::sqlite::SqliteStorage;

    async fn setup_repo() -> SqliteProjectRepository {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        SqliteProjectRepository::new(storage.pool().clone())
    }

    #[tokio::test]
    async fn save_and_get_project_roundtrips() {
        let repo = setup_repo().await;
        let project = Project::create("Hank Dev", "gabriel", Some("Description".into())).unwrap();

        repo.save(&project).await.unwrap();

        let retrieved = repo.get_by_id(&project.id).await.unwrap().expect("projeto deve existir");
        assert_eq!(retrieved.id, project.id);
        assert_eq!(retrieved.name, "Hank Dev");
        assert_eq!(retrieved.owner, "gabriel");
        assert_eq!(retrieved.status, ProjectStatus::Active);
        assert_eq!(retrieved.description, Some("Description".into()));
    }

    #[tokio::test]
    async fn save_duplicate_project_fails() {
        let repo = setup_repo().await;
        let project = Project::create("Hank Dev", "gabriel", None).unwrap();

        repo.save(&project).await.unwrap();
        let err = repo.save(&project).await.unwrap_err();
        assert!(matches!(err, DomainError::Duplicate(_)));
    }

    #[tokio::test]
    async fn update_project_modifies_fields() {
        let repo = setup_repo().await;
        let mut project = Project::create("Hank Dev", "gabriel", None).unwrap();
        repo.save(&project).await.unwrap();

        project.name = "Hank Renamed".into();
        project.pause().unwrap();
        repo.update(&project).await.unwrap();

        let retrieved = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Hank Renamed");
        assert_eq!(retrieved.status, ProjectStatus::Paused);
    }

    #[tokio::test]
    async fn list_projects_with_pagination() {
        let repo = setup_repo().await;

        for i in 1..=5 {
            let project = Project::create(format!("Project {}", i), "gabriel", None).unwrap();
            repo.save(&project).await.unwrap();
        }

        let list_p1 = repo.list(3, 0).await.unwrap();
        assert_eq!(list_p1.len(), 3);

        let list_p2 = repo.list(3, 3).await.unwrap();
        assert_eq!(list_p2.len(), 2);
    }

    #[tokio::test]
    async fn delete_project_removes_record() {
        let repo = setup_repo().await;
        let project = Project::create("Hank To Delete", "gabriel", None).unwrap();
        repo.save(&project).await.unwrap();

        let deleted = repo.delete(&project.id).await.unwrap();
        assert!(deleted);

        let not_found = repo.get_by_id(&project.id).await.unwrap();
        assert!(not_found.is_none());

        let deleted_again = repo.delete(&project.id).await.unwrap();
        assert!(!deleted_again);
    }
}
