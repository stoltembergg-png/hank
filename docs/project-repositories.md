# Project Repositories (PR-034)

## Overview

The `ProjectGitRepo` value object models git repository bindings attached to a `Project` scope without executing Git binaries or operations directly.

## Domain Model & Constraints

- **Repository Identity:** Prefixed identifier (`repo-<uuid>`).
- **Validation Rules:**
  - Name: Non-empty, $\le 128$ characters, no control characters.
  - URL: Non-empty, $\le 1024$ characters, no control characters, no plaintext basic-auth credentials (`user:pass@host`).
  - Default Branch: Non-empty, $\le 256$ characters, no control characters.
  - Worktree Path: Optional, $\le 1024$ characters, no path traversal (`..`).
- **Uniqueness & Cascading:**
  - Unique per project on `(project_id, url)` in SQLite table `project_repositories`.
  - Cascades deletions on `ON DELETE CASCADE` when the parent project is deleted.
- **Archival Protection:** Adding repositories to an archived project is rejected with `DomainError::InvalidStateTransition`.

## Schema (`migrations/0003_project_repositories.sql`)

```sql
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
```

## Repository API

```rust
pub trait ProjectRepository: Send + Sync {
    // ...
    fn add_git_repo(&self, project_id: &ProjectId, repo: &ProjectGitRepo) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn list_git_repos(&self, project_id: &ProjectId) -> impl Future<Output = Result<Vec<ProjectGitRepo>, DomainError>> + Send;
    fn remove_git_repo(&self, project_id: &ProjectId, repo_id: &str) -> impl Future<Output = Result<bool, DomainError>> + Send;
}
```
