-- Migration 0015: lease fencing and recovery classification

ALTER TABLE workflow_runs ADD COLUMN lease_owner TEXT;
ALTER TABLE workflow_runs ADD COLUMN lease_expires_at_ms INTEGER;
ALTER TABLE workflow_node_states ADD COLUMN recovery_class TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE workflow_node_states ADD COLUMN unknown_effect INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS workflow_recovery_reports (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    recovery_id TEXT NOT NULL,
    previous_generation INTEGER NOT NULL,
    new_generation INTEGER NOT NULL,
    recovery_class TEXT NOT NULL,
    requires_reconcile INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, recovery_id),
    FOREIGN KEY (project_id, run_id) REFERENCES workflow_runs(project_id, run_id) ON DELETE CASCADE
);
