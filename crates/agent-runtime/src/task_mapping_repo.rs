//! Persistência SQLite do contrato de task-to-branch mapping.
//!
//! Este adapter só persiste metadados validados pelo domínio. Ele não executa
//! Git, não concede capabilities e não interpreta conteúdo de arquivos.

use agent_core::task_mapping::{MappingObservation, MappingState, TaskWorkspaceMapping};
use agent_core::{ProjectId, TaskId, TraceId};
use sqlx::{Pool, Row, Sqlite};
use std::str::FromStr;
use thiserror::Error;

const MAX_LIST_LIMIT: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TaskMappingPersistenceError {
    #[error("task mapping identity is invalid")]
    InvalidIdentity,
    #[error("task mapping project scope is invalid")]
    ProjectScope,
    #[error("task mapping was not found")]
    NotFound,
    #[error("task mapping revision conflict")]
    Conflict,
    #[error("task mapping already exists")]
    Duplicate,
    #[error("task mapping row is malformed")]
    Corrupt,
    #[error("task mapping timestamp is invalid")]
    Timestamp,
    #[error("task mapping storage query failed")]
    Query,
}

#[derive(Clone)]
pub struct TaskWorkspaceMappingRepository {
    pool: Pool<Sqlite>,
}

impl TaskWorkspaceMappingRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        mapping: &TaskWorkspaceMapping,
        now_ms: u64,
    ) -> Result<(), TaskMappingPersistenceError> {
        if mapping.revision() != 1 {
            return Err(TaskMappingPersistenceError::InvalidIdentity);
        }
        let now = to_i64(now_ms)?;
        let project = mapping.project_id().to_string();
        let task = mapping.task_id().to_string();
        let observation = mapping.observation();
        let observed_at_ms = observation
            .map(|value| to_i64(value.observed_at_ms()))
            .transpose()?;
        let last_reconciled_at_ms = mapping.last_reconciled_at_ms().map(to_i64).transpose()?;
        let last_resumed_at_ms = mapping.last_resumed_at_ms().map(to_i64).transpose()?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| TaskMappingPersistenceError::Query)?;

        let project_exists = sqlx::query("SELECT 1 FROM projects WHERE id = ?")
            .bind(&project)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| TaskMappingPersistenceError::Query)?
            .is_some();
        if !project_exists {
            return Err(TaskMappingPersistenceError::ProjectScope);
        }

        let result = sqlx::query(
            "INSERT INTO task_workspace_mappings (
                project_id, task_id, repository_id, worktree_id, branch, agent_run_id,
                pull_request_id, correlation_id, policy_revision, state, revision,
                observed_repository_id, observed_worktree_id, observed_branch,
                observed_at_ms, observed_correlation_id, reconcile_reason,
                last_reconciled_at_ms, last_resumed_at_ms, created_at_ms, updated_at_ms
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&project)
        .bind(&task)
        .bind(mapping.repository_id())
        .bind(mapping.worktree_id())
        .bind(mapping.branch())
        .bind(mapping.agent_run_id().to_string())
        .bind(mapping.pull_request_id())
        .bind(mapping.correlation_id().to_string())
        .bind(mapping.policy_revision())
        .bind(state_to_str(mapping.state()))
        .bind(1_i64)
        .bind(observation.map(MappingObservation::repository_id))
        .bind(observation.map(MappingObservation::worktree_id))
        .bind(observation.map(MappingObservation::branch))
        .bind(observed_at_ms)
        .bind(observation.map(|value| value.correlation_id().to_string()))
        .bind(mapping.reconcile_reason())
        .bind(last_reconciled_at_ms)
        .bind(last_resumed_at_ms)
        .bind(now)
        .bind(now)
        .execute(&mut *transaction)
        .await;

        match result {
            Ok(_) => transaction
                .commit()
                .await
                .map_err(|_| TaskMappingPersistenceError::Query),
            Err(error) if is_unique_violation(&error) => {
                Err(TaskMappingPersistenceError::Duplicate)
            }
            Err(_) => Err(TaskMappingPersistenceError::Query),
        }
    }

    pub async fn get(
        &self,
        project_id: ProjectId,
        task_id: TaskId,
    ) -> Result<Option<TaskWorkspaceMapping>, TaskMappingPersistenceError> {
        let row = sqlx::query(
            "SELECT project_id, task_id, repository_id, worktree_id, branch, agent_run_id,
                    pull_request_id, correlation_id, policy_revision, state, revision,
                    observed_repository_id, observed_worktree_id, observed_branch,
                    observed_at_ms, observed_correlation_id, reconcile_reason,
                    last_reconciled_at_ms, last_resumed_at_ms
             FROM task_workspace_mappings WHERE project_id = ? AND task_id = ?",
        )
        .bind(project_id.to_string())
        .bind(task_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| TaskMappingPersistenceError::Query)?;

        row.map(decode_mapping).transpose()
    }

    pub async fn list(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<TaskWorkspaceMapping>, TaskMappingPersistenceError> {
        let rows = sqlx::query(
            "SELECT project_id, task_id, repository_id, worktree_id, branch, agent_run_id,
                    pull_request_id, correlation_id, policy_revision, state, revision,
                    observed_repository_id, observed_worktree_id, observed_branch,
                    observed_at_ms, observed_correlation_id, reconcile_reason,
                    last_reconciled_at_ms, last_resumed_at_ms
             FROM task_workspace_mappings WHERE project_id = ? ORDER BY task_id LIMIT ?",
        )
        .bind(project_id.to_string())
        .bind(i64::from(MAX_LIST_LIMIT))
        .fetch_all(&self.pool)
        .await
        .map_err(|_| TaskMappingPersistenceError::Query)?;

        rows.into_iter().map(decode_mapping).collect()
    }

    pub async fn update(
        &self,
        mapping: &TaskWorkspaceMapping,
        expected_revision: u64,
        now_ms: u64,
    ) -> Result<(), TaskMappingPersistenceError> {
        if mapping.revision() != expected_revision.saturating_add(1) {
            return Err(TaskMappingPersistenceError::Conflict);
        }
        let now = to_i64(now_ms)?;
        let observation = mapping.observation();
        let observed_at_ms = observation
            .map(|value| to_i64(value.observed_at_ms()))
            .transpose()?;
        let last_reconciled_at_ms = mapping.last_reconciled_at_ms().map(to_i64).transpose()?;
        let last_resumed_at_ms = mapping.last_resumed_at_ms().map(to_i64).transpose()?;
        let result = sqlx::query(
            "UPDATE task_workspace_mappings SET
                repository_id = ?, worktree_id = ?, branch = ?, agent_run_id = ?,
                pull_request_id = ?, correlation_id = ?, policy_revision = ?, state = ?,
                revision = ?, observed_repository_id = ?, observed_worktree_id = ?,
                observed_branch = ?, observed_at_ms = ?, observed_correlation_id = ?,
                reconcile_reason = ?, last_reconciled_at_ms = ?, last_resumed_at_ms = ?,
                updated_at_ms = ?
             WHERE project_id = ? AND task_id = ? AND revision = ?",
        )
        .bind(mapping.repository_id())
        .bind(mapping.worktree_id())
        .bind(mapping.branch())
        .bind(mapping.agent_run_id().to_string())
        .bind(mapping.pull_request_id())
        .bind(mapping.correlation_id().to_string())
        .bind(mapping.policy_revision())
        .bind(state_to_str(mapping.state()))
        .bind(to_i64(mapping.revision())?)
        .bind(observation.map(MappingObservation::repository_id))
        .bind(observation.map(MappingObservation::worktree_id))
        .bind(observation.map(MappingObservation::branch))
        .bind(observed_at_ms)
        .bind(observation.map(|value| value.correlation_id().to_string()))
        .bind(mapping.reconcile_reason())
        .bind(last_reconciled_at_ms)
        .bind(last_resumed_at_ms)
        .bind(now)
        .bind(mapping.project_id().to_string())
        .bind(mapping.task_id().to_string())
        .bind(to_i64(expected_revision)?)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                TaskMappingPersistenceError::Duplicate
            } else {
                TaskMappingPersistenceError::Query
            }
        })?;

        if result.rows_affected() == 1 {
            return Ok(());
        }

        match self.get(mapping.project_id(), mapping.task_id()).await? {
            Some(_) => Err(TaskMappingPersistenceError::Conflict),
            None => Err(TaskMappingPersistenceError::NotFound),
        }
    }
}

