#![allow(clippy::manual_async_fn)]
use agent_core::agent::{Agent, AgentStatus};
use agent_core::error::DomainError;
use agent_core::ids::{AgentId, ProjectId};
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;

#[derive(Clone)]
pub struct SqliteAgentRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAgentRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn save(&self, agent: &Agent) -> Result<(), DomainError> {
        agent.validate()?;
        let result = sqlx::query(
            "INSERT INTO agents (id, project_id, name, description, status, personality, policy, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(agent.id.to_string())
        .bind(agent.project_id.to_string())
        .bind(&agent.name)
        .bind(&agent.description)
        .bind(status_name(agent.status))
        .bind(serde_json::to_string(&agent.personality).map_err(DomainError::Serialization)?)
        .bind(serde_json::to_string(&agent.policy).map_err(DomainError::Serialization)?)
        .bind(agent.created_at.to_rfc3339())
        .bind(agent.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                Err(DomainError::Duplicate("agent already exists".into()))
            }
            Err(error) => Err(DomainError::InvariantViolation(format!(
                "agent persistence failed: {error}"
            ))),
        }
    }

    pub async fn get(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
    ) -> Result<Option<Agent>, DomainError> {
        let row = sqlx::query("SELECT id, project_id, name, description, status, personality, policy, created_at, updated_at FROM agents WHERE project_id = ? AND id = ?")
            .bind(project_id.to_string()).bind(agent_id.to_string()).fetch_optional(&self.pool).await
            .map_err(|error| DomainError::InvariantViolation(format!("agent query failed: {error}")))?;
        row.map(decode_agent).transpose()
    }

    pub async fn list(
        &self,
        project_id: &ProjectId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Agent>, DomainError> {
        let rows = sqlx::query("SELECT id, project_id, name, description, status, personality, policy, created_at, updated_at FROM agents WHERE project_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(project_id.to_string()).bind(limit.min(100) as i64).bind(offset as i64).fetch_all(&self.pool).await
            .map_err(|error| DomainError::InvariantViolation(format!("agent list failed: {error}")))?;
        rows.into_iter().map(decode_agent).collect()
    }

    pub async fn update(&self, agent: &Agent) -> Result<(), DomainError> {
        agent.validate()?;
        let result = sqlx::query("UPDATE agents SET name = ?, description = ?, status = ?, personality = ?, policy = ?, updated_at = ? WHERE project_id = ? AND id = ?")
            .bind(&agent.name).bind(&agent.description).bind(status_name(agent.status))
            .bind(serde_json::to_string(&agent.personality).map_err(DomainError::Serialization)?)
            .bind(serde_json::to_string(&agent.policy).map_err(DomainError::Serialization)?)
            .bind(agent.updated_at.to_rfc3339()).bind(agent.project_id.to_string()).bind(agent.id.to_string())
            .execute(&self.pool).await.map_err(|error| DomainError::InvariantViolation(format!("agent update failed: {error}")))?;
        if result.rows_affected() == 0 {
            Err(DomainError::NotFound("agent not found in project".into()))
        } else {
            Ok(())
        }
    }
}

impl agent_core::AgentRepository for SqliteAgentRepository {
    #[allow(clippy::manual_async_fn)]
    fn save(
        &self,
        agent: &Agent,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
        async move { self.save(agent).await }
    }

    #[allow(clippy::manual_async_fn)]
    fn get(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
    ) -> impl std::future::Future<Output = Result<Option<Agent>, DomainError>> + Send {
        async move { self.get(project_id, agent_id).await }
    }

    #[allow(clippy::manual_async_fn)]
    fn list(
        &self,
        project_id: &ProjectId,
        limit: usize,
        offset: usize,
    ) -> impl std::future::Future<Output = Result<Vec<Agent>, DomainError>> + Send {
        async move { self.list(project_id, limit, offset).await }
    }

    #[allow(clippy::manual_async_fn)]
    fn update(
        &self,
        agent: &Agent,
    ) -> impl std::future::Future<Output = Result<(), DomainError>> + Send {
        async move { self.update(agent).await }
    }
}

fn status_name(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Active => "active",
        AgentStatus::Inactive => "inactive",
        AgentStatus::Suspended => "suspended",
    }
}

fn sqlx_error(error: sqlx::Error) -> DomainError {
    DomainError::InvariantViolation(format!("agent row decode failed: {error}"))
}

fn decode_agent(row: sqlx::sqlite::SqliteRow) -> Result<Agent, DomainError> {
    let status: String = row.try_get("status").map_err(sqlx_error)?;
    let status = match status.as_str() {
        "active" => AgentStatus::Active,
        "inactive" => AgentStatus::Inactive,
        "suspended" => AgentStatus::Suspended,
        _ => return Err(DomainError::Validation("unknown agent status".into())),
    };
    Ok(Agent {
        id: AgentId::from_str(&row.try_get::<String, _>("id").map_err(sqlx_error)?)
            .map_err(|_| DomainError::Validation("invalid agent id".into()))?,
        project_id: ProjectId::from_str(
            &row.try_get::<String, _>("project_id").map_err(sqlx_error)?,
        )
        .map_err(|_| DomainError::Validation("invalid project id".into()))?,
        name: row.try_get("name").map_err(sqlx_error)?,
        description: row.try_get("description").map_err(sqlx_error)?,
        status,
        personality: serde_json::from_str(
            &row.try_get::<String, _>("personality")
                .map_err(sqlx_error)?,
        )
        .map_err(DomainError::Serialization)?,
        policy: serde_json::from_str(&row.try_get::<String, _>("policy").map_err(sqlx_error)?)
            .map_err(DomainError::Serialization)?,
        skills: Default::default(),
        created_at: chrono::DateTime::parse_from_rfc3339(
            &row.try_get::<String, _>("created_at").map_err(sqlx_error)?,
        )
        .map_err(|_| DomainError::Validation("invalid created_at".into()))?
        .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(
            &row.try_get::<String, _>("updated_at").map_err(sqlx_error)?,
        )
        .map_err(|_| DomainError::Validation("invalid updated_at".into()))?
        .with_timezone(&chrono::Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{migrations::run_migrations, sqlite::SqliteStorage};

    async fn repository() -> (SqliteAgentRepository, ProjectId) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let project_id = ProjectId::new();
        sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Project', 'active', 'owner', '2026-01-01', '2026-01-01', '{}')")
            .bind(project_id.to_string()).execute(storage.pool()).await.unwrap();
        (
            SqliteAgentRepository::new(storage.pool().clone()),
            project_id,
        )
    }

    #[tokio::test]
    async fn agent_roundtrip_and_project_scoped_lookup() {
        let (repo, project_id) = repository().await;
        let agent = Agent::new(project_id, "worker".into(), Default::default());
        repo.save(&agent).await.unwrap();
        assert_eq!(
            repo.get(&project_id, &agent.id).await.unwrap().unwrap().id,
            agent.id
        );
        assert_eq!(repo.list(&project_id, 10, 0).await.unwrap().len(), 1);
        assert!(repo
            .get(&ProjectId::new(), &agent.id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn duplicate_agent_id_is_rejected() {
        let (repo, project_id) = repository().await;
        let agent = Agent::new(project_id, "worker".into(), Default::default());
        repo.save(&agent).await.unwrap();
        assert!(matches!(
            repo.save(&agent).await,
            Err(DomainError::Duplicate(_))
        ));
    }
}
