//! SQLite persistence boundary for project-scoped AgentGroup entities.

use agent_core::{AgentGroup, AgentGroupLifecycle, DomainError, ProjectId};
use sqlx::{Pool, Row, Sqlite};

#[derive(Debug, Clone)]
pub struct AgentGroupRecord {
    pub group: AgentGroup,
    pub revision: u64,
}

#[derive(Clone)]
pub struct SqliteAgentGroupRepository {
    pool: Pool<Sqlite>,
}

impl SqliteAgentGroupRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, group: &AgentGroup) -> Result<AgentGroupRecord, DomainError> {
        group.domain_error()?;
        let json = serde_json::to_string(group)?;
        let result = sqlx::query(
            "INSERT INTO agent_groups (group_id, project_id, group_json, lifecycle, revision, created_at, updated_at) VALUES (?, ?, ?, ?, 1, datetime('now'), datetime('now'))",
        )
        .bind(group.id.to_string())
        .bind(group.project_id.to_string())
        .bind(json)
        .bind(lifecycle_to_db(group.lifecycle))
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(AgentGroupRecord {
                group: group.clone(),
                revision: 1,
            }),
            Err(error) if error.to_string().contains("UNIQUE") => Err(DomainError::Duplicate(
                "agent group already exists in project".into(),
            )),
            Err(error) => Err(DomainError::Validation(format!(
                "agent group persistence failed: {error}"
            ))),
        }
    }

    pub async fn get(
        &self,
        project_id: ProjectId,
        group_id: uuid::Uuid,
    ) -> Result<Option<AgentGroupRecord>, DomainError> {
        let row = sqlx::query(
            "SELECT group_json, revision FROM agent_groups WHERE project_id = ? AND group_id = ?",
        )
        .bind(project_id.to_string())
        .bind(group_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| DomainError::Validation(format!("agent group lookup failed: {error}")))?;
        row.map(decode).transpose()
    }

    pub async fn archive(
        &self,
        project_id: ProjectId,
        group_id: uuid::Uuid,
        expected_revision: u64,
    ) -> Result<AgentGroupRecord, DomainError> {
        let current = self
            .get(project_id, group_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("agent group".into()))?;
        if current.revision != expected_revision {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_revision.to_string(),
                actual: current.revision.to_string(),
            });
        }
        if current.group.lifecycle == AgentGroupLifecycle::Archived {
            return Ok(current);
        }
        let result = sqlx::query("UPDATE agent_groups SET group_json = ?, lifecycle = 'archived', revision = revision + 1, updated_at = datetime('now') WHERE project_id = ? AND group_id = ? AND revision = ?")
            .bind(serde_json::to_string(&archived(&current.group))?).bind(project_id.to_string()).bind(group_id.to_string()).bind(expected_revision as i64).execute(&self.pool).await
            .map_err(|error| DomainError::Validation(format!("agent group archive failed: {error}")))?;
        if result.rows_affected() != 1 {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_revision.to_string(),
                actual: "changed".into(),
            });
        }
        let mut value = archived(&current.group);
        value.lifecycle = AgentGroupLifecycle::Archived;
        Ok(AgentGroupRecord {
            group: value,
            revision: expected_revision + 1,
        })
    }
}

fn archived(group: &AgentGroup) -> AgentGroup {
    let mut value = group.clone();
    value.lifecycle = AgentGroupLifecycle::Archived;
    value
}
fn lifecycle_to_db(value: AgentGroupLifecycle) -> &'static str {
    match value {
        AgentGroupLifecycle::Draft => "draft",
        AgentGroupLifecycle::Active => "active",
        AgentGroupLifecycle::Archived => "archived",
    }
}
fn decode(row: sqlx::sqlite::SqliteRow) -> Result<AgentGroupRecord, DomainError> {
    let json: String = row
        .try_get("group_json")
        .map_err(|error| DomainError::Validation(format!("agent group decode failed: {error}")))?;
    let revision: i64 = row.try_get("revision").map_err(|error| {
        DomainError::Validation(format!("agent group revision failed: {error}"))
    })?;
    Ok(AgentGroupRecord {
        group: serde_json::from_str(&json)?,
        revision: revision as u64,
    })
}
