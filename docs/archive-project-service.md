# Archive Project Service (PR-032)

## Overview

The `ArchiveProjectService` performs a state-level soft archive transition on a `Project` aggregate root without destructive cascading or deleting associated artifacts, memories, or skills.

## Architectural Rules

- **Soft Transition vs Hard Purge:** Sets `ProjectStatus::Archived`. Prevents subsequent modifications or agent executions on the project without deleting its audit history or stored resources.
- **Idempotency:** If the project is already in `Archived` state, subsequent archive calls succeed idempotently without emitting duplicate `ProjectArchived` events.
- **Event Notification:** Dispatches `ApplicationEvent` with `EventKind::ProjectArchived` only after the state update has been committed to SQLite.

## Usage

```rust
use std::sync::Arc;
use agent_runtime::event_bus::EventBus;
use agent_runtime::project_archive_service::{ArchiveProjectInput, ArchiveProjectService};
use agent_runtime::project_repo::SqliteProjectRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Arc::new(SqliteProjectRepository::new(pool));
    let bus = EventBus::bounded(64);
    let service = ArchiveProjectService::new(repo, Some(bus));

    let output = service.execute(ArchiveProjectInput {
        id: project_id,
        reason: Some("Workspace decommissioned".into()),
        correlation_id: Some("req-arch-001".into()),
    }).await?;

    println!("Project archived: {} (already: {})", output.project.id, output.already_archived);
    Ok(())
}
```
