ALTER TABLE scheduler_jobs ADD COLUMN next_due_at_ms INTEGER;

CREATE TABLE IF NOT EXISTS scheduler_runs (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    job_id TEXT NOT NULL,
    due_at_ms INTEGER NOT NULL,
    status TEXT NOT NULL,
    lease_owner TEXT,
    lease_expires_at_ms INTEGER,
    completed_at_ms INTEGER,
    outcome TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, run_id),
    FOREIGN KEY (project_id, job_id) REFERENCES scheduler_jobs(project_id, job_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_scheduler_runs_due_status
    ON scheduler_runs(project_id, status, due_at_ms);
CREATE INDEX IF NOT EXISTS idx_scheduler_runs_lease
    ON scheduler_runs(project_id, lease_expires_at_ms);
