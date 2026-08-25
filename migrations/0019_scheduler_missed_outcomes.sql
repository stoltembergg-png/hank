ALTER TABLE scheduler_jobs ADD COLUMN missed_policy_version TEXT NOT NULL DEFAULT 'v1';

CREATE TABLE IF NOT EXISTS scheduler_missed_outcomes (
    project_id TEXT NOT NULL,
    outcome_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    occurrence_at_ms INTEGER NOT NULL,
    action TEXT NOT NULL,
    reason TEXT NOT NULL,
    coalesce_key TEXT,
    policy_version TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, outcome_id),
    UNIQUE (project_id, run_id, occurrence_at_ms, action),
    FOREIGN KEY (project_id, run_id) REFERENCES scheduler_runs(project_id, run_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_scheduler_missed_outcomes_run
    ON scheduler_missed_outcomes(project_id, run_id, occurrence_at_ms);
