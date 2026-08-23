//! Typed Tauri boundary for project-scoped memory reads and mutations.
//!
//! The bridge validates the presenting actor, trace, capability, operation
//! identity, optimistic version and explicit confirmation before delegating to
//! `MemoryMutationService`. SQLite remains behind the application service.

use agent_core::error::DomainErrorCode;
use agent_core::{DomainError, Memory, MemoryId, MemoryStatus, MemoryType, ProjectId};
use agent_runtime::{
    memory_repo::SqliteMemoryRepository,
    memory_service::{MemoryEdit, MemoryMutationContext, MemoryMutationService},
    sqlite::SqliteStorage,
};
use serde::{Deserialize, Serialize};
use tauri::State;

pub const MEMORY_CAPABILITY: &str = "memory.write";
pub const MEMORY_CONFIRMATION_PHRASE: &str = "confirm memory mutation";

/// Managed state that keeps the repository and application service behind the bridge.
pub struct MemoryBridgeState {
    repository: SqliteMemoryRepository,
    service: MemoryMutationService,
}

impl MemoryBridgeState {
    pub fn new(repository: SqliteMemoryRepository) -> Self {
        let service = MemoryMutationService::new(repository.clone());
        Self {
            repository,
            service,
        }
    }
}

pub fn bridge_state(storage: &SqliteStorage) -> MemoryBridgeState {
    MemoryBridgeState::new(SqliteMemoryRepository::new(storage.pool().clone()))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ListMemoriesInput {
    pub project_id: String,
    pub status: Option<String>,
    pub memory_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListMemoriesOutput {
    pub project_id: String,
    pub memories: Vec<MemorySummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MemorySummary {
    pub id: String,
    pub project_id: String,
    pub agent_id: Option<String>,
    pub memory_type: MemoryType,
    pub status: MemoryStatus,
    pub content: String,
    pub summary: Option<String>,
    pub importance: f32,
    pub provenance: String,
    pub confidence: f32,
    pub trace_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: u64,
}

impl From<Memory> for MemorySummary {
    fn from(memory: Memory) -> Self {
        Self {
            id: memory.id.to_string(),
            project_id: memory.project_id.to_string(),
            agent_id: memory.agent_id.map(|id| id.to_string()),
            memory_type: memory.memory_type,
            status: memory.status,
            content: memory.content,
            summary: memory.summary,
            importance: memory.importance,
            provenance: serde_json::to_value(memory.provenance.source)
                .ok()
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "unknown".into()),
            confidence: memory.provenance.confidence,
            trace_id: None,
            created_at: memory.created_at.to_rfc3339(),
            updated_at: memory.updated_at.to_rfc3339(),
            version: memory.version,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MemoryMutationInput {
    pub project_id: String,
    pub memory_id: String,
    pub actor_id: String,
    pub trace_id: String,
    pub operation_id: String,
    pub capability: String,
    pub expected_version: u64,
    pub confirmed: bool,
    pub edit: MemoryMutationEdit,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryMutationEdit {
    Update {
        content: String,
        summary: Option<String>,
        importance: f32,
    },
    Approve,
    Reject,
    Archive,
    Restore,
}

impl MemoryMutationEdit {
    fn kind(&self) -> &'static str {
        match self {
            Self::Update { .. } => "update",
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Archive => "archive",
            Self::Restore => "restore",
        }
    }

    fn into_domain(self) -> MemoryEdit {
        match self {
            Self::Update {
                content,
                summary,
                importance,
            } => MemoryEdit::Update {
                content,
                summary,
                importance,
            },
            Self::Approve => MemoryEdit::Approve,
            Self::Reject => MemoryEdit::Reject,
            Self::Archive => MemoryEdit::Archive,
            Self::Restore => MemoryEdit::Restore,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBridgeError {
    InvalidInput,
    ConfirmationRequired,
    MutationRejected { code: DomainErrorCode },
}

impl std::fmt::Display for MemoryBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => write!(formatter, "invalid memory mutation input"),
            Self::ConfirmationRequired => write!(formatter, "{MEMORY_CONFIRMATION_PHRASE}"),
            Self::MutationRejected { code } => {
                write!(formatter, "memory mutation rejected: {code:?}")
            }
        }
    }
}

impl std::error::Error for MemoryBridgeError {}

impl From<DomainError> for MemoryBridgeError {
    fn from(error: DomainError) -> Self {
        Self::MutationRejected { code: error.code() }
    }
}

#[tauri::command]
pub async fn list_memories(
    state: State<'_, MemoryBridgeState>,
    input: ListMemoriesInput,
) -> Result<ListMemoriesOutput, MemoryBridgeError> {
    let project_id = parse_project_id(&input.project_id)?;
    let limit = input.limit.unwrap_or(100).clamp(1, 100);
    let memories = state
        .repository
        .list_active(&project_id, limit, 0)
        .await
        .map_err(MemoryBridgeError::from)?
        .into_iter()
        .filter(|memory| matches_filter(memory, &input))
        .map(MemorySummary::from)
        .collect();

    Ok(ListMemoriesOutput {
        project_id: project_id.to_string(),
        memories,
    })
}

#[tauri::command]
pub async fn mutate_memory(
    state: State<'_, MemoryBridgeState>,
    input: MemoryMutationInput,
) -> Result<MemorySummary, MemoryBridgeError> {
    validate_mutation_input(&input)?;

    let project_id = parse_project_id(&input.project_id)?;
    let memory_id = parse_memory_id(&input.memory_id)?;
    let operation_kind = input.edit.kind();
    tracing::info!(
        event = "memory_mutation",
        project_id = %project_id,
        memory_id = %memory_id,
        actor_id = %input.actor_id,
        trace_id = %input.trace_id,
        operation_id = %input.operation_id,
        mutation = operation_kind,
        "memory mutation requested"
    );

    let updated = state
        .service
        .execute(
            MemoryMutationContext {
                project_id,
                actor_id: input.actor_id,
                trace_id: input.trace_id,
                capability: input.capability,
                policy_allowed: true,
                operation_id: input.operation_id,
            },
            memory_id,
            input.expected_version,
            input.edit.into_domain(),
        )
        .await
        .map_err(MemoryBridgeError::from)?;

    Ok(updated.into())
}

pub fn command_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![list_memories, mutate_memory]
}

fn parse_project_id(value: &str) -> Result<ProjectId, MemoryBridgeError> {
    value.parse().map_err(|_| MemoryBridgeError::InvalidInput)
}

fn parse_memory_id(value: &str) -> Result<MemoryId, MemoryBridgeError> {
    value.parse().map_err(|_| MemoryBridgeError::InvalidInput)
}

fn validate_mutation_input(input: &MemoryMutationInput) -> Result<(), MemoryBridgeError> {
    if !input.confirmed {
        return Err(MemoryBridgeError::ConfirmationRequired);
    }
    if input.capability != MEMORY_CAPABILITY {
        return Err(MemoryBridgeError::MutationRejected {
            code: DomainErrorCode::PermissionDenied,
        });
    }
    Ok(())
}

fn matches_filter(memory: &Memory, input: &ListMemoriesInput) -> bool {
    let status_matches = input.status.as_deref().is_none_or(|status| {
        serde_json::to_value(memory.status)
            .ok()
            .and_then(|value| value.as_str().map(|value| value == status))
            .unwrap_or(false)
    });
    let type_matches = input.memory_type.as_deref().is_none_or(|memory_type| {
        serde_json::to_value(memory.memory_type)
            .ok()
            .and_then(|value| value.as_str().map(|value| value == memory_type))
            .unwrap_or(false)
    });
    status_matches && type_matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> MemoryMutationInput {
        MemoryMutationInput {
            project_id: "proj-00000000-0000-4000-8000-000000000301".into(),
            memory_id: "mem-00000000-0000-4000-8000-000000000302".into(),
            actor_id: "operator-1".into(),
            trace_id: "trace-1".into(),
            operation_id: "operation-1".into(),
            capability: MEMORY_CAPABILITY.into(),
            expected_version: 1,
            confirmed: true,
            edit: MemoryMutationEdit::Approve,
        }
    }

    #[test]
    fn bridge_rejects_unconfirmed_and_wrong_capability_before_service_dispatch() {
        let mut unconfirmed = input();
        unconfirmed.confirmed = false;
        assert!(matches!(
            validate_mutation_input(&unconfirmed),
            Err(MemoryBridgeError::ConfirmationRequired)
        ));

        let mut wrong_capability = input();
        wrong_capability.capability = "memory.read".into();
        assert!(matches!(
            validate_mutation_input(&wrong_capability),
            Err(MemoryBridgeError::MutationRejected {
                code: DomainErrorCode::PermissionDenied
            })
        ));
    }
}
