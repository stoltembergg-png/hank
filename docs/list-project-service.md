# List & Query Project Service (PR-030)

## Overview

The `ListProjectsService` provides bounded, paginated project querying and single-project retrieval for consumers of the Application API layer without direct database access.

## Architecture and Query Limits

- **Clean Boundary (AI-001, AI-003, D-001):** Consumes `ProjectRepository` port. Frontend, Tauri commands, and CLI consumers interact only via `ListProjectsService`.
- **Bounded Pagination:** Default limit is 20 items; maximum clamped limit is 100 items to prevent memory exhaustion and unbounded payload transfers.
- **DTO Mapping:** Transforms domain `Project` aggregates into lightweight `ProjectSummary` DTOs containing essential metadata and agent counts.

## Query DTOs

```rust
pub struct ListProjectsInput {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status_filter: Option<ProjectStatus>,
    pub correlation_id: Option<String>,
}

pub struct ListProjectsOutput {
    pub items: Vec<ProjectSummary>,
    pub limit: usize,
    pub offset: usize,
    pub correlation_id: Option<String>,
}
```

## Usage

```rust
use std::sync::Arc;
use agent_runtime::project_query_service::{ListProjectsInput, ListProjectsService};
use agent_runtime::project_repo::SqliteProjectRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Arc::new(SqliteProjectRepository::new(pool));
    let query_service = ListProjectsService::new(repo);

    let output = query_service.list(ListProjectsInput {
        limit: Some(10),
        offset: Some(0),
        status_filter: None,
        correlation_id: Some("req-query-001".into()),
    }).await?;

    for project in output.items {
        println!("Project: {} ({})", project.name, project.id);
    }
    Ok(())
}
```
