-- Migration 0003: Project repositories table and integrity constraints

CREATE TABLE IF NOT EXISTS project_repositories (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    branch TEXT NOT NULL,
    worktree_path TEXT,
    added_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE (project_id, url)
);

CREATE INDEX IF NOT EXISTS idx_project_repositories_project ON project_repositories(project_id);
