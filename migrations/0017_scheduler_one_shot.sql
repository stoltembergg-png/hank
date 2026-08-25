-- Migration 0017: one-shot consumed/claim state
ALTER TABLE scheduler_jobs ADD COLUMN expires_at_ms INTEGER;
ALTER TABLE scheduler_jobs ADD COLUMN claim_id TEXT;
ALTER TABLE scheduler_jobs ADD COLUMN consumed_at_ms INTEGER;
CREATE UNIQUE INDEX IF NOT EXISTS idx_scheduler_jobs_claim_id
    ON scheduler_jobs(project_id, claim_id)
    WHERE claim_id IS NOT NULL;
