//! Bounded Git worktree adapter over the structured process primitive.
//!
//! This module is the infrastructure boundary for materializing the pure
//! `agent_core::worktree::WorktreeRequest`. It never invokes a shell and never
//! accepts free-form command strings.

use crate::{PermissionDecision, ProcessError, ProcessSpec, run_process};
use agent_core::ids::ProjectId;
use agent_core::worktree::{WorktreeMode, WorktreeRequest};
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

pub const DEFAULT_MAX_WORKTREE_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeListEntry {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub locked: bool,
    pub prunable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeListResult {
    pub trace_id: TraceId,
    pub entries: Vec<GitWorktreeListEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeMutationResult {
    pub trace_id: TraceId,
    pub worktree_path: String,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitWorktreeError {
    #[error("project is not authorized")]
    ProjectUnauthorized,
    #[error("permission denied")]
    PermissionDenied,
    #[error("repository is invalid")]
    InvalidRepository,
    #[error("workspace root is invalid")]
    InvalidWorkspace,
    #[error("worktree request is invalid")]
    InvalidRequest,
    #[error("worktree output limit is invalid")]
    InvalidLimit,
    #[error("git worktree operation failed")]
    GitFailed,
    #[error("git worktree output was truncated")]
    OutputTruncated,
    #[error("git worktree output is malformed")]
    MalformedOutput,
    #[error("process failed")]
    Process(#[from] ProcessError),
}

#[derive(Debug, Clone)]
pub struct GitWorktreeTool {
    project_id: ProjectId,
    repository: PathBuf,
    workspace_root: PathBuf,
    git_program: PathBuf,
    max_output_bytes: usize,
}

impl GitWorktreeTool {
    pub fn new(
        project_id: ProjectId,
        repository: PathBuf,
        workspace_root: PathBuf,
        git_program: PathBuf,
        max_output_bytes: usize,
    ) -> Result<Self, GitWorktreeError> {
        if !repository.is_dir()
            || !repository.join(".git").exists()
            || !workspace_root.is_dir()
            || !repository.starts_with(&workspace_root)
        {
            return Err(GitWorktreeError::InvalidRepository);
        }
        if !git_program.is_file() {
            return Err(GitWorktreeError::InvalidRepository);
        }
        if max_output_bytes == 0 {
            return Err(GitWorktreeError::InvalidLimit);
        }
        Ok(Self {
            project_id,
            repository,
            workspace_root,
            git_program,
            max_output_bytes,
        })
    }

    pub fn add(
        &self,
        request: WorktreeRequest,
        permission: PermissionDecision,
        trace_id: TraceId,
        cancel: Arc<AtomicBool>,
    ) -> Result<GitWorktreeMutationResult, GitWorktreeError> {
        self.validate_request(&request, &permission)?;
        let mut args = vec!["worktree".to_owned(), "add".to_owned()];
        if let WorktreeMode::Branch { branch } = &request.mode {
            args.push("-b".to_owned());
            args.push(branch.clone());
        } else {
            args.push("--detach".to_owned());
        }
        args.push(request.worktree_path.clone());
        let process = self.run_git(args, permission, trace_id, cancel, Duration::from_secs(30))?;
        if process.stdout_truncated || process.stderr_truncated {
            return Err(GitWorktreeError::OutputTruncated);
        }
        Ok(GitWorktreeMutationResult {
            trace_id,
            worktree_path: request.worktree_path,
            truncated: false,
        })
    }

    pub fn list(
        &self,
        project_id: ProjectId,
        permission: PermissionDecision,
        trace_id: TraceId,
        cancel: Arc<AtomicBool>,
    ) -> Result<GitWorktreeListResult, GitWorktreeError> {
        self.validate_project_and_permission(project_id, &permission)?;
        let process = self.run_git(
            vec!["worktree".into(), "list".into(), "--porcelain".into()],
            permission,
            trace_id,
            cancel,
            Duration::from_secs(5),
        )?;
        if process.stdout_truncated || process.stderr_truncated {
            return Err(GitWorktreeError::OutputTruncated);
        }
        Ok(GitWorktreeListResult {
            trace_id,
            entries: parse_worktree_porcelain(&process.stdout)?,
        })
    }

    pub fn remove(
        &self,
        request: WorktreeRequest,
        permission: PermissionDecision,
        trace_id: TraceId,
        cancel: Arc<AtomicBool>,
    ) -> Result<GitWorktreeMutationResult, GitWorktreeError> {
        self.validate_request(&request, &permission)?;
        let process = self.run_git(
            vec![
                "worktree".into(),
                "remove".into(),
                request.worktree_path.clone(),
            ],
            permission,
            trace_id,
            cancel,
            Duration::from_secs(30),
        )?;
        if process.stdout_truncated || process.stderr_truncated {
            return Err(GitWorktreeError::OutputTruncated);
        }
        Ok(GitWorktreeMutationResult {
            trace_id,
            worktree_path: request.worktree_path,
            truncated: false,
        })
    }

    fn validate_project_and_permission(
        &self,
        project_id: ProjectId,
        permission: &PermissionDecision,
    ) -> Result<(), GitWorktreeError> {
        if project_id != self.project_id {
            return Err(GitWorktreeError::ProjectUnauthorized);
        }
        if !permission.is_allowed() {
            return Err(GitWorktreeError::PermissionDenied);
        }
        Ok(())
    }

    fn validate_request(
        &self,
        request: &WorktreeRequest,
        permission: &PermissionDecision,
    ) -> Result<(), GitWorktreeError> {
        if request.project_id != self.project_id.to_string() {
            return Err(GitWorktreeError::ProjectUnauthorized);
        }
        if !permission.is_allowed() {
            return Err(GitWorktreeError::PermissionDenied);
        }
        request
            .validate()
            .map_err(|_| GitWorktreeError::InvalidRequest)?;
        if Path::new(&request.workspace_root) != self.workspace_root
            || !Path::new(&request.worktree_path).starts_with(&self.workspace_root)
        {
            return Err(GitWorktreeError::InvalidWorkspace);
        }
        Ok(())
    }

    fn run_git(
        &self,
        args: Vec<String>,
        permission: PermissionDecision,
        trace_id: TraceId,
        cancel: Arc<AtomicBool>,
        timeout: Duration,
    ) -> Result<crate::ProcessResult, GitWorktreeError> {
        let spec = ProcessSpec {
            project_id: self.project_id,
            program: self.git_program.clone(),
            args,
            cwd: self.repository.clone(),
            env: BTreeMap::from([
                (String::from("GIT_OPTIONAL_LOCKS"), String::from("0")),
                (String::from("GIT_TERMINAL_PROMPT"), String::from("0")),
            ]),
            allowed_programs: BTreeSet::from([self.git_program.clone()]),
            allowed_roots: vec![self.workspace_root.clone()],
            permission,
            timeout,
            max_output_bytes: self.max_output_bytes,
            trace_id,
        };
        let process = run_process(&spec, cancel)?;
        if process.timed_out || process.cancelled || process.exit_code != Some(0) {
            return Err(GitWorktreeError::GitFailed);
        }
        Ok(process)
    }
}

/// Parses bounded `git worktree list --porcelain` output fail-closed.
pub fn parse_worktree_porcelain(
    output: &str,
) -> Result<Vec<GitWorktreeListEntry>, GitWorktreeError> {
    let mut entries = Vec::new();
    let mut current: Option<GitWorktreeListEntry> = None;

    let finish = |current: &mut Option<GitWorktreeListEntry>,
                  entries: &mut Vec<GitWorktreeListEntry>|
     -> Result<(), GitWorktreeError> {
        if let Some(entry) = current.take() {
            if entry.head.is_empty() || entry.path.is_empty() {
                return Err(GitWorktreeError::MalformedOutput);
            }
            entries.push(entry);
        }
        Ok(())
    };

    for raw_line in output.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            finish(&mut current, &mut entries)?;
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            finish(&mut current, &mut entries)?;
            if path.is_empty() || path.chars().any(char::is_control) {
                return Err(GitWorktreeError::MalformedOutput);
            }
            current = Some(GitWorktreeListEntry {
                path: path.to_owned(),
                head: String::new(),
                branch: None,
                detached: false,
                bare: false,
                locked: false,
                prunable: false,
            });
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            let entry = current.as_mut().ok_or(GitWorktreeError::MalformedOutput)?;
            if head.len() != 40 && head.len() != 64
                || !head.chars().all(|character| character.is_ascii_hexdigit())
            {
                return Err(GitWorktreeError::MalformedOutput);
            }
            entry.head = head.to_owned();
        } else if let Some(branch) = line.strip_prefix("branch ") {
            let entry = current.as_mut().ok_or(GitWorktreeError::MalformedOutput)?;
            let branch = branch
                .strip_prefix("refs/heads/")
                .ok_or(GitWorktreeError::MalformedOutput)?;
            if branch.is_empty() {
                return Err(GitWorktreeError::MalformedOutput);
            }
            entry.branch = Some(branch.to_owned());
        } else if line == "detached" {
            current
                .as_mut()
                .ok_or(GitWorktreeError::MalformedOutput)?
                .detached = true;
        } else if line == "bare" {
            current
                .as_mut()
                .ok_or(GitWorktreeError::MalformedOutput)?
                .bare = true;
        } else if line == "locked" || line.starts_with("locked ") {
            current
                .as_mut()
                .ok_or(GitWorktreeError::MalformedOutput)?
                .locked = true;
        } else if line == "prunable" || line.starts_with("prunable ") {
            current
                .as_mut()
                .ok_or(GitWorktreeError::MalformedOutput)?
                .prunable = true;
        } else {
            return Err(GitWorktreeError::MalformedOutput);
        }
    }
    finish(&mut current, &mut entries)?;
    if entries.is_empty() {
        return Err(GitWorktreeError::MalformedOutput);
    }
    Ok(entries)
}
