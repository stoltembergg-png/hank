-- Versioned, project/global-scoped Skill persistence for M8.
CREATE TABLE skill_versions (
    namespace TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    scope TEXT NOT NULL CHECK (scope IN ('project', 'global')),
    name TEXT NOT NULL,
    manifest_version TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    skill_json TEXT NOT NULL,
    parsed_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (namespace, skill_id, manifest_version),
    UNIQUE (namespace, name, manifest_version, content_hash),
    CHECK (
        (scope = 'project' AND project_id IS NOT NULL)
        OR (scope = 'global' AND project_id IS NULL)
    )
);

CREATE TABLE skill_heads (
    namespace TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
    scope TEXT NOT NULL CHECK (scope IN ('project', 'global')),
    current_version TEXT NOT NULL,
    status TEXT NOT NULL,
    pinned_version TEXT,
    activated_at TEXT,
    rollback_version TEXT,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (namespace, skill_id),
    FOREIGN KEY (namespace, skill_id, current_version)
        REFERENCES skill_versions(namespace, skill_id, manifest_version),
    CHECK (
        (scope = 'project' AND project_id IS NOT NULL)
        OR (scope = 'global' AND project_id IS NULL)
    )
);

CREATE INDEX skill_versions_namespace_name_idx
    ON skill_versions(namespace, name, manifest_version);

CREATE INDEX skill_heads_namespace_status_idx
    ON skill_heads(namespace, status, updated_at DESC, skill_id ASC);
