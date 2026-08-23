-- Project-scoped memory persistence for PR-123.
CREATE TABLE memories (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    agent_id TEXT,
    session_id TEXT,
    memory_type TEXT NOT NULL,
    status TEXT NOT NULL,
    content TEXT NOT NULL,
    summary TEXT,
    importance REAL NOT NULL,
    tags TEXT NOT NULL,
    provenance TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    accessed_at TEXT,
    access_count INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    UNIQUE(project_id, id)
);

CREATE INDEX memories_project_active_idx
    ON memories(project_id, status, updated_at DESC);
