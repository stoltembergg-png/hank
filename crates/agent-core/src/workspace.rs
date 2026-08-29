//! Boundary pura de ownership para workspaces de repositórios.
//!
//! A raiz recebida por este módulo já deve ter sido canonicalizada por um
//! adapter de infraestrutura. Este domínio não acessa filesystem, Git, shell,
//! storage ou secrets.

use crate::{DomainError, DomainResult};
use std::collections::BTreeMap;

pub const MAX_WORKSPACE_ID_LEN: usize = 128;
pub const MAX_PROJECT_ID_LEN: usize = 128;
pub const MAX_REPOSITORY_ID_LEN: usize = 128;
pub const MAX_CANONICAL_ROOT_LEN: usize = 4096;

/// Dados necessários para registrar um workspace project-scoped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRegistration {
    workspace_id: String,
    project_id: String,
    repository_id: String,
    canonical_root: String,
}

impl WorkspaceRegistration {
    pub fn new(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        repository_id: impl Into<String>,
        canonical_root: impl Into<String>,
    ) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            repository_id: repository_id.into(),
            canonical_root: canonical_root.into(),
        }
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    pub fn canonical_root(&self) -> &str {
        &self.canonical_root
    }
}

/// Snapshot de ownership e lease de um workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    registration: WorkspaceRegistration,
    active_lease: Option<WorkspaceLease>,
    next_epoch: u64,
}

impl WorkspaceRecord {
    pub fn workspace_id(&self) -> &str {
        self.registration.workspace_id()
    }

    pub fn project_id(&self) -> &str {
        self.registration.project_id()
    }

    pub fn repository_id(&self) -> &str {
        self.registration.repository_id()
    }

    pub fn canonical_root(&self) -> &str {
        self.registration.canonical_root()
    }
}

/// Token opaco que identifica uma aquisição exata de lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLease {
    workspace_id: String,
    holder_id: String,
    epoch: u64,
}

impl WorkspaceLease {
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn holder_id(&self) -> &str {
        &self.holder_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// Manager bounded de registros e leases em memória.
#[derive(Debug, Default)]
pub struct WorkspaceManager {
    workspaces: BTreeMap<String, WorkspaceRecord>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.workspaces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.workspaces.is_empty()
    }

    pub fn get(&self, workspace_id: &str) -> Option<&WorkspaceRecord> {
        self.workspaces.get(workspace_id)
    }

    pub fn register(&mut self, registration: WorkspaceRegistration) -> DomainResult<()> {
        validate_id(
            "workspace_id",
            registration.workspace_id(),
            MAX_WORKSPACE_ID_LEN,
        )?;
        validate_id("project_id", registration.project_id(), MAX_PROJECT_ID_LEN)?;
        validate_id(
            "repository_id",
            registration.repository_id(),
            MAX_REPOSITORY_ID_LEN,
        )?;
        validate_canonical_root(registration.canonical_root())?;

        if self.workspaces.contains_key(registration.workspace_id()) {
            return Err(DomainError::Duplicate(format!(
                "workspace já registrado: {}",
                registration.workspace_id()
            )));
        }

        if self
            .workspaces
            .values()
            .any(|record| record.canonical_root() == registration.canonical_root())
        {
            return Err(DomainError::Validation(
                "canonical_root já está vinculada a outro workspace".into(),
            ));
        }

        let workspace_id = registration.workspace_id().to_owned();
        self.workspaces.insert(
            workspace_id,
            WorkspaceRecord {
                registration,
                active_lease: None,
                next_epoch: 1,
            },
        );
        Ok(())
    }

    pub fn acquire_lease(
        &mut self,
        workspace_id: &str,
        holder_id: &str,
    ) -> DomainResult<WorkspaceLease> {
        validate_id("workspace_id", workspace_id, MAX_WORKSPACE_ID_LEN)?;
        validate_id("holder_id", holder_id, MAX_WORKSPACE_ID_LEN)?;

        let workspace = self.workspaces.get_mut(workspace_id).ok_or_else(|| {
            DomainError::NotFound(format!("workspace não encontrado: {workspace_id}"))
        })?;

        if let Some(active) = &workspace.active_lease {
            return Err(DomainError::ConcurrencyConflict {
                expected: format!("workspace livre: {workspace_id}"),
                actual: format!("holder ativo: {}", active.holder_id()),
            });
        }

        let lease = WorkspaceLease {
            workspace_id: workspace_id.to_owned(),
            holder_id: holder_id.to_owned(),
            epoch: workspace.next_epoch,
        };
        workspace.next_epoch = workspace
            .next_epoch
            .checked_add(1)
            .ok_or_else(|| DomainError::InvariantViolation("epoch de lease esgotado".into()))?;
        workspace.active_lease = Some(lease.clone());
        Ok(lease)
    }

    pub fn release_lease(&mut self, lease: &WorkspaceLease) -> DomainResult<()> {
        let workspace = self
            .workspaces
            .get_mut(lease.workspace_id())
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "workspace não encontrado: {}",
                    lease.workspace_id()
                ))
            })?;

        if workspace.active_lease.as_ref() != Some(lease) {
            return Err(DomainError::ConcurrencyConflict {
                expected: format!("token ativo para {}", lease.workspace_id()),
                actual: "token ausente ou divergente".into(),
            });
        }

        workspace.active_lease = None;
        Ok(())
    }

    pub fn active_holder(&self, workspace_id: &str) -> Option<&str> {
        self.workspaces
            .get(workspace_id)
            .and_then(|workspace| workspace.active_lease.as_ref())
            .map(WorkspaceLease::holder_id)
    }
}

fn validate_id(field: &str, value: &str, max_len: usize) -> DomainResult<()> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(DomainError::Validation(format!("{field} inválido")));
    }
    Ok(())
}

fn validate_canonical_root(value: &str) -> DomainResult<()> {
    if value.trim().is_empty()
        || value.len() > MAX_CANONICAL_ROOT_LEN
        || value.chars().any(char::is_control)
        || !looks_absolute(value)
        || value
            .split(['/', '\\'])
            .any(|segment| segment == "." || segment == "..")
    {
        return Err(DomainError::Validation(
            "canonical_root deve ser absoluta, bounded e sem traversal".into(),
        ));
    }
    Ok(())
}

fn looks_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with('/')
        || value.starts_with("\\\\")
        || (bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'/' || bytes[2] == b'\\'))
}
