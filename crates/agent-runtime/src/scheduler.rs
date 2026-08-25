use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use sqlx::{Pool, Row, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MIN_INTERVAL_SECONDS: u64 = 60;
const MAX_TEXT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    OneShot { at_ms: u64 },
    Interval { seconds: u64 },
    Cron { expression: String },
    Event { name: String },
    Dependency { job_id: String },
}
impl Trigger {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::OneShot { .. } => "one_shot",
            Self::Interval { .. } => "interval",
            Self::Cron { .. } => "cron",
            Self::Event { .. } => "event",
            Self::Dependency { .. } => "dependency",
        }
    }
    fn value(&self) -> Result<String, JobError> {
        let value = match self {
            Self::OneShot { at_ms } => at_ms.to_string(),
            Self::Interval { seconds } => seconds.to_string(),
            Self::Cron { expression }
            | Self::Event { name: expression }
            | Self::Dependency { job_id: expression } => expression.clone(),
        };
        if value.is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control) {
            return Err(JobError::InvalidTrigger);
        }
        if matches!(self, Self::OneShot { at_ms: 0 }) {
            return Err(JobError::InvalidTrigger);
        }
        if matches!(self, Self::Interval { seconds } if *seconds < MIN_INTERVAL_SECONDS) {
            return Err(JobError::InvalidFrequency);
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobTarget {
    Workflow { workflow_id: String, version: u32 },
    Agent { agent_id: String, version: u32 },
    Tool { tool_id: String, version: u32 },
}
impl JobTarget {
    fn parts(&self) -> (&'static str, &str, u32) {
        match self {
            Self::Workflow {
                workflow_id,
                version,
            } => ("workflow", workflow_id, *version),
            Self::Agent { agent_id, version } => ("agent", agent_id, *version),
            Self::Tool { tool_id, version } => ("tool", tool_id, *version),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedRunPolicy {
    Skip,
    CatchUp,
    Pause,
}
impl MissedRunPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::CatchUp => "catch_up",
            Self::Pause => "pause",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledJob {
    pub project_id: String,
    pub job_id: String,
    pub owner_id: String,
    pub trigger: Trigger,
    pub target: JobTarget,
    pub timezone: String,
    pub concurrency_limit: u32,
    pub missed_run_policy: MissedRunPolicy,
    pub enabled: bool,
    pub lifecycle: String,
    pub revision: u64,
}
impl ScheduledJob {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: &str,
        job: &str,
        owner: &str,
        trigger: Trigger,
        target: JobTarget,
        timezone: &str,
        concurrency: u32,
        missed: MissedRunPolicy,
    ) -> Result<Self, JobError> {
        for value in [project, job, owner, timezone] {
            validate_text(value)?;
        }
        let (_, target_id, version) = target.parts();
        validate_text(target_id)?;
        if version == 0 || concurrency == 0 || concurrency > 64 {
            return Err(JobError::InvalidBounds);
        }
        trigger.value()?;
        Ok(Self {
            project_id: project.into(),
            job_id: job.into(),
            owner_id: owner.into(),
            trigger,
            target,
            timezone: timezone.into(),
            concurrency_limit: concurrency,
            missed_run_policy: missed,
            enabled: true,
            lifecycle: "active".into(),
            revision: 0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronField {
    values: Vec<u32>,
    wildcard: bool,
}

impl CronField {
    fn parse(input: &str, min: u32, max: u32) -> Result<Self, CronError> {
        if input.is_empty() || input.len() > 64 {
            return Err(CronError::InvalidField);
        }
        let wildcard = input == "*";
        let mut values = Vec::new();
        for item in input.split(',') {
            let (base, step) = item
                .split_once('/')
                .map(|(base, step)| (base, step.parse::<u32>().ok()))
                .unwrap_or((item, Some(1)));
            let step = step
                .filter(|value| *value > 0)
                .ok_or(CronError::InvalidField)?;
            let (start, end) = if base == "*" {
                (min, max)
            } else if let Some((start, end)) = base.split_once('-') {
                (
                    start.parse::<u32>().map_err(|_| CronError::InvalidField)?,
                    end.parse::<u32>().map_err(|_| CronError::InvalidField)?,
                )
            } else {
                let value = base.parse::<u32>().map_err(|_| CronError::InvalidField)?;
                (value, value)
            };
            if start < min || end > max || start > end {
                return Err(CronError::ValueOutOfRange);
            }
            for value in (start..=end).step_by(step as usize) {
                if !values.contains(&value) {
                    values.push(value);
                }
            }
        }
        values.sort_unstable();
        if values.is_empty() || values.len() > 256 {
            return Err(CronError::InvalidField);
        }
        Ok(Self { values, wildcard })
    }
    fn matches(&self, value: u32) -> bool {
        self.values.binary_search(&value).is_ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronSchedule {
    minute: CronField,
    hour: CronField,
    day: CronField,
    month: CronField,
    weekday: CronField,
    timezone: Tz,
}

impl CronSchedule {
    pub fn parse(expression: &str, timezone: &str) -> Result<Self, CronError> {
        if expression.len() > 128 {
            return Err(CronError::InputTooLong);
        }
        let fields: Vec<_> = expression.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(CronError::FieldCount);
        }
        let timezone = timezone
            .parse::<Tz>()
            .map_err(|_| CronError::InvalidTimezone)?;
        let schedule = Self {
            minute: CronField::parse(fields[0], 0, 59)?,
            hour: CronField::parse(fields[1], 0, 23)?,
            day: CronField::parse(fields[2], 1, 31)?,
            month: CronField::parse(fields[3], 1, 12)?,
            weekday: CronField::parse(fields[4], 0, 6)?,
            timezone,
        };
        if schedule.minute.wildcard
            && schedule.hour.wildcard
            && schedule.day.wildcard
            && schedule.month.wildcard
            && schedule.weekday.wildcard
        {
            return Err(CronError::TooFrequent);
        }
        Ok(schedule)
    }

    pub fn next_due_after(&self, now: DateTime<Utc>) -> Result<DateTime<Utc>, CronError> {
        let local = now.with_timezone(&self.timezone).naive_local();
        let mut candidate = local
            .date()
            .and_hms_opt(local.hour(), local.minute(), 0)
            .ok_or(CronError::NoOccurrence)?
            + Duration::minutes(1);
        for _ in 0..(366 * 24 * 60) {
            if self.matches(candidate) {
                match self.timezone.from_local_datetime(&candidate) {
                    LocalResult::Single(value) if value.with_timezone(&Utc) > now => {
                        return Ok(value.with_timezone(&Utc));
                    }
                    LocalResult::Ambiguous(first, second) => {
                        let first = first.with_timezone(&Utc);
                        let second = second.with_timezone(&Utc);
                        if let Some(selected) = [first, second]
                            .into_iter()
                            .filter(|value| *value > now)
                            .min()
                        {
                            return Ok(selected);
                        }
                    }
                    LocalResult::None => {}
                    LocalResult::Single(_) => {}
                }
            }
            candidate = candidate
                .checked_add_signed(Duration::minutes(1))
                .ok_or(CronError::SearchOverflow)?;
        }
        Err(CronError::SearchLimit)
    }

    fn matches(&self, value: NaiveDateTime) -> bool {
        let month = value.month();
        let day = value.day();
        let weekday = value.weekday().num_days_from_sunday();
        let day_matches = self.day.matches(day);
        let weekday_matches = self.weekday.matches(weekday);
        self.minute.matches(value.minute())
            && self.hour.matches(value.hour())
            && self.month.matches(month)
            && ((self.day.wildcard && self.weekday.wildcard)
                || (self.day.wildcard && weekday_matches)
                || (self.weekday.wildcard && day_matches)
                || (day_matches || weekday_matches))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CronError {
    #[error("cron input is too long")]
    InputTooLong,
    #[error("cron must contain exactly five fields")]
    FieldCount,
    #[error("cron field is invalid")]
    InvalidField,
    #[error("cron field value is out of range")]
    ValueOutOfRange,
    #[error("cron timezone is invalid")]
    InvalidTimezone,
    #[error("cron frequency is below the minimum")]
    TooFrequent,
    #[error("cron has no occurrence within the search bound")]
    SearchLimit,
    #[error("cron search arithmetic overflowed")]
    SearchOverflow,
    #[error("cron has no representable occurrence")]
    NoOccurrence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ScheduleError {
    #[error("interval is below the minimum frequency")]
    TooFrequent,
    #[error("interval exceeds the bounded maximum")]
    TooLong,
    #[error("interval arithmetic overflowed")]
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalSchedule {
    anchor_ms: u64,
    interval_ms: u64,
}

impl IntervalSchedule {
    pub fn new(anchor_ms: u64, interval_seconds: u64) -> Result<Self, ScheduleError> {
        if interval_seconds < MIN_INTERVAL_SECONDS {
            return Err(ScheduleError::TooFrequent);
        }
        if interval_seconds > 31 * 24 * 60 * 60 {
            return Err(ScheduleError::TooLong);
        }
        let interval_ms = interval_seconds
            .checked_mul(1_000)
            .ok_or(ScheduleError::Overflow)?;
        Ok(Self {
            anchor_ms,
            interval_ms,
        })
    }

    pub fn next_due(&self, now_ms: u64, enabled: bool) -> Result<Option<u64>, ScheduleError> {
        if !enabled {
            return Ok(None);
        }
        let elapsed = now_ms.saturating_sub(self.anchor_ms);
        let periods = elapsed
            .checked_div(self.interval_ms)
            .and_then(|value| value.checked_add(1))
            .ok_or(ScheduleError::Overflow)?;
        let offset = periods
            .checked_mul(self.interval_ms)
            .ok_or(ScheduleError::Overflow)?;
        self.anchor_ms
            .checked_add(offset)
            .map(Some)
            .ok_or(ScheduleError::Overflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum JobError {
    #[error("scheduler job identity is invalid")]
    InvalidIdentity,
    #[error("scheduler job trigger is invalid")]
    InvalidTrigger,
    #[error("scheduler job frequency is below minimum")]
    InvalidFrequency,
    #[error("scheduler job timezone is invalid")]
    InvalidTimezone,
    #[error("scheduler job bounds are invalid")]
    InvalidBounds,
    #[error("scheduler job is duplicated")]
    Duplicate,
    #[error("scheduler job revision is stale")]
    StaleRevision,
    #[error("scheduler job lifecycle is invalid")]
    InvalidLifecycle,
    #[error("scheduler job project scope is invalid")]
    ProjectScope,
    #[error("scheduler job was not found")]
    NotFound,
    #[error("scheduler job storage query failed")]
    Query,
}

#[derive(Clone)]
pub struct JobStore {
    pool: Pool<Sqlite>,
}
impl JobStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
    pub async fn create(&self, job: ScheduledJob) -> Result<(), JobError> {
        validate_job(&job)?;
        sqlx::query("INSERT INTO scheduler_jobs (project_id, job_id, owner_id, trigger_kind, trigger_value, target_kind, target_id, target_version, timezone, enabled, lifecycle, concurrency_limit, missed_run_policy, revision, created_at_ms, updated_at_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)").bind(&job.project_id).bind(&job.job_id).bind(&job.owner_id).bind(job.trigger.kind()).bind(job.trigger.value()?).bind(job.target.parts().0).bind(job.target.parts().1).bind(i64::from(job.target.parts().2)).bind(&job.timezone).bind(job.enabled).bind(&job.lifecycle).bind(i64::from(job.concurrency_limit)).bind(job.missed_run_policy.as_str()).bind(now_ms()).bind(now_ms()).execute(&self.pool).await.map_err(map_error)?;
        Ok(())
    }
    pub async fn get(&self, project: &str, job_id: &str) -> Result<ScheduledJob, JobError> {
        validate_text(project)?;
        validate_text(job_id)?;
        let row = sqlx::query("SELECT owner_id, trigger_kind, trigger_value, target_kind, target_id, target_version, timezone, enabled, lifecycle, concurrency_limit, missed_run_policy, revision FROM scheduler_jobs WHERE project_id = ? AND job_id = ?").bind(project).bind(job_id).fetch_optional(&self.pool).await.map_err(|_| JobError::Query)?.ok_or(JobError::NotFound)?;
        decode(project, job_id, row)
    }
    pub async fn update(
        &self,
        job: ScheduledJob,
        expected_revision: u64,
    ) -> Result<ScheduledJob, JobError> {
        validate_job(&job)?;
        let result = sqlx::query("UPDATE scheduler_jobs SET owner_id=?, trigger_kind=?, trigger_value=?, target_kind=?, target_id=?, target_version=?, timezone=?, enabled=?, lifecycle=?, concurrency_limit=?, missed_run_policy=?, revision=revision+1, updated_at_ms=? WHERE project_id=? AND job_id=? AND revision=?").bind(&job.owner_id).bind(job.trigger.kind()).bind(job.trigger.value()?).bind(job.target.parts().0).bind(job.target.parts().1).bind(i64::from(job.target.parts().2)).bind(&job.timezone).bind(job.enabled).bind(&job.lifecycle).bind(i64::from(job.concurrency_limit)).bind(job.missed_run_policy.as_str()).bind(now_ms()).bind(&job.project_id).bind(&job.job_id).bind(i64::try_from(expected_revision).map_err(|_| JobError::StaleRevision)?).execute(&self.pool).await.map_err(|_| JobError::Query)?;
        if result.rows_affected() != 1 {
            return Err(JobError::StaleRevision);
        }
        self.get(&job.project_id, &job.job_id).await
    }
}
fn validate_job(job: &ScheduledJob) -> Result<(), JobError> {
    validate_text(&job.project_id)?;
    validate_text(&job.job_id)?;
    validate_text(&job.owner_id)?;
    if job.timezone.is_empty() {
        return Err(JobError::InvalidTimezone);
    }
    validate_text(&job.timezone)?;
    if !matches!(job.lifecycle.as_str(), "active" | "disabled" | "archived") {
        return Err(JobError::InvalidLifecycle);
    }
    if job.target.parts().2 == 0 {
        return Err(JobError::InvalidBounds);
    }
    job.trigger.value()?;
    Ok(())
}
fn decode(
    project: &str,
    job: &str,
    row: sqlx::sqlite::SqliteRow,
) -> Result<ScheduledJob, JobError> {
    let trigger_value: String = row.get("trigger_value");
    let trigger = match row.get::<String, _>("trigger_kind").as_str() {
        "one_shot" => Trigger::OneShot {
            at_ms: trigger_value
                .parse()
                .map_err(|_| JobError::InvalidTrigger)?,
        },
        "interval" => Trigger::Interval {
            seconds: trigger_value
                .parse()
                .map_err(|_| JobError::InvalidTrigger)?,
        },
        "cron" => Trigger::Cron {
            expression: trigger_value,
        },
        "event" => Trigger::Event {
            name: trigger_value,
        },
        "dependency" => Trigger::Dependency {
            job_id: trigger_value,
        },
        _ => return Err(JobError::InvalidTrigger),
    };
    let target_id: String = row.get("target_id");
    let version =
        u32::try_from(row.get::<i64, _>("target_version")).map_err(|_| JobError::InvalidBounds)?;
    let target = match row.get::<String, _>("target_kind").as_str() {
        "workflow" => JobTarget::Workflow {
            workflow_id: target_id,
            version,
        },
        "agent" => JobTarget::Agent {
            agent_id: target_id,
            version,
        },
        "tool" => JobTarget::Tool {
            tool_id: target_id,
            version,
        },
        _ => return Err(JobError::InvalidBounds),
    };
    let missed = match row.get::<String, _>("missed_run_policy").as_str() {
        "skip" => MissedRunPolicy::Skip,
        "catch_up" => MissedRunPolicy::CatchUp,
        "pause" => MissedRunPolicy::Pause,
        _ => return Err(JobError::InvalidBounds),
    };
    Ok(ScheduledJob {
        project_id: project.into(),
        job_id: job.into(),
        owner_id: row.get("owner_id"),
        trigger,
        target,
        timezone: row.get("timezone"),
        concurrency_limit: u32::try_from(row.get::<i64, _>("concurrency_limit"))
            .map_err(|_| JobError::InvalidBounds)?,
        missed_run_policy: missed,
        enabled: row.get::<i64, _>("enabled") != 0,
        lifecycle: row.get("lifecycle"),
        revision: u64::try_from(row.get::<i64, _>("revision"))
            .map_err(|_| JobError::InvalidBounds)?,
    })
}
fn validate_text(value: &str) -> Result<(), JobError> {
    if value.trim().is_empty() || value.len() > MAX_TEXT || value.chars().any(char::is_control) {
        Err(JobError::InvalidIdentity)
    } else {
        Ok(())
    }
}
fn map_error(error: sqlx::Error) -> JobError {
    if error
        .as_database_error()
        .is_some_and(|db| db.is_unique_violation())
    {
        JobError::Duplicate
    } else {
        JobError::Query
    }
}
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
