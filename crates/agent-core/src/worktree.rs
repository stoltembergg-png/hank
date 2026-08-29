//! Registry puro para intenções de Git worktree.
//!
//! Esta camada não executa Git nem acessa filesystem. Ela valida a associação
//! task/workspace/owner e garante que um worktree pretendido permaneça dentro
//! da raiz autorizada recebida pelo adapter de infraestrutura.

use crate::{DomainError, DomainResult};
use std::collections::BTreeMap;

pub const MAX_WORKTREE_ID_LEN: usize = 128;
pub const MAX_TASK_ID_LEN: usize = 128;
pub const MAX_WORKTREE_PROJECT_ID_LEN: usize = 128;
pub const MAX_WORKTREE_OWNER_ID_LEN: usize = 128;
pub const MAX_WORKTREE_PATH_LEN: usize = 4096;
pub const MAX_WORKTREE_BRANCH_LEN: usize = 256;
pub const MAX_WORKTREE_REGISTRY_CAPACITY: usize = 1024;
const MAX_OBSERVED_WORKTREE_PATHS: usize = 1024;

/// Modo de associação que o adapter Git deverá materializar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeMode {
    /// Worktree detached, sem criar ou associar branch.
    Detached,
    /// Worktree associado a uma branch já validada pelo domínio.
    Branch { branch: String },
}

/// Pedido bounded de reserva de um worktree isolado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRequest {
    pub worktree_id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub task_id: String,
    pub owner_id: String,
    pub workspace_root: String,
    pub worktree_path: String,
    pub mode: WorktreeMode,
}

impl WorktreeRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        worktree_id: impl Into<String>,
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        task_id: impl Into<String>,
        owner_id: impl Into<String>,
        workspace_root: impl Into<String>,
        worktree_path: impl Into<String>,
        mode: WorktreeMode,
    ) -> Self {
        Self {
            worktree_id: worktree_id.into(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            task_id: task_id.into(),
            owner_id: owner_id.into(),
            workspace_root: workspace_root.into(),
            worktree_path: worktree_path.into(),
            mode,
        }
    }

    pub fn validate(&self) -> DomainResult<()> {
        validate_id("worktree_id", &self.worktree_id, MAX_WORKTREE_ID_LEN)?;
        validate_id("workspace_id", &self.workspace_id, MAX_WORKTREE_ID_LEN)?;
        validate_id("project_id", &self.project_id, MAX_WORKTREE_PROJECT_ID_LEN)?;
        validate_id("task_id", &self.task_id, MAX_TASK_ID_LEN)?;
        validate_id("owner_id", &self.owner_id, MAX_WORKTREE_OWNER_ID_LEN)?;
        validate_path("workspace_root", &self.workspace_root)?;
        validate_path("worktree_path", &self.worktree_path)?;
        if !is_strictly_within(&self.workspace_root, &self.worktree_path) {
            return Err(DomainError::Validation(
                "worktree_path deve estar estritamente dentro do workspace_root".into(),
            ));
        }
        if let WorktreeMode::Branch { branch } = &self.mode {
            validate_branch(branch)?;
        }
        Ok(())
    }
}

/// Registro imutável da intenção de um worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRecord {
    request: WorktreeRequest,
}

impl WorktreeRecord {
    pub fn request(&self) -> &WorktreeRequest {
        &self.request
    }

    pub fn worktree_id(&self) -> &str {
        &self.request.worktree_id
    }

    pub fn workspace_id(&self) -> &str {
        &self.request.workspace_id
    }

    pub fn project_id(&self) -> &str {
        &self.request.project_id
    }

    pub fn task_id(&self) -> &str {
        &self.request.task_id
    }

    pub fn owner_id(&self) -> &str {
        &self.request.owner_id
    }

    pub fn workspace_root(&self) -> &str {
        &self.request.workspace_root
    }

    pub fn worktree_path(&self) -> &str {
        &self.request.worktree_path
    }

    pub fn mode(&self) -> &WorktreeMode {
        &self.request.mode
    }
}

/// Ação segura produzida pelo plano de recuperação dry-run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeRecoveryAction {
    /// Pode ser entregue ao adapter de remoção sem `force`.
    RemoveRegistered {
        worktree_id: String,
        worktree_path: String,
    },
    /// Deve permanecer intocado até uma autorização/registro explícitos.
    PreserveUnknown { worktree_path: String },
}

/// Registro bounded e determinístico de intenções de worktree.
#[derive(Debug)]
pub struct WorktreeRegistry {
    capacity: usize,
    records: BTreeMap<String, WorktreeRecord>,
}

impl WorktreeRegistry {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            records: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn get(&self, worktree_id: &str) -> Option<&WorktreeRecord> {
        self.records.get(worktree_id)
    }

    pub fn list(&self) -> Vec<WorktreeRecord> {
        self.records.values().cloned().collect()
    }

