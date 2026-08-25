-- Migration 0014: durable workflow run/node state, transition journal and pending anchors

CREATE TABLE IF NOT EXISTS workflow_runs (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    workflow_version INTEGER NOT NULL,
    state TEXT NOT NULL,
    generation INTEGER NOT NULL,
    sequence INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, run_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_node_states (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    state TEXT NOT NULL,
    generation INTEGER NOT NULL,
    attempt INTEGER NOT NULL,
    checkpoint_before TEXT,
    checkpoint_after TEXT,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, run_id, node_id),
    FOREIGN KEY (project_id, run_id) REFERENCES workflow_runs(project_id, run_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_transitions (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    transition_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    node_id TEXT NOT NULL,
    expected_state TEXT NOT NULL,
    next_state TEXT NOT NULL,
    generation INTEGER NOT NULL,
    recovery_class TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (project_id, run_id, transition_id),
    UNIQUE (project_id, run_id, idempotency_key),
    FOREIGN KEY (project_id, run_id) REFERENCES workflow_runs(project_id, run_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_pending_approvals (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    approval_id TEXT NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    state TEXT NOT NULL,
    PRIMARY KEY (project_id, run_id, node_id, generation),
    UNIQUE (project_id, approval_id),
    FOREIGN KEY (project_id, run_id) REFERENCES workflow_runs(project_id, run_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workflow_pending_delays (
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    deadline_ms INTEGER NOT NULL,
    state TEXT NOT NULL,
    PRIMARY KEY (project_id, run_id, node_id, generation),
    FOREIGN KEY (project_id, run_id) REFERENCES workflow_runs(project_id, run_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_workflow_transitions_run_sequence
    ON workflow_transitions(project_id, run_id, sequence);
