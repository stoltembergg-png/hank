use sqlx::{Pool, Row, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MAX_ID_BYTES: usize = 128;
const MAX_REPORT_CANDIDATES: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStatus {
    Pending,
    Committed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub generation: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCandidate {
    pub run_id: String,
    pub previous_generation: u64,
    pub new_generation: u64,
    pub status: RecoveryStatus,
    pub requires_reconcile: bool,
    pub executed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub candidates: Vec<RecoveryCandidate>,
    pub diagnostics: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LeaseError {
    #[error("recovery identity is invalid")]
    InvalidIdentity,
    #[error("recovery project scope is invalid")]
    ProjectScope,
    #[error("workflow run was not found")]
    NotFound,
    #[error("workflow lease is held by another runner")]
    Busy,
    #[error("recovery query failed")]
    Query,
    #[error("recovery serialization failed")]
    Serialization,
    #[error("recovery budget is invalid")]
    Budget,
}

#[derive(Clone)]
pub struct RecoveryStore {
    pool: Pool<Sqlite>,
}

impl RecoveryStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn acquire_lease(
        &self,
        project_id: &str,
        run_id: &str,
        runner_id: &str,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<Lease, LeaseError> {
        validate_id(project_id)?;
        validate_id(run_id)?;
        validate_id(runner_id)?;
        if ttl_ms == 0 {
            return Err(LeaseError::Budget);
        }
        let expiry = now_ms.checked_add(ttl_ms).ok_or(LeaseError::Budget)?;
        let result = sqlx::query("UPDATE workflow_runs SET lease_owner = ?, lease_expires_at_ms = ?, generation = generation + 1, updated_at_ms = ? WHERE project_id = ? AND run_id = ? AND (lease_owner IS NULL OR lease_expires_at_ms <= ? OR lease_owner = ?)")
            .bind(runner_id).bind(i64::try_from(expiry).map_err(|_| LeaseError::Serialization)?).bind(i64::try_from(now_ms).map_err(|_| LeaseError::Serialization)?).bind(project_id).bind(run_id).bind(i64::try_from(now_ms).map_err(|_| LeaseError::Serialization)?).bind(runner_id).execute(&self.pool).await.map_err(|_| LeaseError::Query)?;
        if result.rows_affected() == 0 {
            let exists =
                sqlx::query("SELECT 1 FROM workflow_runs WHERE project_id = ? AND run_id = ?")
                    .bind(project_id)
                    .bind(run_id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|_| LeaseError::Query)?;
            return Err(if exists.is_some() {
                LeaseError::Busy
            } else {
                LeaseError::NotFound
            });
        }
        let row =
            sqlx::query("SELECT generation FROM workflow_runs WHERE project_id = ? AND run_id = ?")
                .bind(project_id)
                .bind(run_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|_| LeaseError::Query)?;
        Ok(Lease {
            generation: to_u64(row.get("generation"))?,
            expires_at_ms: expiry,
        })
    }

    pub async fn fence(
        &self,
        project_id: &str,
        run_id: &str,
        runner_id: &str,
        generation: u64,
        now_ms: u64,
    ) -> Result<bool, LeaseError> {
        validate_id(project_id)?;
        validate_id(run_id)?;
        validate_id(runner_id)?;
        let row = sqlx::query("SELECT 1 FROM workflow_runs WHERE project_id = ? AND run_id = ? AND lease_owner = ? AND generation = ? AND lease_expires_at_ms > ?")
            .bind(project_id).bind(run_id).bind(runner_id).bind(i64::try_from(generation).map_err(|_| LeaseError::Serialization)?).bind(i64::try_from(now_ms).map_err(|_| LeaseError::Serialization)?).fetch_optional(&self.pool).await.map_err(|_| LeaseError::Query)?;
        Ok(row.is_some())
    }

    pub async fn recover_expired(
        &self,
        project_id: &str,
        runner_id: &str,
        now_ms: u64,
        ttl_ms: u64,
        max_candidates: u32,
    ) -> Result<RecoveryReport, LeaseError> {
        validate_id(project_id)?;
        validate_id(runner_id)?;
        if max_candidates == 0 || max_candidates > MAX_REPORT_CANDIDATES {
            return Err(LeaseError::Budget);
        }
        let project = sqlx::query("SELECT 1 FROM projects WHERE id = ?")
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| LeaseError::Query)?;
        if project.is_none() {
            return Err(LeaseError::ProjectScope);
        }
        let rows = sqlx::query("SELECT run_id, generation FROM workflow_runs WHERE project_id = ? AND lease_owner IS NOT NULL AND lease_expires_at_ms <= ? ORDER BY run_id LIMIT ?")
            .bind(project_id).bind(i64::try_from(now_ms).map_err(|_| LeaseError::Serialization)?).bind(i64::from(max_candidates)).fetch_all(&self.pool).await.map_err(|_| LeaseError::Query)?;
        let mut candidates = Vec::with_capacity(rows.len());
        for row in rows {
            let run_id: String = row.get("run_id");
            let previous_generation = to_u64(row.get("generation"))?;
            let lease = self
                .acquire_lease(project_id, &run_id, runner_id, now_ms, ttl_ms)
                .await?;
            let result = sqlx::query("UPDATE workflow_node_states SET recovery_class = 'unknown', unknown_effect = 1, generation = ?, updated_at_ms = ? WHERE project_id = ? AND run_id = ? AND state = 'running'")
                .bind(i64::try_from(lease.generation).map_err(|_| LeaseError::Serialization)?).bind(i64::try_from(now_ms).map_err(|_| LeaseError::Serialization)?).bind(project_id).bind(&run_id).execute(&self.pool).await.map_err(|_| LeaseError::Query)?;
            if result.rows_affected() > 0 {
                let recovery_id = format!("{}:{}", run_id, lease.generation);
                sqlx::query("INSERT INTO workflow_recovery_reports (project_id, run_id, recovery_id, previous_generation, new_generation, recovery_class, requires_reconcile, created_at_ms) VALUES (?, ?, ?, ?, ?, 'unknown', 1, ?)")
                    .bind(project_id).bind(&run_id).bind(&recovery_id)
                    .bind(i64::try_from(previous_generation).map_err(|_| LeaseError::Serialization)?)
                    .bind(i64::try_from(lease.generation).map_err(|_| LeaseError::Serialization)?)
                    .bind(i64::try_from(now_ms).map_err(|_| LeaseError::Serialization)?)
                    .execute(&self.pool).await.map_err(|_| LeaseError::Query)?;
                candidates.push(RecoveryCandidate {
                    run_id,
                    previous_generation,
                    new_generation: lease.generation,
                    status: RecoveryStatus::Unknown,
                    requires_reconcile: true,
                    executed: false,
                });
            }
        }
        candidates.sort_by(|left, right| left.run_id.cmp(&right.run_id));
        Ok(RecoveryReport {
            candidates,
            diagnostics: "recovery classified expired runs; no capability executed".into(),
        })
    }
}

fn validate_id(value: &str) -> Result<(), LeaseError> {
    if value.trim().is_empty() || value.len() > MAX_ID_BYTES || value.chars().any(char::is_control)
    {
        Err(LeaseError::InvalidIdentity)
    } else {
        Ok(())
    }
}
fn to_u64(value: i64) -> Result<u64, LeaseError> {
    u64::try_from(value).map_err(|_| LeaseError::Serialization)
}
#[allow(dead_code)]
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
