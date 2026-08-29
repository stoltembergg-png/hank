-- Migration 0021: durable project-scoped task/worktree/branch mappings

CREATE TABLE IF NOT EXISTS task_workspace_mappings (
    project_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    branch TEXT NOT NULL,
    agent_run_id TEXT NOT NULL,
    pull_request_id TEXT,
    correlation_id TEXT NOT NULL,
    policy_revision TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('active', 'detached', 'reconcile_required', 'released')),
    revision INTEGER NOT NULL CHECK (revision > 0),
    observed_repository_id TEXT,
    observed_worktree_id TEXT,
    observed_branch TEXT,
    observed_at_ms INTEGER,
    observed_correlation_id TEXT,
    reconcile_reason TEXT,
    last_reconciled_at_ms INTEGER,
    last_resumed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, task_id),
    UNIQUE (project_id, worktree_id),
    UNIQUE (project_id, repository_id, branch),
    UNIQUE (project_id, agent_run_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_task_workspace_mappings_project_state
    ON task_workspace_mappings(project_id, state, updated_at_ms);
