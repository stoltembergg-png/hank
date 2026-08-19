# Create Project Service (PR-029)

## Overview

The `CreateProjectService` coordinates the creation of a new `Project` aggregate root, persists it via `ProjectRepository`, and dispatches a validated `ProjectCreated` event through the `EventBus`.

## Flow & Transactional Invariants

```text
CreateProjectInput
       │
       ▼
 1. Project::create (Validate Name/Owner/Desc)
       │
       ▼
 2. ProjectRepository::save (Transactional Persistence)
       │
       ├─ [Failure: Duplicate/DB error] ──► Return DomainError (NO Event Published)
       │
       ▼
 3. EventBus::publish (Emit ProjectCreated ApplicationEvent)
       │
       ▼
CreateProjectOutput
```

1. **Validation First:** Validates all fields through `Project::create`. Rejects invalid, empty, or oversized names and owners.
2. **Transactional Boundary:** Persists the entity before publishing events. If persistence fails (e.g. unique constraint violation), the service fails immediately without emitting phantom events.
3. **Event Notification:** Constructs an `ApplicationEvent` with `EventKind::ProjectCreated` and publishes it to the bounded `EventBus`.

## Usage

```rust
use std::sync::Arc;
use agent_runtime::event_bus::EventBus;
use agent_runtime::project_repo::SqliteProjectRepository;
use agent_runtime::project_service::{CreateProjectInput, CreateProjectService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Arc::new(SqliteProjectRepository::new(pool));
    let bus = EventBus::bounded(128);
    let service = CreateProjectService::new(repo, Some(bus));

    let output = service.execute(CreateProjectInput {
        name: "My Project".into(),
        owner: "user1".into(),
        description: Some("New workspace".into()),
        correlation_id: Some("req-001".into()),
    }).await?;

    println!("Project created: {}", output.project.id);
    Ok(())
}
```
