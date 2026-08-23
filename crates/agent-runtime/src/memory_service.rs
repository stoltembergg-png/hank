//! Explicit, project-scoped memory mutation application boundary.

use crate::memory_repo::SqliteMemoryRepository;
use agent_core::DomainError;
use agent_core::{Memory, MemoryId, ProjectId};

#[derive(Debug, Clone)]
pub struct MemoryMutationContext {
    pub project_id: ProjectId,
    pub actor_id: String,
    pub trace_id: String,
    pub capability: String,
    pub policy_allowed: bool,
    pub operation_id: String,
}

#[derive(Debug, Clone)]
pub enum MemoryEdit {
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

#[derive(Clone)]
pub struct MemoryMutationService {
    repository: SqliteMemoryRepository,
}

impl MemoryMutationService {
    pub fn new(repository: SqliteMemoryRepository) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        context: MemoryMutationContext,
        memory_id: MemoryId,
        expected_version: u64,
        edit: MemoryEdit,
    ) -> Result<Memory, DomainError> {
        validate_context(&context)?;
        let mut memory = self
            .repository
            .get(&context.project_id, &memory_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("memory".into()))?;
        if memory.version != expected_version {
            return Err(DomainError::ConcurrencyConflict {
                expected: expected_version.to_string(),
                actual: memory.version.to_string(),
            });
        }
        match edit {
            MemoryEdit::Update {
                content,
                summary,
                importance,
            } => {
                memory.content = content;
                memory.summary = summary;
                memory.importance = importance;
            }
            MemoryEdit::Approve => memory
                .approve(memory.importance, memory.summary.clone())
                .map_err(|error| DomainError::Validation(error.to_string()))?,
            MemoryEdit::Reject => memory.reject(),
            MemoryEdit::Archive => memory
                .archive()
                .map_err(|error| DomainError::Validation(error.to_string()))?,
            MemoryEdit::Restore => memory
                .restore()
                .map_err(|error| DomainError::Validation(error.to_string()))?,
        }
        if memory.version == expected_version {
            memory.version = memory.version.saturating_add(1);
        }
        memory.updated_at = chrono::Utc::now();
        memory
            .validate()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        self.repository.update(&memory, expected_version).await?;
        self.repository
            .get(&context.project_id, &memory_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("memory after mutation".into()))
    }
}

fn validate_context(context: &MemoryMutationContext) -> Result<(), DomainError> {
    if context.actor_id.trim().is_empty()
        || context.trace_id.trim().is_empty()
        || context.operation_id.trim().is_empty()
        || context.actor_id.len() > 128
        || context.trace_id.len() > 128
        || context.operation_id.len() > 128
        || !context.policy_allowed
    {
        return Err(DomainError::PermissionDenied {
            capability: context.capability.clone(),
            reason: "invalid or denied mutation context".into(),
        });
    }
    if context.capability != "memory.write" {
        return Err(DomainError::PermissionDenied {
            capability: context.capability.clone(),
            reason: "memory mutation capability required".into(),
        });
    }
    Ok(())
}
