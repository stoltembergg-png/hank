use sqlx::{Pool, Row, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MAX_ID_BYTES: usize = 128;
const MAX_STATE_BYTES: usize = 32;
const MAX_CHECKPOINT_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StateError {
    #[error("workflow state identity is invalid")]
    InvalidIdentity,
    #[error("workflow state value is invalid")]
    InvalidValue,
    #[error("workflow state checkpoint is invalid or sensitive")]
    InvalidCheckpoint,
    #[error("workflow state record already exists")]
    Duplicate,
    #[error("workflow state compare-and-set conflict")]
    Conflict,
    #[error("workflow state query failed")]
    Query,
    #[error("workflow state serialization failed")]
    Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRun {
    pub project_id: String,
    pub run_id: String,
    pub workflow_id: String,
    pub workflow_version: u32,
}

impl CreateRun {
    pub fn new(
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        workflow_id: impl Into<String>,
        workflow_version: u32,
    ) -> Result<Self, StateError> {
        let run = Self {
            project_id: project_id.into(),
            run_id: run_id.into(),
            workflow_id: workflow_id.into(),
            workflow_version,
        };
        for value in [&run.project_id, &run.run_id, &run.workflow_id] {
            if !valid_id(value) {
                return Err(StateError::InvalidIdentity);
            }
        }
        Ok(run)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub project_id: String,
    pub run_id: String,
    pub node_id: String,
    pub generation: u64,
    pub expected_state: String,
    pub next_state: String,
    pub idempotency_key: String,
    pub checkpoint: Option<serde_json::Value>,
}

impl Transition {
    pub fn new(
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        node_id: impl Into<String>,
        generation: u64,
        expected_state: impl Into<String>,
        next_state: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Result<Self, StateError> {
        let transition = Self {
            project_id: project_id.into(),
            run_id: run_id.into(),
            node_id: node_id.into(),
            generation,
            expected_state: expected_state.into(),
            next_state: next_state.into(),
            idempotency_key: idempotency_key.into(),
            checkpoint: None,
        };
        if !valid_id(&transition.project_id)
            || !valid_id(&transition.run_id)
            || !valid_id(&transition.node_id)
            || !valid_id(&transition.idempotency_key)
            || !valid_state(&transition.expected_state)
            || !valid_state(&transition.next_state)
        {
            return Err(StateError::InvalidIdentity);
        }
        Ok(transition)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionOutcome {
    Applied { sequence: u64, generation: u64 },
    Replayed { sequence: u64, generation: u64 },
}

#[derive(Clone)]
pub struct StateStore {
    pool: Pool<Sqlite>,
}

impl StateStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create_run(&self, run: CreateRun) -> Result<(), StateError> {
        sqlx::query("INSERT INTO workflow_runs (project_id, run_id, workflow_id, workflow_version, state, generation, sequence, created_at_ms, updated_at_ms) VALUES (?, ?, ?, ?, 'running', 0, 0, ?, ?)")
            .bind(run.project_id).bind(run.run_id).bind(run.workflow_id).bind(i64::from(run.workflow_version)).bind(now_ms()).bind(now_ms()).execute(&self.pool).await.map_err(map_query)?;
        Ok(())
    }

    pub async fn transition(
        &self,
        transition: Transition,
    ) -> Result<TransitionOutcome, StateError> {
        validate_checkpoint(&transition.checkpoint)?;
        let mut tx = self.pool.begin().await.map_err(map_query)?;
        if let Some(row) = sqlx::query("SELECT sequence, generation FROM workflow_transitions WHERE project_id = ? AND run_id = ? AND idempotency_key = ?")
            .bind(&transition.project_id).bind(&transition.run_id).bind(&transition.idempotency_key).fetch_optional(&mut *tx).await.map_err(map_query)? {
            return Ok(TransitionOutcome::Replayed { sequence: to_u64(row.get::<i64, _>("sequence"))?, generation: to_u64(row.get::<i64, _>("generation"))? });
        }
        let run =
            sqlx::query("SELECT sequence FROM workflow_runs WHERE project_id = ? AND run_id = ?")
                .bind(&transition.project_id)
                .bind(&transition.run_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_query)?
                .ok_or(StateError::Conflict)?;
        let sequence = to_u64(run.get::<i64, _>("sequence"))?
            .checked_add(1)
            .ok_or(StateError::Serialization)?;
        let node = sqlx::query("SELECT state, generation FROM workflow_node_states WHERE project_id = ? AND run_id = ? AND node_id = ?")
            .bind(&transition.project_id).bind(&transition.run_id).bind(&transition.node_id).fetch_optional(&mut *tx).await.map_err(map_query)?;
        let (current_state, current_generation) = node
            .map(|row| {
                (
                    row.get::<String, _>("state"),
                    to_u64(row.get::<i64, _>("generation")).unwrap_or(u64::MAX),
                )
            })
            .unwrap_or_else(|| ("ready".into(), 0));
        if current_state != transition.expected_state || current_generation != transition.generation
        {
            return Err(StateError::Conflict);
        }
        let checkpoint = transition
            .checkpoint
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| StateError::Serialization)?;
        sqlx::query("INSERT INTO workflow_transitions (project_id, run_id, transition_id, idempotency_key, sequence, node_id, expected_state, next_state, generation, recovery_class, created_at_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'committed', ?)")
            .bind(&transition.project_id).bind(&transition.run_id).bind(format!("{}:{}", transition.node_id, sequence)).bind(&transition.idempotency_key).bind(i64::try_from(sequence).map_err(|_| StateError::Serialization)?).bind(&transition.node_id).bind(&transition.expected_state).bind(&transition.next_state).bind(i64::try_from(transition.generation).map_err(|_| StateError::Serialization)?).bind(now_ms()).execute(&mut *tx).await.map_err(map_query)?;
        sqlx::query("INSERT INTO workflow_node_states (project_id, run_id, node_id, state, generation, attempt, checkpoint_before, checkpoint_after, updated_at_ms) VALUES (?, ?, ?, ?, ?, 1, NULL, ?, ?) ON CONFLICT(project_id, run_id, node_id) DO UPDATE SET state = excluded.state, generation = excluded.generation, checkpoint_before = workflow_node_states.checkpoint_after, checkpoint_after = excluded.checkpoint_after, attempt = workflow_node_states.attempt + 1, updated_at_ms = excluded.updated_at_ms")
            .bind(&transition.project_id).bind(&transition.run_id).bind(&transition.node_id).bind(&transition.next_state).bind(i64::try_from(transition.generation).map_err(|_| StateError::Serialization)?).bind(checkpoint).bind(now_ms()).execute(&mut *tx).await.map_err(map_query)?;
        sqlx::query("UPDATE workflow_runs SET sequence = ?, updated_at_ms = ? WHERE project_id = ? AND run_id = ?")
            .bind(i64::try_from(sequence).map_err(|_| StateError::Serialization)?).bind(now_ms()).bind(&transition.project_id).bind(&transition.run_id).execute(&mut *tx).await.map_err(map_query)?;
        tx.commit().await.map_err(map_query)?;
        Ok(TransitionOutcome::Applied {
            sequence,
            generation: transition.generation,
        })
    }
}

fn validate_checkpoint(value: &Option<serde_json::Value>) -> Result<(), StateError> {
    let Some(value) = value else {
        return Ok(());
    };
    let text = serde_json::to_string(value).map_err(|_| StateError::Serialization)?;
    if text.len() > MAX_CHECKPOINT_BYTES
        || text.to_ascii_lowercase().contains("credential")
        || text.to_ascii_lowercase().contains("password")
        || text.to_ascii_lowercase().contains("secret")
        || text.to_ascii_lowercase().contains("token")
    {
        return Err(StateError::InvalidCheckpoint);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.chars().all(|character| !character.is_control())
}
fn valid_state(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_STATE_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
fn to_u64(value: i64) -> Result<u64, StateError> {
    u64::try_from(value).map_err(|_| StateError::Serialization)
}
fn map_query(error: sqlx::Error) -> StateError {
    if error
        .as_database_error()
        .is_some_and(|db| db.is_unique_violation())
    {
        StateError::Duplicate
    } else {
        StateError::Query
    }
}
