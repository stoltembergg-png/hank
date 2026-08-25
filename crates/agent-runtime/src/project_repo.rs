//! Implementação SQLite do ProjectRepository.
//!
//! Conforme PR-028, PR-033 e PR-034: regras de persistência segura, isolamento, folders e git repositories.

use agent_core::error::DomainError;
use agent_core::ids::ProjectId;
use agent_core::project::{
    Project, ProjectFolder, ProjectGitRepo, ProjectRepository, ProjectSettings, ProjectStatus,
};
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

    pub async fn count(&self) -> Result<usize, DomainError> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM projects")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                DomainError::InvariantViolation(format!("erro ao contar projetos: {e}"))
            })?;
        let count: i64 = row.get("count");
        usize::try_from(count)
            .map_err(|_| DomainError::InvariantViolation("contagem de projetos inválida".into()))
    }
}

impl ProjectRepository for SqliteProjectRepository {
    async fn save(&self, project: &Project) -> Result<(), DomainError> {
        let settings_json =
            serde_json::to_string(&project.settings).map_err(DomainError::Serialization)?;

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
            Ok(_) => {
                for folder in &project.folders {
                    self.add_folder(&project.id, folder).await?;
                }
                for repo in &project.repositories {
                    self.add_git_repo(&project.id, repo).await?;
                }
                Ok(())
            }
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Err(
                DomainError::Duplicate(format!("projeto já existe: {}", project.id)),
            ),
            Err(e) => Err(DomainError::InvariantViolation(format!(
                "erro no banco: {}",
                e
            ))),
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
                    other => {
                        return Err(DomainError::Validation(format!(
                            "status desconhecido: {}",
                            other
                        )))
                    }
                };

                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| {
                        DomainError::Validation(format!("data created_at inválida: {}", e))
                    })?
                    .with_timezone(&Utc);

                let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| {
                        DomainError::Validation(format!("data updated_at inválida: {}", e))
                    })?
                    .with_timezone(&Utc);

                let settings: ProjectSettings =
                    serde_json::from_str(&settings_json).map_err(DomainError::Serialization)?;

                let folders = self.list_folders(&id).await?;
                let repositories = self.list_git_repos(&id).await?;

                Ok(Some(Project {
                    id,
                    name,
                    description,
                    status,
                    owner,
                    created_at,
                    updated_at,
                    settings,
                    folders,
                    repositories,
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
                other => {
                    return Err(DomainError::Validation(format!(
                        "status desconhecido: {}",
                        other
                    )))
                }
            };

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DomainError::Validation(format!("data created_at inválida: {}", e)))?
                .with_timezone(&Utc);

            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| DomainError::Validation(format!("data updated_at inválida: {}", e)))?
                .with_timezone(&Utc);

            let settings: ProjectSettings =
                serde_json::from_str(&settings_json).map_err(DomainError::Serialization)?;

            let folders = self.list_folders(&id).await?;
            let repositories = self.list_git_repos(&id).await?;

            projects.push(Project {
                id,
                name,
                description,
                status,
                owner,
                created_at,
                updated_at,
                settings,
                folders,
                repositories,
                agents: HashSet::new(),
                skills: HashSet::new(),
                workflows: HashSet::new(),
            });
        }

        Ok(projects)
    }

    async fn update(&self, project: &Project) -> Result<(), DomainError> {
        let settings_json =
            serde_json::to_string(&project.settings).map_err(DomainError::Serialization)?;

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
            Err(DomainError::NotFound(format!(
                "projeto não encontrado: {}",
                project.id
            )))
        } else {
            Ok(())
        }
    }

    async fn delete(&self, id: &ProjectId) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::InvariantViolation(format!("erro ao deletar projeto: {}", e))
            })?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_settings(
        &self,
        project_id: &ProjectId,
        settings: &ProjectSettings,
    ) -> Result<(), DomainError> {
        let settings_json = serde_json::to_string(settings).map_err(DomainError::Serialization)?;
        let updated_at = Utc::now();

        let result = sqlx::query("UPDATE projects SET settings = ?, updated_at = ? WHERE id = ?")
            .bind(settings_json)
            .bind(updated_at.to_rfc3339())
            .bind(project_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::InvariantViolation(format!("erro ao atualizar configurações: {}", e))
            })?;

        if result.rows_affected() == 0 {
            Err(DomainError::NotFound(format!(
                "projeto não encontrado: {}",
                project_id
            )))
        } else {
            Ok(())
        }
    }

    async fn get_settings(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<ProjectSettings>, DomainError> {
        let row = sqlx::query("SELECT settings FROM projects WHERE id = ?")
            .bind(project_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                DomainError::InvariantViolation(format!("erro ao buscar configurações: {}", e))
            })?;

        match row {
            Some(r) => {
                let settings_json: String = r.get("settings");
                let settings: ProjectSettings =
                    serde_json::from_str(&settings_json).map_err(DomainError::Serialization)?;
                Ok(Some(settings))
            }
            None => Ok(None),
        }
    }

    async fn add_folder(
        &self,
        project_id: &ProjectId,
        folder: &ProjectFolder,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "INSERT INTO project_folders (id, project_id, name, path, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&folder.id)
        .bind(project_id.to_string())
        .bind(&folder.name)
        .bind(&folder.path)
        .bind(folder.created_at.to_rfc3339())
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(DomainError::Duplicate(format!(
                    "pasta já cadastrada neste projeto: {}",
                    folder.path
                )))
            }
            Err(e) => Err(DomainError::InvariantViolation(format!(
                "erro no banco: {}",
                e
            ))),
        }
    }

    async fn list_folders(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ProjectFolder>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, name, path, created_at FROM project_folders WHERE project_id = ? ORDER BY created_at ASC",
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::InvariantViolation(format!("erro ao listar pastas: {}", e)))?;

        let mut folders = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.get("id");
            let name: String = r.get("name");
            let path: String = r.get("path");
            let created_at_str: String = r.get("created_at");
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| DomainError::Validation(format!("data inválida: {}", e)))?
                .with_timezone(&Utc);

            folders.push(ProjectFolder {
                id,
                name,
                path,
                created_at,
            });
        }
        Ok(folders)
    }

    async fn remove_folder(
        &self,
        project_id: &ProjectId,
        folder_id: &str,
    ) -> Result<bool, DomainError> {
        let result = sqlx::query("DELETE FROM project_folders WHERE project_id = ? AND id = ?")
            .bind(project_id.to_string())
            .bind(folder_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                DomainError::InvariantViolation(format!("erro ao deletar pasta: {}", e))
            })?;

        Ok(result.rows_affected() > 0)
    }

    async fn add_git_repo(
        &self,
        project_id: &ProjectId,
        repo: &ProjectGitRepo,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "INSERT INTO project_repositories (id, project_id, name, url, branch, worktree_path, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&repo.id)
        .bind(project_id.to_string())
        .bind(&repo.name)
        .bind(&repo.url)
        .bind(&repo.branch)
        .bind(&repo.worktree_path)
        .bind(repo.added_at.to_rfc3339())
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(DomainError::Duplicate(format!(
                    "repositório já cadastrado neste projeto: {}",
                    repo.url
                )))
            }
            Err(e) => Err(DomainError::InvariantViolation(format!(
                "erro no banco: {}",
                e
            ))),
        }
    }

    async fn list_git_repos(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ProjectGitRepo>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, name, url, branch, worktree_path, added_at \
             FROM project_repositories WHERE project_id = ? ORDER BY added_at ASC",
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            DomainError::InvariantViolation(format!("erro ao listar repositórios: {}", e))
        })?;

        let mut repos = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.get("id");
            let name: String = r.get("name");
            let url: String = r.get("url");
            let branch: String = r.get("branch");
            let worktree_path: Option<String> = r.get("worktree_path");
            let added_at_str: String = r.get("added_at");
            let added_at = DateTime::parse_from_rfc3339(&added_at_str)
                .map_err(|e| DomainError::Validation(format!("data inválida: {}", e)))?
                .with_timezone(&Utc);

            repos.push(ProjectGitRepo {
                id,
                name,
                url,
                branch,
                worktree_path,
                added_at,
            });
        }
        Ok(repos)
    }

    async fn remove_git_repo(
        &self,
        project_id: &ProjectId,
        repo_id: &str,
    ) -> Result<bool, DomainError> {
        let result =
            sqlx::query("DELETE FROM project_repositories WHERE project_id = ? AND id = ?")
                .bind(project_id.to_string())
                .bind(repo_id)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    DomainError::InvariantViolation(format!("erro ao deletar repositório: {}", e))
                })?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use crate::sqlite::{SqliteStorage, SqliteStorageConfig};
    use tempfile::tempdir;

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

        let retrieved = repo
            .get_by_id(&project.id)
            .await
            .unwrap()
            .expect("projeto deve existir");
        assert_eq!(retrieved.id, project.id);
        assert_eq!(retrieved.name, "Hank Dev");
        assert_eq!(retrieved.owner, "gabriel");
        assert_eq!(retrieved.status, ProjectStatus::Active);
        assert_eq!(retrieved.description, Some("Description".into()));
    }

    #[tokio::test]
    // @spec:AC-114
    async fn project_survives_sqlite_reopen() {
        let directory = tempdir().unwrap();
        let database_path = directory.path().join("projects.db");
        let project = Project::create("Persistent Desktop", "gabriel", None).unwrap();

        {
            let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&database_path))
                .await
                .unwrap();
            run_migrations(storage.pool()).await.unwrap();
            let repo = SqliteProjectRepository::new(storage.pool().clone());
            repo.save(&project).await.unwrap();
            storage.close().await;
        }

        let storage = SqliteStorage::connect(SqliteStorageConfig::for_file(&database_path))
            .await
            .unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let repo = SqliteProjectRepository::new(storage.pool().clone());
        let reopened = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(reopened.name, "Persistent Desktop");
        assert_eq!(repo.count().await.unwrap(), 1);
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

    #[tokio::test]
    async fn folder_crud_and_persistence() {
        let repo = setup_repo().await;
        let project = Project::create("Hank With Folders", "gabriel", None).unwrap();
        repo.save(&project).await.unwrap();

        let folder1 = ProjectFolder::create("src", "C:/repos/hank/src").unwrap();
        let folder2 = ProjectFolder::create("docs", "C:/repos/hank/docs").unwrap();

        repo.add_folder(&project.id, &folder1).await.unwrap();
        repo.add_folder(&project.id, &folder2).await.unwrap();

        // Duplicata deve retornar DomainError::Duplicate
        let dup = ProjectFolder::create("src_dup", "C:/repos/hank/src").unwrap();
        let err = repo.add_folder(&project.id, &dup).await.unwrap_err();
        assert!(matches!(err, DomainError::Duplicate(_)));

        // Listagem
        let folders = repo.list_folders(&project.id).await.unwrap();
        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0].name, "src");
        assert_eq!(folders[1].name, "docs");

        // get_by_id carrega folders
        let loaded = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(loaded.folders.len(), 2);

        // Remoção
        let removed = repo.remove_folder(&project.id, &folder1.id).await.unwrap();
        assert!(removed);
        let folders_after = repo.list_folders(&project.id).await.unwrap();
        assert_eq!(folders_after.len(), 1);
    }

    #[tokio::test]
    async fn git_repo_crud_and_persistence() {
        let repo = setup_repo().await;
        let project = Project::create("Hank With Repos", "gabriel", None).unwrap();
        repo.save(&project).await.unwrap();

        let repo1 = ProjectGitRepo::create(
            "core-repo",
            "https://github.com/hank/core.git",
            "main",
            Some("C:/wt/core".into()),
        )
        .unwrap();

        let repo2 =
            ProjectGitRepo::create("ui-repo", "https://github.com/hank/ui.git", "develop", None)
                .unwrap();

        repo.add_git_repo(&project.id, &repo1).await.unwrap();
        repo.add_git_repo(&project.id, &repo2).await.unwrap();

        // Duplicata por URL deve falhar
        let dup =
            ProjectGitRepo::create("core-dup", "https://github.com/hank/core.git", "main", None)
                .unwrap();
        let err = repo.add_git_repo(&project.id, &dup).await.unwrap_err();
        assert!(matches!(err, DomainError::Duplicate(_)));

        // Listagem
        let repos = repo.list_git_repos(&project.id).await.unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "core-repo");
        assert_eq!(repos[1].name, "ui-repo");

        // get_by_id carrega repositories
        let loaded = repo.get_by_id(&project.id).await.unwrap().unwrap();
        assert_eq!(loaded.repositories.len(), 2);

        // Remoção
        let removed = repo.remove_git_repo(&project.id, &repo1.id).await.unwrap();
        assert!(removed);
        let repos_after = repo.list_git_repos(&project.id).await.unwrap();
        assert_eq!(repos_after.len(), 1);
    }

    #[tokio::test]
    async fn settings_update_and_get() {
        let repo = setup_repo().await;
        let project = Project::create("Hank Settings", "gabriel", None).unwrap();
        repo.save(&project).await.unwrap();

        let custom_settings = ProjectSettings {
            retention_days: 120,
            max_active_agents: 8,
            telemetry_enabled: true,
            ..ProjectSettings::default()
        };

        repo.update_settings(&project.id, &custom_settings)
            .await
            .unwrap();

        let retrieved = repo
            .get_settings(&project.id)
            .await
            .unwrap()
            .expect("settings devem existir");
        assert_eq!(retrieved.retention_days, 120);
        assert_eq!(retrieved.max_active_agents, 8);
        assert!(retrieved.telemetry_enabled);

        let non_existent = ProjectId::new();
        let not_found = repo.get_settings(&non_existent).await.unwrap();
        assert!(not_found.is_none());
    }
}
