//! SQLite persistence boundary for Session metadata.

use agent_core::ids::{AgentId, ProjectId, SessionId};
use agent_core::session::{Session, SessionParticipant, SessionStatus};
use agent_protocol::ids::TraceId;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;
use thiserror::Error;

const MAX_PAGE_SIZE: u32 = 100;

#[derive(Debug, Error)]
pub enum SessionStorageError {
    #[error("session project scope mismatch")]
    ScopeMismatch,
    #[error("session was not found")]
    NotFound,
    #[error("session optimistic concurrency conflict")]
    Conflict,
    #[error("session state or metadata is invalid")]
    Invalid,
    #[error("session serialization failed: {0}")]
    Serialization(String),
    #[error("session database operation failed: {0}")]
    Database(String),
}

#[derive(Clone)]
pub struct SqliteSessionRepository {
    pool: Pool<Sqlite>,
}

impl SqliteSessionRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> Pool<Sqlite> {
        self.pool.clone()
    }

    pub async fn create(&self, session: &Session) -> Result<(), SessionStorageError> {
        let encoded = encode_session(session)?;
        let result = sqlx::query(
            "INSERT INTO sessions (id, project_id, agent_id, status, title, message_count, token_count, cost_usd, created_at, updated_at, closed_at, schema_version, correlation_id, participants, metadata, budget_ref, trace_id, failure_reason) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session.id.to_string())
        .bind(session.project_id.to_string())
        .bind(session.agent_id.to_string())
        .bind(encoded.status)
        .bind(&session.title)
        .bind(session.message_count as i64)
        .bind(session.token_count as i64)
        .bind(session.cost_usd)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(session.closed_at.map(|value| value.to_rfc3339()))
        .bind(session.schema_version as i64)
        .bind(&session.correlation_id)
        .bind(encoded.participants)
        .bind(encoded.metadata)
        .bind(&session.budget_ref)
        .bind(encoded.trace_id)
        .bind(&session.failure_reason)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            if matches!(error, sqlx::Error::Database(ref database) if database.is_unique_violation()) {
                SessionStorageError::Conflict
            } else {
                SessionStorageError::Database(error.to_string())
            }
        })?;
        if result.rows_affected() != 1 {
            return Err(SessionStorageError::Conflict);
        }
        Ok(())
    }

    pub async fn get_by_id(
        &self,
        project_id: &ProjectId,
        session_id: &SessionId,
    ) -> Result<Option<Session>, SessionStorageError> {
        let row = sqlx::query(
            "SELECT id, project_id, agent_id, status, title, message_count, token_count, cost_usd, created_at, updated_at, closed_at, schema_version, correlation_id, participants, metadata, budget_ref, trace_id, failure_reason \
             FROM sessions WHERE id = ? AND project_id = ?",
        )
        .bind(session_id.to_string())
        .bind(project_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| SessionStorageError::Database(error.to_string()))?;

        if let Some(row) = row {
            return decode_session(row).map(Some);
        }

        let exists = sqlx::query("SELECT project_id FROM sessions WHERE id = ?")
            .bind(session_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| SessionStorageError::Database(error.to_string()))?;
        if exists.is_some() {
            return Err(SessionStorageError::ScopeMismatch);
        }
        Ok(None)
    }

    pub async fn list(
        &self,
        project_id: &ProjectId,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Session>, SessionStorageError> {
        if limit == 0 {
            return Err(SessionStorageError::Invalid);
        }
        let bounded_limit = limit.min(MAX_PAGE_SIZE);
        let rows = sqlx::query(
            "SELECT id, project_id, agent_id, status, title, message_count, token_count, cost_usd, created_at, updated_at, closed_at, schema_version, correlation_id, participants, metadata, budget_ref, trace_id, failure_reason \
             FROM sessions WHERE project_id = ? ORDER BY created_at DESC, id ASC LIMIT ? OFFSET ?",
        )
        .bind(project_id.to_string())
        .bind(i64::from(bounded_limit))
        .bind(i64::from(offset))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| SessionStorageError::Database(error.to_string()))?;
        rows.into_iter().map(decode_session).collect()
    }

    pub async fn list_for_agent(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Session>, SessionStorageError> {
        if limit == 0 {
            return Err(SessionStorageError::Invalid);
        }
        let bounded_limit = limit.min(MAX_PAGE_SIZE);
        let rows = sqlx::query(
            "SELECT id, project_id, agent_id, status, title, message_count, token_count, cost_usd, created_at, updated_at, closed_at, schema_version, correlation_id, participants, metadata, budget_ref, trace_id, failure_reason \
             FROM sessions WHERE project_id = ? AND agent_id = ? ORDER BY created_at DESC, id ASC LIMIT ? OFFSET ?",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .bind(i64::from(bounded_limit))
        .bind(i64::from(offset))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| SessionStorageError::Database(error.to_string()))?;
        rows.into_iter().map(decode_session).collect()
    }

    pub async fn count_for_agent(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
    ) -> Result<usize, SessionStorageError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS count FROM sessions WHERE project_id = ? AND agent_id = ?",
        )
        .bind(project_id.to_string())
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(|error| SessionStorageError::Database(error.to_string()))?;
        let count: i64 = row.get("count");
        usize::try_from(count).map_err(|_| SessionStorageError::Invalid)
    }

    pub async fn update(
        &self,
        session: &Session,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<(), SessionStorageError> {
        let encoded = encode_session(session)?;
        let result = sqlx::query(
            "UPDATE sessions SET status = ?, title = ?, message_count = ?, token_count = ?, cost_usd = ?, updated_at = ?, closed_at = ?, schema_version = ?, correlation_id = ?, participants = ?, metadata = ?, budget_ref = ?, trace_id = ?, failure_reason = ? \
             WHERE id = ? AND project_id = ? AND updated_at = ?",
        )
        .bind(encoded.status)
        .bind(&session.title)
        .bind(session.message_count as i64)
        .bind(session.token_count as i64)
        .bind(session.cost_usd)
        .bind(session.updated_at.to_rfc3339())
        .bind(session.closed_at.map(|value| value.to_rfc3339()))
        .bind(session.schema_version as i64)
        .bind(&session.correlation_id)
        .bind(encoded.participants)
        .bind(encoded.metadata)
        .bind(&session.budget_ref)
        .bind(encoded.trace_id)
        .bind(&session.failure_reason)
        .bind(session.id.to_string())
        .bind(session.project_id.to_string())
        .bind(expected_updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|error| SessionStorageError::Database(error.to_string()))?;
        if result.rows_affected() == 1 {
            return Ok(());
        }
        if self
            .get_by_id(&session.project_id, &session.id)
            .await?
            .is_some()
        {
            Err(SessionStorageError::Conflict)
        } else {
            Err(SessionStorageError::NotFound)
        }
    }

    pub async fn close(
        &self,
        project_id: &ProjectId,
        session_id: &SessionId,
    ) -> Result<Session, SessionStorageError> {
        let mut session = self
            .get_by_id(project_id, session_id)
            .await?
            .ok_or(SessionStorageError::NotFound)?;
        if session.status == SessionStatus::Closed {
            return Ok(session);
        }
        let expected = session.updated_at;
        session
            .begin_close()
            .map_err(|_| SessionStorageError::Invalid)?;
        session.close().map_err(|_| SessionStorageError::Invalid)?;
        self.update(&session, expected).await?;
        Ok(session)
    }
}

