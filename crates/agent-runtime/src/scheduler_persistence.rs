use sqlx::{Pool, QueryBuilder, Row, Sqlite};
use thiserror::Error;

const MAX_ID: usize = 128;
const MAX_OUTCOME: usize = 64;
const MAX_HISTORY_PAGE: u32 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerRun {
    pub project_id: String,
    pub run_id: String,
    pub job_id: String,
    pub due_at_ms: u64,
    pub status: String,
    pub lease_owner: Option<String>,
    pub lease_expires_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissedOutcomeRecord {
    pub outcome_id: String,
    pub run_id: String,
    pub occurrence_at_ms: u64,
    pub action: String,
    pub reason: String,
    pub coalesce_key: Option<String>,
    pub policy_version: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchedulerHistoryQuery<'a> {
    pub job_id: Option<&'a str>,
    pub status: Option<&'a str>,
    pub from_due_at_ms: Option<u64>,
    pub to_due_at_ms: Option<u64>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerHistoryEntry {
    pub project_id: String,
    pub run_id: String,
    pub job_id: String,
    pub due_at_ms: u64,
    pub status: String,
    pub completed_at_ms: Option<u64>,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PersistenceError {
    #[error("scheduler persistence identity is invalid")]
    InvalidIdentity,
    #[error("scheduler run was not found")]
    NotFound,
    #[error("scheduler run is already terminal")]
    Terminal,
    #[error("scheduler run claim was not acquired")]
    NotClaimed,
    #[error("scheduler history pagination is invalid")]
    InvalidPagination,
    #[error("scheduler run storage query failed")]
    Query,
}

#[derive(Clone)]
pub struct SchedulerPersistence {
    pool: Pool<Sqlite>,
}

impl SchedulerPersistence {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create_run(
        &self,
        project: &str,
        run_id: &str,
        job_id: &str,
        due_at_ms: u64,
    ) -> Result<(), PersistenceError> {
        for value in [project, run_id, job_id] {
            validate_id(value)?;
        }
        let due = i64::try_from(due_at_ms).map_err(|_| PersistenceError::InvalidIdentity)?;
        sqlx::query("INSERT INTO scheduler_runs (project_id, run_id, job_id, due_at_ms, status, created_at_ms, updated_at_ms) VALUES (?, ?, ?, ?, 'pending', ?, ?)")
            .bind(project).bind(run_id).bind(job_id).bind(due).bind(due).bind(due)
            .execute(&self.pool).await.map_err(|error| if error.as_database_error().is_some_and(|db| db.is_unique_violation()) { PersistenceError::Terminal } else { PersistenceError::Query })?;
        Ok(())
    }

    pub async fn claim(
        &self,
        project: &str,
        run_id: &str,
        lease_owner: &str,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<SchedulerRun, PersistenceError> {
        for value in [project, run_id, lease_owner] {
            validate_id(value)?;
        }
        let now = i64::try_from(now_ms).map_err(|_| PersistenceError::InvalidIdentity)?;
        let lease = now
            .checked_add(
                i64::try_from(lease_duration_ms).map_err(|_| PersistenceError::InvalidIdentity)?,
            )
            .ok_or(PersistenceError::InvalidIdentity)?;
        let result = sqlx::query("UPDATE scheduler_runs SET status='claimed', lease_owner=?, lease_expires_at_ms=?, updated_at_ms=? WHERE project_id=? AND run_id=? AND (status='pending' OR (status='claimed' AND lease_expires_at_ms <= ?)) AND due_at_ms <= ?")
            .bind(lease_owner)
            .bind(lease)
            .bind(now)
            .bind(project)
            .bind(run_id)
            .bind(now)
            .bind(now)
            .execute(&self.pool).await.map_err(|_| PersistenceError::Query)?;
        if result.rows_affected() != 1 {
            let row = self.row(project, run_id).await?;
            return match row.status.as_str() {
                "completed" => Err(PersistenceError::Terminal),
                _ => Err(PersistenceError::NotClaimed),
            };
        }
        self.row(project, run_id).await
    }

    pub async fn claim_next_due(
        &self,
        project: &str,
        lease_owner: &str,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<Option<SchedulerRun>, PersistenceError> {
        validate_id(project)?;
        let now = i64::try_from(now_ms).map_err(|_| PersistenceError::InvalidIdentity)?;
        let row = sqlx::query("SELECT run_id FROM scheduler_runs WHERE project_id=? AND due_at_ms <= ? AND (status='pending' OR (status='claimed' AND lease_expires_at_ms <= ?)) ORDER BY due_at_ms, run_id LIMIT 1")
            .bind(project).bind(now).bind(now).fetch_optional(&self.pool).await.map_err(|_| PersistenceError::Query)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let run_id: String = row.get("run_id");
        self.claim(project, &run_id, lease_owner, now_ms, lease_duration_ms)
            .await
            .map(Some)
    }

    pub async fn renew(
        &self,
        project: &str,
        run_id: &str,
        lease_owner: &str,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<SchedulerRun, PersistenceError> {
        for value in [project, run_id, lease_owner] {
            validate_id(value)?;
        }
        let now = i64::try_from(now_ms).map_err(|_| PersistenceError::InvalidIdentity)?;
        let lease = now
            .checked_add(
                i64::try_from(lease_duration_ms).map_err(|_| PersistenceError::InvalidIdentity)?,
            )
            .ok_or(PersistenceError::InvalidIdentity)?;
        let result = sqlx::query("UPDATE scheduler_runs SET lease_expires_at_ms=?, updated_at_ms=? WHERE project_id=? AND run_id=? AND status='claimed' AND lease_owner=? AND lease_expires_at_ms > ?")
            .bind(lease).bind(now).bind(project).bind(run_id).bind(lease_owner).bind(now).execute(&self.pool).await.map_err(|_| PersistenceError::Query)?;
        if result.rows_affected() != 1 {
            return Err(PersistenceError::NotClaimed);
        }
        self.row(project, run_id).await
    }
    pub async fn complete(
        &self,
        project: &str,
        run_id: &str,
        lease_owner: &str,
        outcome: &str,
        completed_at_ms: u64,
    ) -> Result<SchedulerRun, PersistenceError> {
        for value in [project, run_id, lease_owner] {
            validate_id(value)?;
        }
        if outcome.is_empty()
            || outcome.len() > MAX_OUTCOME
            || outcome.chars().any(char::is_control)
        {
            return Err(PersistenceError::InvalidIdentity);
        }
        let completed =
            i64::try_from(completed_at_ms).map_err(|_| PersistenceError::InvalidIdentity)?;
        let result = sqlx::query("UPDATE scheduler_runs SET status='completed', completed_at_ms=?, outcome=?, updated_at_ms=? WHERE project_id=? AND run_id=? AND status='claimed' AND lease_owner=?")
            .bind(completed).bind(outcome).bind(completed).bind(project).bind(run_id).bind(lease_owner)
            .execute(&self.pool).await.map_err(|_| PersistenceError::Query)?;
        if result.rows_affected() != 1 {
            return Err(PersistenceError::NotClaimed);
        }
        self.row(project, run_id).await
    }

    pub async fn list_history(
        &self,
        project: &str,
        query: &SchedulerHistoryQuery<'_>,
    ) -> Result<Vec<SchedulerHistoryEntry>, PersistenceError> {
        validate_id(project)?;
        if query.limit == 0 || query.limit > MAX_HISTORY_PAGE {
            return Err(PersistenceError::InvalidPagination);
        }
        if let Some(job_id) = query.job_id {
            validate_id(job_id)?;
        }
        if let Some(status) = query.status {
            validate_id(status)?;
        }
        if let (Some(from), Some(to)) = (query.from_due_at_ms, query.to_due_at_ms) {
            if from > to {
                return Err(PersistenceError::InvalidPagination);
            }
        }
        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT project_id, run_id, job_id, due_at_ms, status, completed_at_ms, outcome FROM scheduler_runs WHERE project_id = ",
        );
        builder.push_bind(project);
        if let Some(job_id) = query.job_id {
            builder.push(" AND job_id = ").push_bind(job_id);
        }
        if let Some(status) = query.status {
            builder.push(" AND status = ").push_bind(status);
        }
        if let Some(from) = query.from_due_at_ms {
            builder.push(" AND due_at_ms >= ").push_bind(to_i64(from)?);
        }
        if let Some(to) = query.to_due_at_ms {
            builder.push(" AND due_at_ms <= ").push_bind(to_i64(to)?);
        }
        builder
            .push(" ORDER BY due_at_ms ASC, run_id ASC LIMIT ")
            .push_bind(i64::from(query.limit))
            .push(" OFFSET ")
            .push_bind(i64::from(query.offset));
        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|_| PersistenceError::Query)?;
        rows.into_iter().map(history_entry).collect()
    }

    pub async fn prune_completed(
        &self,
        project: &str,
        completed_before_ms: u64,
        limit: u32,
    ) -> Result<u64, PersistenceError> {
        validate_id(project)?;
        if limit == 0 || limit > MAX_HISTORY_PAGE {
            return Err(PersistenceError::InvalidPagination);
        }
        let cutoff = to_i64(completed_before_ms)?;
        let result = sqlx::query(
            "DELETE FROM scheduler_runs WHERE project_id = ? AND run_id IN (SELECT run_id FROM scheduler_runs WHERE project_id = ? AND status = 'completed' AND completed_at_ms IS NOT NULL AND completed_at_ms < ? ORDER BY completed_at_ms ASC, run_id ASC LIMIT ?)",
        )
        .bind(project)
        .bind(project)
        .bind(cutoff)
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await
        .map_err(|_| PersistenceError::Query)?;
        Ok(result.rows_affected())
    }

    pub async fn record_missed_outcome(
        &self,
        project: &str,
        record: &MissedOutcomeRecord,
        created_at_ms: u64,
    ) -> Result<String, PersistenceError> {
        for value in [
            project,
            record.outcome_id.as_str(),
            record.run_id.as_str(),
            record.action.as_str(),
            record.reason.as_str(),
            record.policy_version.as_str(),
        ] {
            validate_id(value)?;
        }
        let occurrence = i64::try_from(record.occurrence_at_ms)
            .map_err(|_| PersistenceError::InvalidIdentity)?;
        let created =
            i64::try_from(created_at_ms).map_err(|_| PersistenceError::InvalidIdentity)?;
        let existing = sqlx::query("SELECT outcome_id FROM scheduler_missed_outcomes WHERE project_id=? AND run_id=? AND occurrence_at_ms=? AND action=?")
            .bind(project).bind(&record.run_id).bind(occurrence).bind(&record.action).fetch_optional(&self.pool).await.map_err(|_| PersistenceError::Query)?;
        if let Some(row) = existing {
            return Ok(row.get("outcome_id"));
        }
        self.row(project, &record.run_id).await?;
        sqlx::query("INSERT INTO scheduler_missed_outcomes (project_id, outcome_id, run_id, occurrence_at_ms, action, reason, coalesce_key, policy_version, created_at_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(project).bind(&record.outcome_id).bind(&record.run_id).bind(occurrence).bind(&record.action).bind(&record.reason).bind(&record.coalesce_key).bind(&record.policy_version).bind(created)
            .execute(&self.pool).await.map_err(|_| PersistenceError::Query)?;
        Ok(record.outcome_id.clone())
    }
    pub async fn get_run(
        &self,
        project: &str,
        run_id: &str,
    ) -> Result<SchedulerRun, PersistenceError> {
        validate_id(project)?;
        validate_id(run_id)?;
        self.row(project, run_id).await
    }
    async fn row(&self, project: &str, run_id: &str) -> Result<SchedulerRun, PersistenceError> {
        let row = sqlx::query("SELECT project_id, run_id, job_id, due_at_ms, status, lease_owner, lease_expires_at_ms, completed_at_ms, outcome FROM scheduler_runs WHERE project_id=? AND run_id=?")
            .bind(project).bind(run_id).fetch_optional(&self.pool).await.map_err(|_| PersistenceError::Query)?.ok_or(PersistenceError::NotFound)?;
        Ok(SchedulerRun {
            project_id: row.get("project_id"),
            run_id: row.get("run_id"),
            job_id: row.get("job_id"),
            due_at_ms: decode_u64(row.get("due_at_ms"))?,
            status: row.get("status"),
            lease_owner: row.get("lease_owner"),
            lease_expires_at_ms: row
                .get::<Option<i64>, _>("lease_expires_at_ms")
                .map(decode_u64)
                .transpose()?,
            completed_at_ms: row
                .get::<Option<i64>, _>("completed_at_ms")
                .map(decode_u64)
                .transpose()?,
            outcome: row.get("outcome"),
        })
    }
}

fn history_entry(row: sqlx::sqlite::SqliteRow) -> Result<SchedulerHistoryEntry, PersistenceError> {
    Ok(SchedulerHistoryEntry {
        project_id: row.get("project_id"),
        run_id: row.get("run_id"),
        job_id: row.get("job_id"),
        due_at_ms: decode_u64(row.get("due_at_ms"))?,
        status: row.get("status"),
        completed_at_ms: row
            .get::<Option<i64>, _>("completed_at_ms")
            .map(decode_u64)
            .transpose()?,
        outcome: row.get("outcome"),
    })
}

fn to_i64(value: u64) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::InvalidIdentity)
}

fn validate_id(value: &str) -> Result<(), PersistenceError> {
    if value.is_empty() || value.len() > MAX_ID || value.chars().any(char::is_control) {
        Err(PersistenceError::InvalidIdentity)
    } else {
        Ok(())
    }
}
fn decode_u64(value: i64) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| PersistenceError::Query)
}
