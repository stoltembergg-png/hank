#![allow(clippy::manual_async_fn)]

use agent_core::{DomainError, Memory, MemoryId, MemoryStatus, ProjectId};
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;

#[derive(Clone)]
pub struct SqliteMemoryRepository {
    pool: Pool<Sqlite>,
}

impl SqliteMemoryRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, memory: &Memory) -> Result<(), DomainError> {
        memory
            .validate()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        let result = sqlx::query(
            "INSERT INTO memories (id, project_id, agent_id, session_id, memory_type, status, content, summary, importance, tags, provenance, created_at, updated_at, accessed_at, access_count, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(memory.id.to_string())
        .bind(memory.project_id.to_string())
        .bind(memory.agent_id.map(|id| id.to_string()))
        .bind(memory.session_id.map(|id| id.to_string()))
        .bind(serde_json::to_string(&memory.memory_type)?)
        .bind(serde_json::to_string(&memory.status)?)
        .bind(&memory.content)
        .bind(&memory.summary)
        .bind(memory.importance as f64)
        .bind(serde_json::to_string(&memory.tags)?)
        .bind(serde_json::to_string(&memory.provenance)?)
        .bind(memory.created_at.to_rfc3339())
        .bind(memory.updated_at.to_rfc3339())
        .bind(memory.accessed_at.map(|value| value.to_rfc3339()))
        .bind(memory.access_count as i64)
        .bind(memory.version as i64)
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                Err(DomainError::Duplicate("memory already exists".into()))
            }
            Err(error) => Err(DomainError::InvariantViolation(format!(
                "memory persistence failed: {error}"
            ))),
        }
    }

    pub async fn get(
        &self,
        project_id: &ProjectId,
        memory_id: &MemoryId,
    ) -> Result<Option<Memory>, DomainError> {
        let row = sqlx::query("SELECT * FROM memories WHERE project_id = ? AND id = ?")
            .bind(project_id.to_string())
            .bind(memory_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                DomainError::InvariantViolation(format!("memory query failed: {error}"))
            })?;
        row.map(decode_memory).transpose()
    }

    pub async fn list_active(
        &self,
        project_id: &ProjectId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Memory>, DomainError> {
        let rows = sqlx::query("SELECT * FROM memories WHERE project_id = ? AND status != ? ORDER BY updated_at DESC, id ASC LIMIT ? OFFSET ?")
            .bind(project_id.to_string())
            .bind(serde_json::to_string(&MemoryStatus::Archived)?)
            .bind(limit.min(100) as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| DomainError::InvariantViolation(format!("memory list failed: {error}")))?;
        rows.into_iter().map(decode_memory).collect()
    }

    pub async fn archive(
        &self,
        project_id: &ProjectId,
        memory_id: &MemoryId,
        expected_version: u64,
    ) -> Result<(), DomainError> {
        let archived = serde_json::to_string(&MemoryStatus::Archived)?;
        let result = sqlx::query("UPDATE memories SET status = ?, updated_at = ?, version = version + 1 WHERE project_id = ? AND id = ? AND version = ? AND status != ?")
            .bind(archived)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(project_id.to_string())
            .bind(memory_id.to_string())
            .bind(expected_version as i64)
            .bind(serde_json::to_string(&MemoryStatus::Archived)?)
            .execute(&self.pool)
            .await
            .map_err(|error| DomainError::InvariantViolation(format!("memory archive failed: {error}")))?;
        if result.rows_affected() == 0 {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_version.to_string(),
                actual: "unknown".into(),
            });
        }
        Ok(())
    }
}

fn decode_memory(row: sqlx::sqlite::SqliteRow) -> Result<Memory, DomainError> {
    let parse_id =
        |value: Result<String, sqlx::Error>, label: &str| -> Result<MemoryId, DomainError> {
            MemoryId::from_str(&value.map_err(|_| DomainError::Validation(label.into()))?)
                .map_err(|_| DomainError::Validation(label.into()))
        };
    let project_id = ProjectId::from_str(
        &row.try_get::<String, _>("project_id")
            .map_err(|_| DomainError::Validation("invalid memory project".into()))?,
    )
    .map_err(|_| DomainError::Validation("invalid memory project".into()))?;
    Ok(Memory {
        id: parse_id(row.try_get("id"), "invalid memory id")?,
        project_id,
        agent_id: None,
        session_id: None,
        memory_type: serde_json::from_str(
            &row.try_get::<String, _>("memory_type")
                .map_err(|_| DomainError::Validation("invalid memory type".into()))?,
        )?,
        status: serde_json::from_str(
            &row.try_get::<String, _>("status")
                .map_err(|_| DomainError::Validation("invalid memory status".into()))?,
        )?,
        content: row
            .try_get("content")
            .map_err(|_| DomainError::Validation("invalid memory content".into()))?,
        summary: row
            .try_get("summary")
            .map_err(|_| DomainError::Validation("invalid memory summary".into()))?,
        importance: row
            .try_get::<f64, _>("importance")
            .map_err(|_| DomainError::Validation("invalid memory importance".into()))?
            as f32,
        tags: serde_json::from_str(
            &row.try_get::<String, _>("tags")
                .map_err(|_| DomainError::Validation("invalid memory tags".into()))?,
        )?,
        provenance: serde_json::from_str(
            &row.try_get::<String, _>("provenance")
                .map_err(|_| DomainError::Validation("invalid memory provenance".into()))?,
        )?,
        created_at: parse_time(row.try_get("created_at"))?,
        updated_at: parse_time(row.try_get("updated_at"))?,
        accessed_at: row
            .try_get::<Option<String>, _>("accessed_at")
            .map_err(|_| DomainError::Validation("invalid memory access time".into()))?
            .map(|value| {
                chrono::DateTime::parse_from_rfc3339(&value)
                    .map(|date| date.with_timezone(&chrono::Utc))
                    .map_err(|_| DomainError::Validation("invalid memory access time".into()))
            })
            .transpose()?,
        access_count: row
            .try_get::<i64, _>("access_count")
            .map_err(|_| DomainError::Validation("invalid memory access count".into()))?
            as u64,
        version: row
            .try_get::<i64, _>("version")
            .map_err(|_| DomainError::Validation("invalid memory version".into()))?
            as u64,
    })
}

fn parse_time(
    value: Result<String, sqlx::Error>,
) -> Result<chrono::DateTime<chrono::Utc>, DomainError> {
    chrono::DateTime::parse_from_rfc3339(
        &value.map_err(|_| DomainError::Validation("invalid memory time".into()))?,
    )
    .map(|date| date.with_timezone(&chrono::Utc))
    .map_err(|_| DomainError::Validation("invalid memory time".into()))
}