fn decode_mapping(
    row: sqlx::sqlite::SqliteRow,
) -> Result<TaskWorkspaceMapping, TaskMappingPersistenceError> {
    let observation_values = (
        row.try_get::<Option<String>, _>("observed_repository_id"),
        row.try_get::<Option<String>, _>("observed_worktree_id"),
        row.try_get::<Option<String>, _>("observed_branch"),
        row.try_get::<Option<i64>, _>("observed_at_ms"),
        row.try_get::<Option<String>, _>("observed_correlation_id"),
    );
    let (
        observed_repository_id,
        observed_worktree_id,
        observed_branch,
        observed_at_ms,
        observed_correlation_id,
    ) = observation_values;
    let observation_values = [
        observed_repository_id.map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        observed_worktree_id.map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        observed_branch.map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        observed_at_ms
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?
            .map(|value| value.to_string()),
        observed_correlation_id.map_err(|_| TaskMappingPersistenceError::Corrupt)?,
    ];
    let observation = if observation_values.iter().all(Option::is_none) {
        None
    } else if observation_values.iter().all(Option::is_some) {
        let observed_at_ms = observation_values[3]
            .as_deref()
            .ok_or(TaskMappingPersistenceError::Corrupt)?
            .parse::<i64>()
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?;
        Some(
            MappingObservation::new(
                observation_values[0]
                    .as_deref()
                    .ok_or(TaskMappingPersistenceError::Corrupt)?,
                observation_values[1]
                    .as_deref()
                    .ok_or(TaskMappingPersistenceError::Corrupt)?,
                observation_values[2]
                    .as_deref()
                    .ok_or(TaskMappingPersistenceError::Corrupt)?,
                to_u64(observed_at_ms)?,
                parse_id::<TraceId>(
                    observation_values[4]
                        .as_deref()
                        .ok_or(TaskMappingPersistenceError::Corrupt)?,
                )?,
            )
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        )
    } else {
        return Err(TaskMappingPersistenceError::Corrupt);
    };

    let state = match row
        .try_get::<String, _>("state")
        .map_err(|_| TaskMappingPersistenceError::Corrupt)?
        .as_str()
    {
        "active" => MappingState::Active,
        "detached" => MappingState::Detached,
        "reconcile_required" => MappingState::ReconcileRequired,
        "released" => MappingState::Released,
        _ => return Err(TaskMappingPersistenceError::Corrupt),
    };

    TaskWorkspaceMapping::restore(
        parse_id::<ProjectId>(&row_string(&row, "project_id")?)?,
        parse_id::<TaskId>(&row_string(&row, "task_id")?)?,
        row_string(&row, "repository_id")?,
        row_string(&row, "worktree_id")?,
        row_string(&row, "branch")?,
        parse_id(&row_string(&row, "agent_run_id")?)?,
        row.try_get("pull_request_id")
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        parse_id(&row_string(&row, "correlation_id")?)?,
        row_string(&row, "policy_revision")?,
        state,
        to_u64(
            row.try_get("revision")
                .map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        )?,
        observation,
        row.try_get("reconcile_reason")
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?,
        row.try_get::<Option<i64>, _>("last_reconciled_at_ms")
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?
            .map(to_u64)
            .transpose()?,
        row.try_get::<Option<i64>, _>("last_resumed_at_ms")
            .map_err(|_| TaskMappingPersistenceError::Corrupt)?
            .map(to_u64)
            .transpose()?,
    )
    .map_err(|_| TaskMappingPersistenceError::Corrupt)
}

fn row_string(
    row: &sqlx::sqlite::SqliteRow,
    column: &str,
) -> Result<String, TaskMappingPersistenceError> {
    row.try_get(column)
        .map_err(|_| TaskMappingPersistenceError::Corrupt)
}

fn parse_id<T>(value: &str) -> Result<T, TaskMappingPersistenceError>
where
    T: FromStr,
{
    value
        .parse::<T>()
        .map_err(|_| TaskMappingPersistenceError::Corrupt)
}

fn state_to_str(state: MappingState) -> &'static str {
    match state {
        MappingState::Active => "active",
        MappingState::Detached => "detached",
        MappingState::ReconcileRequired => "reconcile_required",
        MappingState::Released => "released",
    }
}

fn to_i64(value: u64) -> Result<i64, TaskMappingPersistenceError> {
    i64::try_from(value).map_err(|_| TaskMappingPersistenceError::Timestamp)
}

fn to_u64(value: i64) -> Result<u64, TaskMappingPersistenceError> {
    u64::try_from(value).map_err(|_| TaskMappingPersistenceError::Corrupt)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|database| database.is_unique_violation())
}
