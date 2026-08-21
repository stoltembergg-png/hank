//! Read-only bounded Git diff adapter over the process primitive.

use crate::{PermissionDecision, ProcessError, ProcessSpec, run_process};
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitDiffMode {
    Staged,
    Unstaged,
    Path,
}

#[derive(Debug, Clone)]
pub struct GitDiffRequest {
    pub project_id: ProjectId,
    pub mode: GitDiffMode,
    pub path: Option<String>,
    pub permission: PermissionDecision,
    pub trace_id: TraceId,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDiffResult {
    pub trace_id: TraceId,
    pub diff: String,
    pub bytes: usize,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitDiffError {
    #[error("project is not authorized")]
    ProjectUnauthorized,
    #[error("permission denied")]
    PermissionDenied,
    #[error("repository is invalid")]
    InvalidRepository,
    #[error("diff path is invalid")]
    InvalidPath,
    #[error("diff limit is invalid")]
    InvalidLimit,
    #[error("git diff failed")]
    GitFailed,
    #[error("process failed")]
    Process(#[from] ProcessError),
}

#[derive(Debug, Clone)]
pub struct GitDiffTool {
    project_id: ProjectId,
    repository: PathBuf,
    git_program: PathBuf,
}

impl GitDiffTool {
    pub fn new(
        project_id: ProjectId,
        repository: PathBuf,
        git_program: PathBuf,
    ) -> Result<Self, GitDiffError> {
        if !repository.is_dir() || !repository.join(".git").exists() {
            return Err(GitDiffError::InvalidRepository);
        }
        if !git_program.is_file() {
            return Err(GitDiffError::InvalidRepository);
        }
        Ok(Self {
            project_id,
            repository,
            git_program,
        })
    }

    pub fn diff(
        &self,
        request: GitDiffRequest,
        cancel: Arc<AtomicBool>,
    ) -> Result<GitDiffResult, GitDiffError> {
        if request.project_id != self.project_id {
            return Err(GitDiffError::ProjectUnauthorized);
        }
        if !request.permission.is_allowed() {
            return Err(GitDiffError::PermissionDenied);
        }
        if request.max_bytes == 0 {
            return Err(GitDiffError::InvalidLimit);
        }
        let mut args = vec![
            "diff".into(),
            "--no-ext-diff".into(),
            "--no-textconv".into(),
            "--unified=3".into(),
        ];
        match request.mode {
            GitDiffMode::Staged => args.push("--cached".into()),
            GitDiffMode::Unstaged => {}
            GitDiffMode::Path => {
                let path = request.path.as_deref().ok_or(GitDiffError::InvalidPath)?;
                if path.is_empty()
                    || std::path::Path::new(path).is_absolute()
                    || path.split('/').any(|part| part == ".." || part.is_empty())
                {
                    return Err(GitDiffError::InvalidPath);
                }
                args.extend(["--".into(), path.into()]);
            }
        }
        let spec = ProcessSpec {
            project_id: request.project_id,
            program: self.git_program.clone(),
            args,
            cwd: self.repository.clone(),
            env: BTreeMap::from([(String::from("GIT_OPTIONAL_LOCKS"), String::from("0"))]),
            allowed_programs: BTreeSet::from([self.git_program.clone()]),
            allowed_roots: vec![self.repository.clone()],
            permission: request.permission,
            timeout: Duration::from_secs(5),
            max_output_bytes: request.max_bytes,
            trace_id: request.trace_id,
        };
        let process = run_process(&spec, cancel)?;
        if process.timed_out || process.cancelled || process.exit_code != Some(0) {
            return Err(GitDiffError::GitFailed);
        }
        let redacted_diff = redact_diff(&process.stdout);
        let truncated = process.stdout_truncated || redacted_diff.len() > request.max_bytes;
        let mut bounded = redacted_diff.into_bytes();
        if bounded.len() > request.max_bytes {
            bounded.truncate(request.max_bytes);
        }
        if truncated {
            bounded.extend_from_slice(b"\n[truncated]\n");
        }
        let bytes = bounded.len();
        Ok(GitDiffResult {
            trace_id: request.trace_id,
            diff: String::from_utf8_lossy(&bounded).into_owned(),
            bytes,
            truncated,
        })
    }
}

fn redact_diff(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            let clean = line
                .chars()
                .filter(|character| !character.is_control() || *character == '\t')
                .collect::<String>();
            let lower = clean.to_ascii_lowercase();
            if ["secret", "token", "password", "api_key"]
                .iter()
                .any(|needle| lower.contains(needle))
            {
                "[redacted]".to_string()
            } else {
                clean
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
