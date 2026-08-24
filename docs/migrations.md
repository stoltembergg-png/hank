# SQL Migrations (PR-026)

## Overview

The `migrations` module in `crates/agent-runtime` provides deterministic, embedded schema versioning for SQLite metadata persistence using SQLx migrations.

## Migration Principles

- **Deterministic Execution:** Migrations are embedded at compile time via `sqlx::migrate!("../../migrations")` and executed in order (`0001_initial_schema.sql`, etc.).
- **Atomic & Transactional:** Each migration runs inside a transaction. If a statement fails, changes are rolled back completely.
- **Idempotent:** Running `run_migrations` multiple times is safe and performs no-op when schemas are up-to-date.
- **Foreign Key Cascades:** Relationships (e.g. `agents.project_id -> projects.id`, `sessions.project_id -> projects.id`, `messages.session_id -> sessions.id`) define `ON DELETE CASCADE` to prevent orphaned records upon project deletion.

## Initial Schema (`0001_initial_schema.sql`)

- `projects`: aggregate root for project boundaries, settings, and lifecycle.
- `agents`: agents scoped to projects.
- `sessions`: chat sessions scoped to projects and agents.
- `messages`: session messages with role, content, and tool executions.
- `skill_versions` and `skill_heads`: immutable, scoped Skill history and its
  optimistic-concurrency head (migration `0007_skill_storage.sql`).

## Usage

```rust
use agent_runtime::sqlite::SqliteStorage;
use agent_runtime::migrations::run_migrations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = SqliteStorage::connect_in_memory().await?;
    run_migrations(storage.pool()).await?;
    Ok(())
}
```
