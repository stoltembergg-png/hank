# Update Project Service (PR-031)

## Overview

The `UpdateProjectService` handles validated modifications to existing `Project` aggregate roots, supporting optimistic concurrency control and emitting `ProjectUpdated` events upon persistence.

## Mutation Rules and Guards

- **Archived Immutability:** Updating an archived project fails with `DomainError::InvalidStateTransition`.
- **Optimistic Concurrency:** If `expected_updated_at` is provided, the service verifies that the entity has not been modified since the caller last read it.
- **Strict Input Validation:** Project name, description, and status transitions (Pause/Resume) are strictly validated.
- **Event Dispatch:** Dispatches `ApplicationEvent` with `EventKind::ProjectUpdated` only after SQLite persistence completes.

## Usage

```rust
use std::sync::Arc;
use agent_core::project::ProjectStatus;
use agent_runtime::event_bus::EventBus;
use agent_runtime::project_repo::SqliteProjectRepository;
use agent_runtime::project_update_service::{UpdateProjectInput, UpdateProjectService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Arc::new(SqliteProjectRepository::new(pool));
    let bus = EventBus::bounded(64);
    let service = UpdateProjectService::new(repo, Some(bus));

    let output = service.execute(UpdateProjectInput {
        id: project_id,
        name: Some("Renamed App".into()),
        description: Some("Updated description".into()),
        status: Some(ProjectStatus::Paused),
        expected_updated_at: None,
        correlation_id: Some("req-update-001".into()),
    }).await?;

    println!("Project updated: {}", output.project.name);
    Ok(())
}
```
