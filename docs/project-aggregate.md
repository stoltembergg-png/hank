# Project Aggregate (PR-027)

## Overview

`Project` is the primary aggregate root and isolation boundary within the `agent-core` domain.

## Architectural Boundaries

- **Pure Domain Object (AI-001, AI-003, D-001):** Implemented in `crates/agent-core`, free of any framework or storage dependencies (no SQLite, no SQLx, no Tokio, no Tauri).
- **Isolation Root:** Agents, chat sessions, memories, skills, and workflows are strictly scoped to a `ProjectId`. Cross-project leakage is rejected by invariant design.

## Invariants & Field Validation

- **Name:** Non-empty, trimmed, maximum 128 characters, no control characters.
- **Owner:** Non-empty, trimmed, maximum 128 characters, no control characters.
- **Description:** Optional, maximum 1024 characters.
- **Identity:** Strongly typed `ProjectId` prefixed with `proj-`.

## Lifecycle

```text
       ┌──────────┐
       │  Active  │ <──── (resume)
       └────┬─────┘
       pause│  ▲
            ▼  │pause
       ┌──────────┐
       │  Paused  │
       └────┬─────┘
     archive│  ▲ archive
            ▼  │
       ┌──────────┐
       │ Archived │ (terminal)
       └──────────┘
```

- **Active:** Default initial state.
- **Paused:** Pauses agent and workflow execution; can be resumed to Active.
- **Archived:** Terminal state. Any mutation (adding agents, resuming, pausing, re-archiving) returns a `DomainError`.

## Usage

```rust
use agent_core::project::{Project, ProjectStatus};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut project = Project::create("Hank Project", "gabriel", Some("Workspace description".into()))?;

    project.pause()?;
    project.resume()?;
    project.archive()?;

    // Archived project rejects further transitions
    assert!(project.resume().is_err());
    Ok(())
}
```
