ALTER TABLE sessions ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE sessions ADD COLUMN correlation_id TEXT NOT NULL DEFAULT 'legacy-correlation';
ALTER TABLE sessions ADD COLUMN participants TEXT NOT NULL DEFAULT '[]';
ALTER TABLE sessions ADD COLUMN metadata TEXT NOT NULL DEFAULT '{}';
ALTER TABLE sessions ADD COLUMN budget_ref TEXT;
ALTER TABLE sessions ADD COLUMN trace_id TEXT;
ALTER TABLE sessions ADD COLUMN failure_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_sessions_project_created
    ON sessions(project_id, created_at DESC);