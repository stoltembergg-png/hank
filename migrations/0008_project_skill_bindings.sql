-- Versioned, project-owned skill assignments for M8.
CREATE TABLE project_skill_bindings (
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    skill_id TEXT NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('project', 'global')),
    current_version TEXT NOT NULL,
    previous_version TEXT,
    import_reference TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    actor_id TEXT NOT NULL,
    approval_id TEXT,
    trace_id TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (project_id, skill_id),
    CHECK (
        (scope = 'project' AND import_reference IS NULL)
        OR (scope = 'global' AND import_reference IS NOT NULL)
    )
);

CREATE INDEX project_skill_bindings_project_enabled_idx
    ON project_skill_bindings(project_id, enabled, updated_at DESC, skill_id ASC);