struct EncodedSession {
    status: &'static str,
    participants: String,
    metadata: String,
    trace_id: Option<String>,
}

fn encode_session(session: &Session) -> Result<EncodedSession, SessionStorageError> {
    if session.schema_version != Session::SCHEMA_VERSION
        || session.project_id.to_string().is_empty()
        || session.agent_id.to_string().is_empty()
        || session.correlation_id.trim().is_empty()
    {
        return Err(SessionStorageError::Invalid);
    }
    let participants = serde_json::to_string(&session.participants)
        .map_err(|error| SessionStorageError::Serialization(error.to_string()))?;
    let metadata = serde_json::to_string(&session.metadata)
        .map_err(|error| SessionStorageError::Serialization(error.to_string()))?;
    Ok(EncodedSession {
        status: status_to_str(session.status),
        participants,
        metadata,
        trace_id: session.trace_id.map(|value| value.to_string()),
    })
}

fn status_to_str(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Created => "created",
        SessionStatus::Active => "active",
        SessionStatus::Closing => "closing",
        SessionStatus::Closed => "closed",
        SessionStatus::Failed => "failed",
    }
}

fn status_from_str(value: &str) -> Result<SessionStatus, SessionStorageError> {
    match value {
        "created" => Ok(SessionStatus::Created),
        "active" => Ok(SessionStatus::Active),
        "closing" => Ok(SessionStatus::Closing),
        "closed" => Ok(SessionStatus::Closed),
        "failed" => Ok(SessionStatus::Failed),
        _ => Err(SessionStorageError::Invalid),
    }
}

fn decode_session(row: sqlx::sqlite::SqliteRow) -> Result<Session, SessionStorageError> {
    let id = SessionId::from_str(&row.get::<String, _>("id"))
        .map_err(|_| SessionStorageError::Invalid)?;
    let project_id = ProjectId::from_str(&row.get::<String, _>("project_id"))
        .map_err(|_| SessionStorageError::Invalid)?;
    let agent_id = AgentId::from_str(&row.get::<String, _>("agent_id"))
        .map_err(|_| SessionStorageError::Invalid)?;
    let participants =
        serde_json::from_str::<Vec<SessionParticipant>>(&row.get::<String, _>("participants"))
            .map_err(|_| SessionStorageError::Serialization("participants".into()))?;
    let metadata = serde_json::from_str::<std::collections::BTreeMap<String, Value>>(
        &row.get::<String, _>("metadata"),
    )
    .map_err(|_| SessionStorageError::Serialization("metadata".into()))?;
    let trace_id = row
        .get::<Option<String>, _>("trace_id")
        .map(|value| TraceId::from_str(&value).map_err(|_| SessionStorageError::Invalid))
        .transpose()?;
    let created_at_value: String = row.get("created_at");
    let updated_at_value: String = row.get("updated_at");
    let closed_at_value: Option<String> = row.get("closed_at");
    Ok(Session {
        schema_version: row.get::<i64, _>("schema_version") as u32,
        id,
        project_id,
        agent_id,
        status: status_from_str(&row.get::<String, _>("status"))?,
        correlation_id: row.get("correlation_id"),
        participants,
        metadata,
        budget_ref: row.get("budget_ref"),
        trace_id,
        failure_reason: row.get("failure_reason"),
        title: row.get("title"),
        message_count: row.get::<i64, _>("message_count") as usize,
        token_count: row.get::<i64, _>("token_count") as u64,
        cost_usd: row.get("cost_usd"),
        created_at: parse_timestamp(&created_at_value)?,
        updated_at: parse_timestamp(&updated_at_value)?,
        closed_at: closed_at_value
            .map(|value| parse_timestamp(&value))
            .transpose()?,
    })
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, SessionStorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| SessionStorageError::Invalid)
}