    /// Constrói um plano dry-run sem tocar no registry ou no filesystem.
    pub fn recovery_plan(
        &self,
        project_id: &str,
        owner_id: &str,
        observed_paths: &[String],
    ) -> DomainResult<Vec<WorktreeRecoveryAction>> {
        validate_id(
            "recovery_project_id",
            project_id,
            MAX_WORKTREE_PROJECT_ID_LEN,
        )?;
        validate_id("recovery_owner_id", owner_id, MAX_WORKTREE_OWNER_ID_LEN)?;
        if observed_paths.len() > MAX_OBSERVED_WORKTREE_PATHS {
            return Err(DomainError::Validation(
                "quantidade de paths observados excede o limite".into(),
            ));
        }
        for path in observed_paths {
            validate_path("observed_worktree_path", path)?;
        }

        let mut actions = BTreeMap::new();
        for path in observed_paths {
            let registered = self
                .records
                .values()
                .find(|record| record.project_id() == project_id && record.worktree_path() == path);
            let (priority, action) = match registered {
                Some(record) if record.owner_id() == owner_id => (
                    1_u8,
                    WorktreeRecoveryAction::RemoveRegistered {
                        worktree_id: record.worktree_id().to_owned(),
                        worktree_path: path.clone(),
                    },
                ),
                _ => (
                    0_u8,
                    WorktreeRecoveryAction::PreserveUnknown {
                        worktree_path: path.clone(),
                    },
                ),
            };
            actions.insert((priority, path.clone()), action);
        }
        Ok(actions.into_values().collect())
    }

    /// Registra ou retorna o mesmo registro quando o request é idêntico.
    pub fn register(&mut self, request: WorktreeRequest) -> DomainResult<WorktreeRecord> {
        request.validate()?;

        if let Some(existing) = self.records.get(&request.worktree_id) {
            if existing.request == request {
                return Ok(existing.clone());
            }
            return Err(DomainError::Duplicate(format!(
                "worktree_id já reservado: {}",
                request.worktree_id
            )));
        }

        if self.capacity == 0 || self.records.len() >= self.capacity {
            return Err(DomainError::Validation(
                "capacidade do registry de worktree excedida".into(),
            ));
        }

        if self
            .records
            .values()
            .any(|record| record.task_id() == request.task_id)
        {
            return Err(DomainError::Duplicate(format!(
                "task já possui worktree: {}",
                request.task_id
            )));
        }
        if self
            .records
            .values()
            .any(|record| record.worktree_path() == request.worktree_path)
        {
            return Err(DomainError::Duplicate(format!(
                "worktree_path já reservado: {}",
                request.worktree_path
            )));
        }
        if let WorktreeMode::Branch { branch } = &request.mode {
            if self.records.values().any(|record| {
                matches!(record.mode(), WorktreeMode::Branch { branch: current } if current == branch)
            }) {
                return Err(DomainError::Duplicate(format!(
                    "branch já reservada: {branch}"
                )));
            }
        }

        let record = WorktreeRecord { request };
        self.records
            .insert(record.worktree_id().to_owned(), record.clone());
        Ok(record)
    }
}

fn validate_id(field: &str, value: &str, max_len: usize) -> DomainResult<()> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(DomainError::Validation(format!("{field} inválido")));
    }
    Ok(())
}

fn validate_path(field: &str, value: &str) -> DomainResult<()> {
    if value.trim().is_empty()
        || value.len() > MAX_WORKTREE_PATH_LEN
        || value.chars().any(char::is_control)
        || !looks_absolute(value)
        || value
            .split(['/', '\\'])
            .any(|segment| segment == "." || segment == "..")
    {
        return Err(DomainError::Validation(format!(
            "{field} deve ser absoluto, bounded e sem traversal"
        )));
    }
    Ok(())
}

fn validate_branch(branch: &str) -> DomainResult<()> {
    if branch.trim().is_empty()
        || branch.len() > MAX_WORKTREE_BRANCH_LEN
        || branch.chars().any(char::is_control)
        || branch.chars().any(char::is_whitespace)
        || branch.contains("..")
        || branch.starts_with('-')
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch.ends_with('.')
        || branch.contains(['~', '^', ':', '?', '*', '[', '\\'])
    {
        return Err(DomainError::Validation(
            "branch de worktree inválida ou não bounded".into(),
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

fn is_strictly_within(root: &str, child: &str) -> bool {
    let root_parts = path_parts(root);
    let child_parts = path_parts(child);
    if child_parts.len() <= root_parts.len() {
        return false;
    }
    let windows_style = root.contains('\\')
        || child.contains('\\')
        || root.as_bytes().get(1) == Some(&b':')
        || child.as_bytes().get(1) == Some(&b':');
    root_parts
        .iter()
        .zip(child_parts.iter())
        .all(|(root, child)| {
            if windows_style {
                root.eq_ignore_ascii_case(child)
            } else {
                root == child
            }
        })
}

fn path_parts(value: &str) -> Vec<&str> {
    value
        .split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .collect()
}
