//! Read-only Git status adapter over the structured process primitive.

use crate::{PermissionDecision, ProcessError, ProcessSpec, run_process};
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusEntry {
    pub index_status: char,
    pub worktree_status: char,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusResult {
    pub trace_id: TraceId,
    pub branch: String,
    pub entries: Vec<GitStatusEntry>,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitStatusError {
    #[error("project is not authorized")]
    ProjectUnauthorized,
    #[error("permission denied")]
    PermissionDenied,
    #[error("repository root is invalid")]
    InvalidRepository,
    #[error("git executable is not allowlisted")]
    GitNotAllowed,
    #[error("git status failed")]
    GitFailed,
    #[error("git status output is malformed")]
    MalformedOutput,
    #[error("git status entry limit is invalid")]
    InvalidLimit,
    #[error("process failed")]
    Process(#[from] ProcessError),
}

#[derive(Debug, Clone)]
pub struct GitStatusTool {
    project_id: ProjectId,
    repository: PathBuf,
    git_program: PathBuf,
    max_entries: usize,
}

impl GitStatusTool {
    pub fn new(
        project_id: ProjectId,
        repository: PathBuf,
        git_program: PathBuf,
        max_entries: usize,
    ) -> Result<Self, GitStatusError> {
        if !repository.is_dir() || !repository.join(".git").exists() {
            return Err(GitStatusError::InvalidRepository);
        }
        if max_entries == 0 {
            return Err(GitStatusError::InvalidLimit);
        }
        if !git_program.is_file() {
            return Err(GitStatusError::GitNotAllowed);
        }
        Ok(Self {
            project_id,
            repository,
            git_program,
            max_entries,
        })
    }

    pub fn status(
        &self,
        project_id: ProjectId,
        permission: PermissionDecision,
        trace_id: TraceId,
        cancel: Arc<AtomicBool>,
    ) -> Result<GitStatusResult, GitStatusError> {
        if project_id != self.project_id {
            return Err(GitStatusError::ProjectUnauthorized);
        }
        if !permission.is_allowed() {
            return Err(GitStatusError::PermissionDenied);
        }
        let spec = ProcessSpec {
            project_id,
            program: self.git_program.clone(),
            args: vec![
                "status".into(),
                "--porcelain=v1".into(),
                "-b".into(),
                "--untracked-files=normal".into(),
            ],
            cwd: self.repository.clone(),
            env: BTreeMap::from([(String::from("GIT_OPTIONAL_LOCKS"), String::from("0"))]),
            allowed_programs: BTreeSet::from([self.git_program.clone()]),
            allowed_roots: vec![self.repository.clone()],
            permission,
            timeout: Duration::from_secs(5),
            max_output_bytes: 256 * 1024,
            trace_id,
        };
        let process = run_process(&spec, cancel)?;
        if process.timed_out || process.cancelled || process.exit_code != Some(0) {
            return Err(GitStatusError::GitFailed);
        }
        parse_status(
            trace_id,
            &process.stdout,
            self.max_entries,
            process.stdout_truncated,
        )
    }
}

fn parse_status(
    trace_id: TraceId,
    output: &str,
    max_entries: usize,
    process_truncated: bool,
) -> Result<GitStatusResult, GitStatusError> {
    let mut lines = output.lines();
    let header = lines.next().ok_or(GitStatusError::MalformedOutput)?;
    let branch = header
        .strip_prefix("## ")
        .ok_or(GitStatusError::MalformedOutput)?
        .split("...")
        .next()
        .unwrap_or_default()
        .trim();
    if branch.is_empty() {
        return Err(GitStatusError::MalformedOutput);
    }
    let all_entries = lines
        .map(|line| {
            if line.len() < 4 {
                return Err(GitStatusError::MalformedOutput);
            }
            let bytes = line.as_bytes();
            let path = line[3..].trim();
            if path.is_empty() {
                return Err(GitStatusError::MalformedOutput);
            }
            Ok(GitStatusEntry {
                index_status: bytes[0] as char,
                worktree_status: bytes[1] as char,
                path: path.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let truncated = process_truncated || all_entries.len() > max_entries;
    let entries = all_entries.into_iter().take(max_entries).collect();
    Ok(GitStatusResult {
        trace_id,
        branch: branch.to_string(),
        entries,
        truncated,
    })
}
