CREATE TABLE IF NOT EXISTS memory_policies (
    project_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    layer TEXT NOT NULL,
    version INTEGER NOT NULL,
    policy_json TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, agent_id, layer, version),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    CHECK (length(project_id) BETWEEN 1 AND 160),
    CHECK (length(agent_id) BETWEEN 1 AND 160),
    CHECK (layer IN ('system', 'security', 'project', 'agent')),
    CHECK (version > 0),
    CHECK (length(policy_json) BETWEEN 2 AND 65536)
);

CREATE INDEX IF NOT EXISTS idx_memory_policies_latest
    ON memory_policies(project_id, agent_id, layer, active, version DESC);
