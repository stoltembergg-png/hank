CREATE TABLE IF NOT EXISTS scheduler_concurrency_admissions (
    project_id TEXT NOT NULL,
    concurrency_key TEXT NOT NULL,
    run_id TEXT NOT NULL,
    lease_owner TEXT NOT NULL,
    lease_expires_at_ms INTEGER NOT NULL,
    admitted_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, concurrency_key, run_id),
    FOREIGN KEY (project_id, run_id) REFERENCES scheduler_runs(project_id, run_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_scheduler_concurrency_active
    ON scheduler_concurrency_admissions(project_id, concurrency_key, lease_expires_at_ms);
