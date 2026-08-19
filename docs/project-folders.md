# Project Folders (PR-033)

## Overview

The `ProjectFolder` value object represents monitored filesystem roots belonging to a `Project` scope. Folders establish the explicit file boundary for agents and tools within that project, avoiding implicit cross-project access.

## Domain Model & Constraints

- **Folder Identity:** Prefixed typed identifier (`fld-<uuid>`).
- **Validation Rules:**
  - Name: Non-empty, $\le 128$ characters, no control characters.
  - Path: Non-empty, $\le 1024$ characters, no control characters, no path traversal (`..`).
- **Uniqueness & Cascading:**
  - Unique per project on `(project_id, path)` in SQLite table `project_folders`.
  - Cascades deletions on `ON DELETE CASCADE` when the parent project is removed.
- **Archival Protection:** Adding folders to an archived project is rejected with `DomainError::InvalidStateTransition`.

## Schema (`migrations/0002_project_folders.sql`)

```sql
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
```

## Repository API

```rust
pub trait ProjectRepository: Send + Sync {
    // ...
    fn add_folder(&self, project_id: &ProjectId, folder: &ProjectFolder) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn list_folders(&self, project_id: &ProjectId) -> impl Future<Output = Result<Vec<ProjectFolder>, DomainError>> + Send;
    fn remove_folder(&self, project_id: &ProjectId, folder_id: &str) -> impl Future<Output = Result<bool, DomainError>> + Send;
}
```
