//! Contrato puro e bounded para vincular task, repositório, worktree e branch.
//!
//! Este módulo não executa Git, acessa filesystem, persiste dados ou concede
//! capabilities. Persistência e adapters ficam em boundaries externas.

use crate::{DomainError, DomainResult, ProjectId, RunId, TaskId, TraceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_TASK_MAPPING_REPOSITORY_ID_LEN: usize = 128;
pub const MAX_TASK_MAPPING_WORKTREE_ID_LEN: usize = 128;
pub const MAX_TASK_MAPPING_BRANCH_LEN: usize = 256;
pub const MAX_TASK_MAPPING_POLICY_REVISION_LEN: usize = 128;
pub const MAX_TASK_MAPPING_PULL_REQUEST_ID_LEN: usize = 128;
pub const MAX_TASK_MAPPING_REASON_LEN: usize = 256;
pub const MAX_TASK_MAPPINGS: usize = 1024;

/// Lifecycle explícito de um vínculo task/worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingState {
    Active,
    Detached,
    ReconcileRequired,
    Released,
}

/// Observação de metadados obtida por uma boundary externa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingObservation {
    repository_id: String,
    worktree_id: String,
    branch: String,
    observed_at_ms: u64,
    correlation_id: TraceId,
}

impl MappingObservation {
    pub fn new(
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        observed_at_ms: u64,
        correlation_id: TraceId,
    ) -> DomainResult<Self> {
        let value = Self {
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            observed_at_ms,
            correlation_id,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    pub fn worktree_id(&self) -> &str {
        &self.worktree_id
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn observed_at_ms(&self) -> u64 {
        self.observed_at_ms
    }

    pub fn correlation_id(&self) -> TraceId {
        self.correlation_id
    }

    fn validate(&self) -> DomainResult<()> {
        validate_identifier(
            "observed repository_id",
            &self.repository_id,
            MAX_TASK_MAPPING_REPOSITORY_ID_LEN,
        )?;
        validate_identifier(
            "observed worktree_id",
            &self.worktree_id,
            MAX_TASK_MAPPING_WORKTREE_ID_LEN,
        )?;
        validate_branch(&self.branch)?;
        Ok(())
    }
}

/// Autorização explícita para rebind; texto não concede capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRebindAuthorization {
    policy_revision: String,
    reason: String,
}

impl MappingRebindAuthorization {
    pub fn new(policy_revision: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            policy_revision: policy_revision.into(),
            reason: reason.into(),
        }
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Snapshot persistível do mapping, sem conteúdo de arquivos ou payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskWorkspaceMapping {
    project_id: ProjectId,
    task_id: TaskId,
    repository_id: String,
    worktree_id: String,
    branch: String,
    agent_run_id: RunId,
    pull_request_id: Option<String>,
    correlation_id: TraceId,
    policy_revision: String,
    state: MappingState,
    revision: u64,
    observation: Option<MappingObservation>,
    reconcile_reason: Option<String>,
    last_reconciled_at_ms: Option<u64>,
    last_resumed_at_ms: Option<u64>,
}

impl TaskWorkspaceMapping {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        agent_run_id: RunId,
        pull_request_id: Option<String>,
        correlation_id: TraceId,
        policy_revision: impl Into<String>,
    ) -> DomainResult<Self> {
        let value = Self {
            project_id,
            task_id,
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            agent_run_id,
            pull_request_id,
            correlation_id,
            policy_revision: policy_revision.into(),
            state: MappingState::Active,
            revision: 1,
            observation: None,
            reconcile_reason: None,
            last_reconciled_at_ms: None,
            last_resumed_at_ms: None,
        };
        value.validate()?;
        Ok(value)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        agent_run_id: RunId,
        pull_request_id: Option<String>,
        correlation_id: TraceId,
        policy_revision: impl Into<String>,
        state: MappingState,
        revision: u64,
        observation: Option<MappingObservation>,
        reconcile_reason: Option<String>,
        last_reconciled_at_ms: Option<u64>,
        last_resumed_at_ms: Option<u64>,
    ) -> DomainResult<Self> {
        let value = Self {
            project_id,
            task_id,
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            agent_run_id,
            pull_request_id,
            correlation_id,
            policy_revision: policy_revision.into(),
            state,
            revision,
            observation,
            reconcile_reason,
            last_reconciled_at_ms,
            last_resumed_at_ms,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    pub fn worktree_id(&self) -> &str {
        &self.worktree_id
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn agent_run_id(&self) -> RunId {
        self.agent_run_id
    }

    pub fn pull_request_id(&self) -> Option<&str> {
        self.pull_request_id.as_deref()
    }

    pub fn correlation_id(&self) -> TraceId {
        self.correlation_id
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn state(&self) -> MappingState {
        self.state
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn observation(&self) -> Option<&MappingObservation> {
        self.observation.as_ref()
    }

    pub fn reconcile_reason(&self) -> Option<&str> {
        self.reconcile_reason.as_deref()
    }

    pub fn last_reconciled_at_ms(&self) -> Option<u64> {
        self.last_reconciled_at_ms
    }

    pub fn last_resumed_at_ms(&self) -> Option<u64> {
        self.last_resumed_at_ms
    }

    fn validate(&self) -> DomainResult<()> {
        validate_identifier(
            "repository_id",
            &self.repository_id,
            MAX_TASK_MAPPING_REPOSITORY_ID_LEN,
        )?;
        validate_identifier(
            "worktree_id",
            &self.worktree_id,
            MAX_TASK_MAPPING_WORKTREE_ID_LEN,
        )?;
        validate_branch(&self.branch)?;
        validate_identifier(
            "policy_revision",
            &self.policy_revision,
            MAX_TASK_MAPPING_POLICY_REVISION_LEN,
        )?;
        if let Some(pull_request_id) = &self.pull_request_id {
            validate_identifier(
                "pull_request_id",
                pull_request_id,
                MAX_TASK_MAPPING_PULL_REQUEST_ID_LEN,
            )?;
        }
        if self.revision == 0 {
            return Err(DomainError::Validation(
                "mapping revision deve ser positiva".into(),
            ));
        }
        if let Some(observation) = &self.observation {
            observation.validate()?;
        }
        if let Some(reason) = &self.reconcile_reason {
            validate_reason(reason)?;
        }
        if self.state == MappingState::ReconcileRequired
            && (self.observation.is_none() || self.reconcile_reason.is_none())
        {
            return Err(DomainError::Validation(
                "mapping em reconcile_required exige observação e razão".into(),
            ));
        }
        Ok(())
    }
}

/// Registry puro, bounded e determinístico de mappings ativos.
#[derive(Debug)]
pub struct TaskWorkspaceMappingRegistry {
    mappings: BTreeMap<String, TaskWorkspaceMapping>,
    capacity: usize,
}

impl TaskWorkspaceMappingRegistry {
    pub fn new(capacity: usize) -> DomainResult<Self> {
        if capacity == 0 || capacity > MAX_TASK_MAPPINGS {
            return Err(DomainError::Validation(
                "capacidade de task mappings inválida".into(),
            ));
        }
        Ok(Self {
            mappings: BTreeMap::new(),
            capacity,
        })
    }

    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    pub fn register(&mut self, mapping: TaskWorkspaceMapping) -> DomainResult<()> {
        mapping.validate()?;
        if self.mappings.len() >= self.capacity {
            return Err(DomainError::Validation(
                "capacidade de task mappings excedida".into(),
            ));
        }
        let task_key = task_key(mapping.project_id(), mapping.task_id());
        if self.mappings.contains_key(&task_key) {
            return Err(DomainError::Duplicate(format!(
                "task mapping já existe: {}",
                mapping.task_id()
            )));
        }
        if self.has_conflict(&mapping, None) {
            return Err(DomainError::Duplicate(
                "worktree ou branch já está vinculado no projeto".into(),
            ));
        }
        self.mappings.insert(task_key, mapping);
        Ok(())
    }

    pub fn get(&self, project_id: ProjectId, task_id: TaskId) -> Option<&TaskWorkspaceMapping> {
        self.mappings.get(&task_key(project_id, task_id))
    }

    pub fn list(&self, project_id: ProjectId) -> DomainResult<Vec<TaskWorkspaceMapping>> {
        Ok(self
            .mappings
            .values()
            .filter(|mapping| mapping.project_id() == project_id)
            .cloned()
            .collect())
    }

    pub fn detach(
        &mut self,
        project_id: ProjectId,
        task_id: TaskId,
        expected_revision: u64,
        _at_ms: u64,
    ) -> DomainResult<TaskWorkspaceMapping> {
        let mapping = self.mapping_mut(project_id, task_id)?;
        ensure_revision(mapping, expected_revision)?;
        if mapping.state != MappingState::Active {
            return Err(invalid_transition(mapping.state, MappingState::Detached));
        }
        mapping.state = MappingState::Detached;
        mapping.revision = next_revision(mapping.revision)?;
        Ok(mapping.clone())
    }

    pub fn resume(
        &mut self,
        project_id: ProjectId,
        task_id: TaskId,
        expected_revision: u64,
        at_ms: u64,
    ) -> DomainResult<TaskWorkspaceMapping> {
        let mapping = self.mapping_mut(project_id, task_id)?;
        ensure_revision(mapping, expected_revision)?;
        if mapping.state != MappingState::Detached {
            return Err(invalid_transition(mapping.state, MappingState::Active));
        }
        mapping.state = MappingState::Active;
        mapping.last_resumed_at_ms = Some(at_ms);
        mapping.revision = next_revision(mapping.revision)?;
        Ok(mapping.clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rebind(
        &mut self,
        project_id: ProjectId,
        task_id: TaskId,
        expected_revision: u64,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        pull_request_id: Option<String>,
        authorization: MappingRebindAuthorization,
        _at_ms: u64,
    ) -> DomainResult<TaskWorkspaceMapping> {
        let repository_id = repository_id.into();
        let worktree_id = worktree_id.into();
        let branch = branch.into();
        validate_identifier(
            "repository_id",
            &repository_id,
            MAX_TASK_MAPPING_REPOSITORY_ID_LEN,
        )?;
        validate_identifier(
            "worktree_id",
            &worktree_id,
            MAX_TASK_MAPPING_WORKTREE_ID_LEN,
        )?;
        validate_branch(&branch)?;
        if let Some(value) = &pull_request_id {
            validate_identifier(
                "pull_request_id",
                value,
                MAX_TASK_MAPPING_PULL_REQUEST_ID_LEN,
            )?;
        }
        validate_identifier(
            "policy_revision",
            authorization.policy_revision(),
            MAX_TASK_MAPPING_POLICY_REVISION_LEN,
        )?;
        validate_reason(authorization.reason())?;

        let current = self.mapping(project_id, task_id)?.clone();
        ensure_revision(&current, expected_revision)?;
        if current.state == MappingState::Released {
            return Err(invalid_transition(current.state, MappingState::Active));
        }
        if current.policy_revision() != authorization.policy_revision() {
            return Err(DomainError::PermissionDenied {
                capability: "task_mapping_rebind".into(),
                reason: "rebind authorization uses stale policy revision".into(),
            });
        }
        let candidate = TaskWorkspaceMapping {
            repository_id,
            worktree_id,
            branch,
            pull_request_id,
            state: MappingState::Active,
            observation: None,
            reconcile_reason: None,
            revision: current.revision(),
            ..current.clone()
        };
        if self.has_conflict(&candidate, Some(task_id)) {
            return Err(DomainError::Duplicate(
                "novo worktree ou branch já está vinculado no projeto".into(),
            ));
        }
        let mapping = self.mapping_mut(project_id, task_id)?;
        *mapping = candidate;
        mapping.revision = next_revision(mapping.revision)?;
        Ok(mapping.clone())
    }

    pub fn reconcile(
        &mut self,
        project_id: ProjectId,
        task_id: TaskId,
        expected_revision: u64,
        observation: MappingObservation,
    ) -> DomainResult<TaskWorkspaceMapping> {
        observation.validate()?;
        let mapping = self.mapping_mut(project_id, task_id)?;
        ensure_revision(mapping, expected_revision)?;
        if mapping.state != MappingState::Active {
            return Err(invalid_transition(
                mapping.state,
                MappingState::ReconcileRequired,
            ));
        }
        let matches = mapping.repository_id() == observation.repository_id()
            && mapping.worktree_id() == observation.worktree_id()
            && mapping.branch() == observation.branch();
        mapping.observation = Some(observation.clone());
        mapping.last_reconciled_at_ms = Some(observation.observed_at_ms());
        if matches {
            mapping.reconcile_reason = None;
            mapping.state = MappingState::Active;
        } else {
            mapping.reconcile_reason = Some("observed identity mismatch".into());
            mapping.state = MappingState::ReconcileRequired;
        }
        mapping.revision = next_revision(mapping.revision)?;
        Ok(mapping.clone())
    }

    pub fn release(
        &mut self,
        project_id: ProjectId,
        task_id: TaskId,
        expected_revision: u64,
        _at_ms: u64,
    ) -> DomainResult<TaskWorkspaceMapping> {
        let mapping = self.mapping_mut(project_id, task_id)?;
        ensure_revision(mapping, expected_revision)?;
        if mapping.state == MappingState::Released {
            return Err(invalid_transition(mapping.state, MappingState::Released));
        }
        mapping.state = MappingState::Released;
        mapping.revision = next_revision(mapping.revision)?;
        Ok(mapping.clone())
    }

    fn mapping(
        &self,
        project_id: ProjectId,
        task_id: TaskId,
    ) -> DomainResult<&TaskWorkspaceMapping> {
        self.get(project_id, task_id)
            .ok_or_else(|| DomainError::NotFound(format!("task mapping não encontrado: {task_id}")))
    }

    fn mapping_mut(
        &mut self,
        project_id: ProjectId,
        task_id: TaskId,
    ) -> DomainResult<&mut TaskWorkspaceMapping> {
        self.mappings
            .get_mut(&task_key(project_id, task_id))
            .ok_or_else(|| DomainError::NotFound(format!("task mapping não encontrado: {task_id}")))
    }

    fn has_conflict(
        &self,
        candidate: &TaskWorkspaceMapping,
        excluded_task_id: Option<TaskId>,
    ) -> bool {
        self.mappings.values().any(|existing| {
            existing.project_id() == candidate.project_id()
                && Some(existing.task_id()) != excluded_task_id
                && (existing.worktree_id() == candidate.worktree_id()
                    || (existing.repository_id() == candidate.repository_id()
                        && existing.branch() == candidate.branch()))
        })
    }
}

impl Default for TaskWorkspaceMappingRegistry {
    fn default() -> Self {
        Self::new(MAX_TASK_MAPPINGS).expect("default mapping capacity is valid")
    }
}

fn task_key(project_id: ProjectId, task_id: TaskId) -> String {
    format!("{project_id}\0{task_id}")
}

fn ensure_revision(mapping: &TaskWorkspaceMapping, expected_revision: u64) -> DomainResult<()> {
    if mapping.revision() != expected_revision {
        return Err(DomainError::ConcurrencyConflict {
            expected: format!("revision {}", expected_revision),
            actual: format!("revision {}", mapping.revision()),
        });
    }
    Ok(())
}

fn next_revision(revision: u64) -> DomainResult<u64> {
    revision
        .checked_add(1)
        .ok_or_else(|| DomainError::InvariantViolation("mapping revision esgotada".into()))
}

fn invalid_transition(from: MappingState, to: MappingState) -> DomainError {
    DomainError::InvalidStateTransition {
        from: format!("{from:?}"),
        to: format!("{to:?}"),
    }
}

fn validate_identifier(field: &str, value: &str, max_len: usize) -> DomainResult<()> {
    if value.trim().is_empty()
        || value.len() > max_len
        || value.chars().any(char::is_control)
        || value.contains(['/', '\\'])
    {
        return Err(DomainError::Validation(format!("{field} inválido")));
    }
    Ok(())
}

fn validate_branch(value: &str) -> DomainResult<()> {
    if value.trim().is_empty()
        || value.len() > MAX_TASK_MAPPING_BRANCH_LEN
        || value.chars().any(char::is_control)
        || value.chars().any(char::is_whitespace)
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.ends_with('.')
        || value.contains("..")
        || value.contains(['~', '^', ':', '?', '*', '[', '\\'])
    {
        return Err(DomainError::Validation("branch inválida".into()));
    }
    Ok(())
}

fn validate_reason(value: &str) -> DomainResult<()> {
    if value.trim().is_empty()
        || value.len() > MAX_TASK_MAPPING_REASON_LEN
        || value.chars().any(char::is_control)
    {
        return Err(DomainError::Validation("razão de mapping inválida".into()));
    }
    Ok(())
}
