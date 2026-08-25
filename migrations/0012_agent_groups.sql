CREATE TABLE IF NOT EXISTS agent_groups (
    group_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    group_json TEXT NOT NULL,
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('draft', 'active', 'archived')),
    revision INTEGER NOT NULL CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, group_id)
);

CREATE INDEX IF NOT EXISTS idx_agent_groups_project_lifecycle
    ON agent_groups(project_id, lifecycle, updated_at);
