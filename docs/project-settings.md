# Project Settings (PR-035)

## Overview

The `ProjectSettings` domain struct models bounded, validated configuration parameters associated with a `Project` aggregate root. It governs retention, concurrency, and default agent policies without exposing sensitive credentials or runtime secrets.

## Domain Model & Allowed Fields

- **Default Budget & Policies:** `BudgetPolicy`, `AgentPolicyConfig`, `InstructionHierarchy`, `CapabilitySet`.
- **Retention Period:** `retention_days: u32` ($1 \le \text{days} \le 3650$, default 90).
- **Auto-Archive Inactivity:** `auto_archive_idle_days: Option<u32>` ($1 \le \text{days} \le 365$).
- **Concurrency Ceiling:** `max_active_agents: u32` ($1 \le \text{agents} \le 50$, default 5).
- **Telemetry Flag:** `telemetry_enabled: bool` (default false).

## Validation Invariants

```rust
impl ProjectSettings {
    pub fn validate(&self) -> Result<(), DomainError> {
        // Enforces bounded ranges and reject invalid configurations
    }
}
```

- Updating settings on an archived project fails with `DomainError::InvalidStateTransition`.
- Serialization format: JSON stored in the `settings` column of SQLite table `projects`.

## Repository API

```rust
pub trait ProjectRepository: Send + Sync {
    // ...
    fn update_settings(&self, project_id: &ProjectId, settings: &ProjectSettings) -> impl Future<Output = Result<(), DomainError>> + Send;
    fn get_settings(&self, project_id: &ProjectId) -> impl Future<Output = Result<Option<ProjectSettings>, DomainError>> + Send;
}
```
