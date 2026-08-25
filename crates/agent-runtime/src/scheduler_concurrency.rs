use sqlx::{Pool, Row, Sqlite};
use thiserror::Error;

const MAX_ID: usize = 128;
const MAX_LIMIT: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub project_id: String,
    pub concurrency_key: String,
    pub run_id: String,
    pub lease_owner: String,
    pub limit: u32,
    pub now_ms: u64,
    pub lease_expires_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AdmissionError {
    #[error("scheduler concurrency identity is invalid")]
    InvalidIdentity,
    #[error("scheduler concurrency limit is invalid")]
    InvalidLimit,
    #[error("scheduler concurrency capacity reached")]
    CapacityReached,
    #[error("scheduler concurrency storage query failed")]
    Query,
}

#[derive(Clone)]
pub struct SchedulerConcurrency {
    pool: Pool<Sqlite>,
}

impl SchedulerConcurrency {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn admit(&self, request: &AdmissionRequest) -> Result<(), AdmissionError> {
        for value in [
            request.project_id.as_str(),
            request.concurrency_key.as_str(),
            request.run_id.as_str(),
            request.lease_owner.as_str(),
        ] {
            validate_id(value)?;
        }
        if request.limit == 0
            || request.limit > MAX_LIMIT
            || request.lease_expires_at_ms <= request.now_ms
        {
            return Err(AdmissionError::InvalidLimit);
        }
        let now = i64::try_from(request.now_ms).map_err(|_| AdmissionError::InvalidIdentity)?;
        let expires = i64::try_from(request.lease_expires_at_ms)
            .map_err(|_| AdmissionError::InvalidIdentity)?;
        let mut tx = self.pool.begin().await.map_err(|_| AdmissionError::Query)?;
        sqlx::query("DELETE FROM scheduler_concurrency_admissions WHERE project_id=? AND concurrency_key=? AND lease_expires_at_ms <= ?")
            .bind(&request.project_id)
            .bind(&request.concurrency_key)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Query)?;
        let existing = sqlx::query("SELECT lease_owner FROM scheduler_concurrency_admissions WHERE project_id=? AND concurrency_key=? AND run_id=?")
            .bind(&request.project_id)
            .bind(&request.concurrency_key)
            .bind(&request.run_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Query)?;
        if existing.is_some() {
            return Ok(());
        }
        let count: i64 = sqlx::query("SELECT COUNT(*) AS count FROM scheduler_concurrency_admissions WHERE project_id=? AND concurrency_key=? AND lease_expires_at_ms > ?")
            .bind(&request.project_id)
            .bind(&request.concurrency_key)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Query)?
            .get("count");
        if count >= i64::from(request.limit) {
            return Err(AdmissionError::CapacityReached);
        }
        sqlx::query("INSERT INTO scheduler_concurrency_admissions (project_id, concurrency_key, run_id, lease_owner, lease_expires_at_ms, admitted_at_ms) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&request.project_id)
            .bind(&request.concurrency_key)
            .bind(&request.run_id)
            .bind(&request.lease_owner)
            .bind(expires)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|_| AdmissionError::Query)?;
        tx.commit().await.map_err(|_| AdmissionError::Query)
    }

    pub async fn release(
        &self,
        project: &str,
        concurrency_key: &str,
        run_id: &str,
        lease_owner: &str,
    ) -> Result<bool, AdmissionError> {
        for value in [project, concurrency_key, run_id, lease_owner] {
            validate_id(value)?;
        }
        let result = sqlx::query("DELETE FROM scheduler_concurrency_admissions WHERE project_id=? AND concurrency_key=? AND run_id=? AND lease_owner=?")
            .bind(project)
            .bind(concurrency_key)
            .bind(run_id)
            .bind(lease_owner)
            .execute(&self.pool)
            .await
            .map_err(|_| AdmissionError::Query)?;
        Ok(result.rows_affected() == 1)
    }
}

fn validate_id(value: &str) -> Result<(), AdmissionError> {
    if value.is_empty() || value.len() > MAX_ID || value.chars().any(char::is_control) {
        Err(AdmissionError::InvalidIdentity)
    } else {
        Ok(())
    }
}
