-- Migration 0002: Project folders table and integrity constraints

CREATE TABLE IF NOT EXISTS project_folders (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE (project_id, path)
);

CREATE INDEX IF NOT EXISTS idx_project_folders_project ON project_folders(project_id);
