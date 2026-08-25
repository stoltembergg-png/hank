-- Migration 0016: bounded scheduled job entity

CREATE TABLE IF NOT EXISTS scheduler_jobs (
    project_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    trigger_kind TEXT NOT NULL,
    trigger_value TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    target_version INTEGER NOT NULL,
    timezone TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    lifecycle TEXT NOT NULL,
    concurrency_limit INTEGER NOT NULL,
    missed_run_policy TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, job_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_scheduler_jobs_project_lifecycle ON scheduler_jobs(project_id, lifecycle, enabled);
