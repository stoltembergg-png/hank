use agent_core::{
    ids::ProjectId,
    project::{ProjectRepository, ProjectStatus},
};
use agent_runtime::{
    scheduler::{JobError, JobStore, JobTarget, MissedRunPolicy, ScheduledJob, Trigger},
    sqlite::SqliteStorage,
    SqliteProjectRepository,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

const MAX_PAGE: u32 = 100;

#[derive(Clone)]
pub struct SchedulerBridgeState {
    store: JobStore,
    projects: Arc<SqliteProjectRepository>,
}

pub fn bridge_state(storage: &SqliteStorage) -> SchedulerBridgeState {
    SchedulerBridgeState {
        store: JobStore::new(storage.pool().clone()),
        projects: Arc::new(SqliteProjectRepository::new(storage.pool().clone())),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SchedulerBridgeError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for SchedulerBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for SchedulerBridgeError {}

fn error(code: &'static str, message: impl Into<String>) -> SchedulerBridgeError {
    SchedulerBridgeError {
        code,
        message: message.into(),
    }
}
fn map_error(e: JobError) -> SchedulerBridgeError {
    error("SCHEDULER_JOB_ERROR", e.to_string())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TriggerInput {
    OneShot { at_ms: u64 },
    Interval { seconds: u64 },
    Cron { expression: String },
    Event { name: String },
    Dependency { job_id: String },
}
impl From<TriggerInput> for Trigger {
    fn from(v: TriggerInput) -> Self {
        match v {
            TriggerInput::OneShot { at_ms } => Self::OneShot { at_ms },
            TriggerInput::Interval { seconds } => Self::Interval { seconds },
            TriggerInput::Cron { expression } => Self::Cron { expression },
            TriggerInput::Event { name } => Self::Event { name },
            TriggerInput::Dependency { job_id } => Self::Dependency { job_id },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetInput {
    Workflow { id: String, version: u32 },
    Agent { id: String, version: u32 },
    Tool { id: String, version: u32 },
}
impl From<TargetInput> for JobTarget {
    fn from(v: TargetInput) -> Self {
        match v {
            TargetInput::Workflow { id, version } => Self::Workflow {
                workflow_id: id,
                version,
            },
            TargetInput::Agent { id, version } => Self::Agent {
                agent_id: id,
                version,
            },
            TargetInput::Tool { id, version } => Self::Tool {
                tool_id: id,
                version,
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduledJobInput {
    pub project_id: String,
    pub owner_id: String,
    pub job_id: String,
    pub trigger: TriggerInput,
    pub target: TargetInput,
    pub timezone: String,
    pub concurrency_limit: u32,
    pub missed_run_policy: MissedRunPolicyInput,
    pub enabled: Option<bool>,
    pub lifecycle: Option<String>,
    pub expires_at_ms: Option<u64>,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicyInput {
    Skip,
    CatchUp,
    Pause,
}
impl From<MissedRunPolicyInput> for MissedRunPolicy {
    fn from(v: MissedRunPolicyInput) -> Self {
        match v {
            MissedRunPolicyInput::Skip => Self::Skip,
            MissedRunPolicyInput::CatchUp => Self::CatchUp,
            MissedRunPolicyInput::Pause => Self::Pause,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListScheduledJobsInput {
    pub project_id: String,
    pub owner_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateScheduledJobInput {
    pub job: ScheduledJobInput,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduledJobView {
    pub project_id: String,
    pub job_id: String,
    pub owner_id: String,
    pub trigger_kind: String,
    pub trigger_value: String,
    pub target_kind: String,
    pub target_id: String,
    pub target_version: u32,
    pub timezone: String,
    pub concurrency_limit: u32,
    pub missed_run_policy: String,
    pub enabled: bool,
    pub lifecycle: String,
    pub revision: u64,
    pub expires_at_ms: Option<u64>,
}
impl From<ScheduledJob> for ScheduledJobView {
    fn from(job: ScheduledJob) -> Self {
        let (trigger_kind, trigger_value) = match job.trigger {
            Trigger::OneShot { at_ms } => ("one_shot", at_ms.to_string()),
            Trigger::Interval { seconds } => ("interval", seconds.to_string()),
            Trigger::Cron { expression } => ("cron", expression),
            Trigger::Event { name } => ("event", name),
            Trigger::Dependency { job_id } => ("dependency", job_id),
        };
        let (target_kind, target_id, target_version) = match job.target {
            JobTarget::Workflow {
                workflow_id,
                version,
            } => ("workflow", workflow_id, version),
            JobTarget::Agent { agent_id, version } => ("agent", agent_id, version),
            JobTarget::Tool { tool_id, version } => ("tool", tool_id, version),
        };
        Self {
            project_id: job.project_id,
            job_id: job.job_id,
            owner_id: job.owner_id,
            trigger_kind: trigger_kind.into(),
            trigger_value,
            target_kind: target_kind.into(),
            target_id,
            target_version,
            timezone: job.timezone,
            concurrency_limit: job.concurrency_limit,
            missed_run_policy: match job.missed_run_policy {
                MissedRunPolicy::Skip => "skip",
                MissedRunPolicy::CatchUp => "catch_up",
                MissedRunPolicy::Pause => "pause",
            }
            .into(),
            enabled: job.enabled,
            lifecycle: job.lifecycle,
            revision: job.revision,
            expires_at_ms: job.expires_at_ms,
        }
    }
}

async fn authorize(
    state: &SchedulerBridgeState,
    project: &str,
    owner: &str,
) -> Result<(), SchedulerBridgeError> {
    let project_id: ProjectId = project
        .parse()
        .map_err(|_| error("SCHEDULER_UNAUTHORIZED", "project identity is invalid"))?;
    let record = state
        .projects
        .get_by_id(&project_id)
        .await
        .map_err(|_| {
            error(
                "SCHEDULER_AUTH_ERROR",
                "project authorization lookup failed",
            )
        })?
        .ok_or_else(|| error("SCHEDULER_UNAUTHORIZED", "active project was not found"))?;
    if record.status != ProjectStatus::Active || record.owner != owner {
        return Err(error(
            "SCHEDULER_UNAUTHORIZED",
            "owner is not authorized for this project",
        ));
    }
    Ok(())
}

fn build_job(input: ScheduledJobInput) -> Result<ScheduledJob, SchedulerBridgeError> {
    let mut job = ScheduledJob::new(
        &input.project_id,
        &input.job_id,
        &input.owner_id,
        input.trigger.into(),
        input.target.into(),
        &input.timezone,
        input.concurrency_limit,
        input.missed_run_policy.into(),
    )
    .map_err(map_error)?;
    if let Some(enabled) = input.enabled {
        job.enabled = enabled;
    }
    if let Some(lifecycle) = input.lifecycle {
        job.lifecycle = lifecycle;
    }
    if let Some(expires) = input.expires_at_ms {
        job = job.with_expiration(expires).map_err(map_error)?;
    }
    Ok(job)
}

#[tauri::command]
pub async fn list_scheduled_jobs(
    state: State<'_, SchedulerBridgeState>,
    input: ListScheduledJobsInput,
) -> Result<Vec<ScheduledJobView>, SchedulerBridgeError> {
    authorize(&state, &input.project_id, &input.owner_id).await?;
    let jobs = state
        .store
        .list(
            &input.project_id,
            input.limit.unwrap_or(50).min(MAX_PAGE),
            input.offset.unwrap_or(0),
        )
        .await
        .map_err(map_error)?;
    Ok(jobs.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn create_scheduled_job(
    state: State<'_, SchedulerBridgeState>,
    input: ScheduledJobInput,
) -> Result<ScheduledJobView, SchedulerBridgeError> {
    authorize(&state, &input.project_id, &input.owner_id).await?;
    let job = build_job(input)?;
    state.store.create(job.clone()).await.map_err(map_error)?;
    Ok(job.into())
}

#[tauri::command]
pub async fn update_scheduled_job(
    state: State<'_, SchedulerBridgeState>,
    input: UpdateScheduledJobInput,
) -> Result<ScheduledJobView, SchedulerBridgeError> {
    authorize(&state, &input.job.project_id, &input.job.owner_id).await?;
    let job = build_job(input.job)?;
    let updated = state
        .store
        .update(job, input.expected_revision)
        .await
        .map_err(map_error)?;
    Ok(updated.into())
}

pub fn command_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        list_scheduled_jobs,
        create_scheduled_job,
        update_scheduled_job
    ]
}
