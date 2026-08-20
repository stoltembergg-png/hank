//! SQLite persistence boundary for ordered Message records.

use agent_core::ids::{ProjectId, SessionId};
use agent_core::session::{Message, MessagePart, MessageProvenance, MessageRole, MessageStatus};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};
use thiserror::Error;

const MAX_PAGE_SIZE: u32 = 100;

#[derive(Debug, Error)]
pub enum MessageStorageError {
    #[error("message session scope mismatch")]
    ScopeMismatch,
    #[error("message was not found")]
    NotFound,
    #[error("message storage conflict")]
    Conflict,
    #[error("message sequence was duplicated")]
    DuplicateSequence,
    #[error("message sequence was out of order: expected {expected}, got {actual}")]
    OutOfOrder { expected: u64, actual: u64 },
    #[error("message generation is stale")]
    StaleGeneration,
    #[error("message is invalid")]
    Invalid,
    #[error("message serialization failed: {0}")]
    Serialization(String),
    #[error("message database operation failed: {0}")]
    Database(String),
}

#[derive(Clone)]
pub struct SqliteMessageRepository {
    pool: Pool<Sqlite>,
}

impl SqliteMessageRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn append(
        &self,
        project_id: &ProjectId,
        session_id: &SessionId,
        message: &Message,
    ) -> Result<(), MessageStorageError> {
        if message.session_id != *session_id {
            return Err(MessageStorageError::ScopeMismatch);
        }
        message
            .validate()
            .map_err(|_| MessageStorageError::Invalid)?;
        self.ensure_session_scope(project_id, session_id).await?;
        let existing_id = sqlx::query("SELECT 1 FROM messages WHERE id = ?")
            .bind(message.id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if existing_id.is_some() {
            return Err(MessageStorageError::Conflict);
        }
        let previous = sqlx::query(
            "SELECT generation, sequence FROM messages WHERE session_id = ? ORDER BY generation DESC, sequence DESC LIMIT 1",
        )
        .bind(message.session_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        if let Some(row) = previous {
            let generation = row.get::<i64, _>("generation") as u64;
            let sequence = row.get::<i64, _>("sequence") as u64;
            if message.generation < generation {
                return Err(MessageStorageError::StaleGeneration);
            }
            if message.generation == generation {
                let expected = sequence.saturating_add(1);
                if message.sequence < expected {
                    return Err(MessageStorageError::DuplicateSequence);
                }
                if message.sequence > expected {
                    return Err(MessageStorageError::OutOfOrder {
                        expected,
                        actual: message.sequence,
                    });
                }
            } else if message.sequence != 0 {
                return Err(MessageStorageError::OutOfOrder {
                    expected: 0,
                    actual: message.sequence,
                });
            }
        } else if message.sequence != 0 {
            return Err(MessageStorageError::OutOfOrder {
                expected: 0,
                actual: message.sequence,
            });
        }

        let encoded = encode_message(message)?;
        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, tool_results, tokens, cost_usd, created_at, schema_version, provenance, status, correlation_id, sequence, generation, parts) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(message.id.to_string())
        .bind(message.session_id.to_string())
        .bind(encoded.role)
        .bind(&message.content)
        .bind(encoded.tool_calls)
        .bind(encoded.tool_results)
        .bind(i64::from(message.tokens))
        .bind(message.cost_usd)
        .bind(message.created_at.to_rfc3339())
        .bind(message.schema_version as i64)
        .bind(encoded.provenance)
        .bind(encoded.status)
        .bind(&message.correlation_id)
        .bind(message.sequence as i64)
        .bind(message.generation as i64)
        .bind(encoded.parts)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            if matches!(error, sqlx::Error::Database(ref database) if database.is_unique_violation()) {
                MessageStorageError::Conflict
            } else {
                db_error(error)
            }
        })?;
        Ok(())
    }

    pub async fn get_by_id(
        &self,
        project_id: &ProjectId,
        session_id: &SessionId,
        message_id: &agent_core::ids::MessageId,
    ) -> Result<Option<Message>, MessageStorageError> {
        let row = sqlx::query(
            "SELECT m.id, m.session_id, m.role, m.content, m.tool_calls, m.tool_results, m.tokens, m.cost_usd, m.created_at, m.schema_version, m.provenance, m.status, m.correlation_id, m.sequence, m.generation, m.parts \
             FROM messages m JOIN sessions s ON s.id = m.session_id WHERE m.id = ? AND m.session_id = ? AND s.project_id = ?",
        )
        .bind(message_id.to_string())
        .bind(session_id.to_string())
        .bind(project_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        if let Some(row) = row {
            return decode_message(row).map(Some);
        }
        let exists = sqlx::query("SELECT session_id FROM messages WHERE id = ?")
            .bind(message_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if exists.is_some() {
            return Err(MessageStorageError::NotFound);
        }
        Ok(None)
    }

    pub async fn list(
        &self,
        project_id: &ProjectId,
        session_id: &SessionId,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Message>, MessageStorageError> {
        if limit == 0 {
            return Err(MessageStorageError::Invalid);
        }
        let bounded_limit = limit.min(MAX_PAGE_SIZE);
        let rows = sqlx::query(
            "SELECT m.id, m.session_id, m.role, m.content, m.tool_calls, m.tool_results, m.tokens, m.cost_usd, m.created_at, m.schema_version, m.provenance, m.status, m.correlation_id, m.sequence, m.generation, m.parts \
             FROM messages m JOIN sessions s ON s.id = m.session_id WHERE m.session_id = ? AND s.project_id = ? ORDER BY m.generation ASC, m.sequence ASC LIMIT ? OFFSET ?",
        )
        .bind(session_id.to_string())
        .bind(project_id.to_string())
        .bind(i64::from(bounded_limit))
        .bind(i64::from(offset))
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        rows.into_iter().map(decode_message).collect()
    }

    pub async fn update(
        &self,
        project_id: &ProjectId,
        message: &Message,
        expected_status: MessageStatus,
    ) -> Result<(), MessageStorageError> {
        message
            .validate()
            .map_err(|_| MessageStorageError::Invalid)?;
        self.ensure_session_scope(project_id, &message.session_id)
            .await?;
        let current = self
            .get_by_id(project_id, &message.session_id, &message.id)
            .await?
            .ok_or(MessageStorageError::NotFound)?;
        if current.status != expected_status {
            return Err(MessageStorageError::Conflict);
        }
        if current.status == message.status && current.status.is_terminal() {
            return Ok(());
        }
        let encoded = encode_message(message)?;
        let result = sqlx::query(
            "UPDATE messages SET role = ?, content = ?, tool_calls = ?, tool_results = ?, tokens = ?, cost_usd = ?, schema_version = ?, provenance = ?, status = ?, correlation_id = ?, sequence = ?, generation = ?, parts = ? WHERE id = ? AND session_id = ? AND status = ?",
        )
        .bind(encoded.role)
        .bind(&message.content)
        .bind(encoded.tool_calls)
        .bind(encoded.tool_results)
        .bind(i64::from(message.tokens))
        .bind(message.cost_usd)
        .bind(message.schema_version as i64)
        .bind(encoded.provenance)
        .bind(encoded.status)
        .bind(&message.correlation_id)
        .bind(message.sequence as i64)
        .bind(message.generation as i64)
        .bind(encoded.parts)
        .bind(message.id.to_string())
        .bind(message.session_id.to_string())
        .bind(status_to_str(expected_status))
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(MessageStorageError::Conflict)
        }
    }

    async fn ensure_session_scope(
        &self,
        project_id: &ProjectId,
        session_id: &SessionId,
    ) -> Result<(), MessageStorageError> {
        let row = sqlx::query("SELECT project_id FROM sessions WHERE id = ?")
            .bind(session_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(MessageStorageError::NotFound)?;
        if row.get::<String, _>("project_id") != project_id.to_string() {
            return Err(MessageStorageError::ScopeMismatch);
        }
        Ok(())
    }
}

struct EncodedMessage {
    role: &'static str,
    provenance: &'static str,
    status: &'static str,
    tool_calls: String,
    tool_results: String,
    parts: String,
}

fn encode_message(message: &Message) -> Result<EncodedMessage, MessageStorageError> {
    Ok(EncodedMessage {
        role: role_to_str(message.role),
        provenance: provenance_to_str(message.provenance),
        status: status_to_str(message.status),
        tool_calls: serde_json::to_string(&message.tool_calls)
            .map_err(|error| MessageStorageError::Serialization(error.to_string()))?,
        tool_results: serde_json::to_string(&message.tool_results)
            .map_err(|error| MessageStorageError::Serialization(error.to_string()))?,
        parts: serde_json::to_string(&message.parts)
            .map_err(|error| MessageStorageError::Serialization(error.to_string()))?,
    })
}

fn role_to_str(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
        MessageRole::ToolResult => "tool_result",
    }
}

fn role_from_str(value: &str) -> Result<MessageRole, MessageStorageError> {
    match value {
        "system" => Ok(MessageRole::System),
        "user" => Ok(MessageRole::User),
        "assistant" => Ok(MessageRole::Assistant),
        "tool" => Ok(MessageRole::Tool),
        "tool_result" => Ok(MessageRole::ToolResult),
        _ => Err(MessageStorageError::Invalid),
    }
}

fn provenance_to_str(value: MessageProvenance) -> &'static str {
    match value {
        MessageProvenance::System => "system",
        MessageProvenance::User => "user",
        MessageProvenance::Agent => "agent",
        MessageProvenance::Provider => "provider",
        MessageProvenance::Tool => "tool",
    }
}

