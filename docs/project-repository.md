# Project Repository (PR-028)

## Overview

The `ProjectRepository` defines the persistence boundary for the `Project` aggregate root. Its port interface is declared in `agent-core` and implemented by `SqliteProjectRepository` in `agent-runtime`.

## Architectural Boundary

- **Dependency Inversion (AI-001, AI-003, D-001):** `agent-core` declares `pub trait ProjectRepository: Send + Sync`. It has zero knowledge of SQL or SQLite.
- **Transactional Persistence:** `SqliteProjectRepository` executes against migrated SQLite schemas (`0001_initial_schema.sql`).
- **Domain Error Mapping:** SQL unique constraint violations are mapped to `DomainError::Duplicate`, missing records to `DomainError::NotFound`, and query failures to `DomainError::Internal`.
- **SQL Injection Prevention:** All SQL queries are strictly parameterized with SQLite query bindings.

## Trait Definition

```rust
pub trait ProjectRepository: Send + Sync {
    fn save(&self, project: &Project) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn get_by_id(&self, id: &ProjectId) -> impl Future<Output = Result<Option<Project>, DomainError>> + Send;
    fn list(&self, limit: usize, offset: usize) -> impl Future<Output = Result<Vec<Project>, DomainError>> + Send;
    fn update(&self, project: &Project) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn delete(&self, id: &ProjectId) -> impl Future<Output = Result<bool, DomainError>> + Send;
}
```

## Usage

```rust
use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::migrations::run_migrations;
use agent_runtime::project_repo::SqliteProjectRepository;
use agent_core::project::{Project, ProjectRepository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = SqliteStorage::connect_in_memory().await?;
    run_migrations(storage.pool()).await?;

    let repo = SqliteProjectRepository::new(storage.pool().clone());
    let project = Project::create("Hank Dev", "user1", None)?;

    repo.save(&project).await?;
    let retrieved = repo.get_by_id(&project.id).await?;
    assert!(retrieved.is_some());

    Ok(())
}
```
