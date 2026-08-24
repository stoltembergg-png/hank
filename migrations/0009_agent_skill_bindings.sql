-- Explicit, agent-scoped skill assignments for M8.
CREATE TABLE agent_skill_bindings (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    skill_id TEXT NOT NULL,
    current_version TEXT NOT NULL,
    previous_version TEXT,
    precedence INTEGER NOT NULL CHECK (precedence >= 0 AND precedence <= 65535),
    max_tokens INTEGER NOT NULL CHECK (max_tokens > 0),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    actor_id TEXT NOT NULL,
    approval_id TEXT,
    trace_id TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, agent_id, skill_id)
);

CREATE INDEX agent_skill_bindings_order_idx
    ON agent_skill_bindings(project_id, agent_id, enabled, precedence ASC, skill_id ASC);