fn provenance_from_str(value: &str) -> Result<MessageProvenance, MessageStorageError> {
    match value {
        "system" => Ok(MessageProvenance::System),
        "user" => Ok(MessageProvenance::User),
        "agent" => Ok(MessageProvenance::Agent),
        "provider" => Ok(MessageProvenance::Provider),
        "tool" => Ok(MessageProvenance::Tool),
        _ => Err(MessageStorageError::Invalid),
    }
}

fn status_to_str(value: MessageStatus) -> &'static str {
    match value {
        MessageStatus::Draft => "draft",
        MessageStatus::Streaming => "streaming",
        MessageStatus::Complete => "complete",
        MessageStatus::Failed => "failed",
        MessageStatus::Cancelled => "cancelled",
    }
}

fn status_from_str(value: &str) -> Result<MessageStatus, MessageStorageError> {
    match value {
        "draft" => Ok(MessageStatus::Draft),
        "streaming" => Ok(MessageStatus::Streaming),
        "complete" => Ok(MessageStatus::Complete),
        "failed" => Ok(MessageStatus::Failed),
        "cancelled" => Ok(MessageStatus::Cancelled),
        _ => Err(MessageStorageError::Invalid),
    }
}

fn decode_message(row: sqlx::sqlite::SqliteRow) -> Result<Message, MessageStorageError> {
    let parts = serde_json::from_str::<Vec<MessagePart>>(&row.get::<String, _>("parts"))
        .map_err(|_| MessageStorageError::Serialization("parts".into()))?;
    let tool_calls = serde_json::from_str(
        &row.get::<Option<String>, _>("tool_calls")
            .unwrap_or_else(|| "[]".into()),
    )
    .map_err(|_| MessageStorageError::Serialization("tool_calls".into()))?;
    let tool_results = serde_json::from_str(
        &row.get::<Option<String>, _>("tool_results")
            .unwrap_or_else(|| "[]".into()),
    )
    .map_err(|_| MessageStorageError::Serialization("tool_results".into()))?;
    Ok(Message {
        schema_version: row.get::<i64, _>("schema_version") as u32,
        id: row
            .get::<String, _>("id")
            .parse()
            .map_err(|_| MessageStorageError::Invalid)?,
        session_id: row
            .get::<String, _>("session_id")
            .parse()
            .map_err(|_| MessageStorageError::Invalid)?,
        role: role_from_str(&row.get::<String, _>("role"))?,
        provenance: provenance_from_str(&row.get::<String, _>("provenance"))?,
        status: status_from_str(&row.get::<String, _>("status"))?,
        correlation_id: row.get("correlation_id"),
        sequence: row.get::<i64, _>("sequence") as u64,
        generation: row.get::<i64, _>("generation") as u64,
        parts,
        content: row.get("content"),
        tool_calls,
        tool_results,
        tokens: row.get::<i64, _>("tokens") as u32,
        cost_usd: row.get("cost_usd"),
        created_at: parse_timestamp(&row.get::<String, _>("created_at"))?,
    })
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, MessageStorageError> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| MessageStorageError::Invalid)
}

fn db_error(error: sqlx::Error) -> MessageStorageError {
    MessageStorageError::Database(error.to_string())
}
