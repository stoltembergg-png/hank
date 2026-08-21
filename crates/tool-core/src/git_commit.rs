//! Explicit, authorized, reversible Git commit adapter over the process primitive.

use crate::{PermissionDecision, ProcessError, ProcessSpec, run_process};
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitRequest {
    pub project_id: ProjectId,
    pub paths: Vec<String>,
    pub message: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub permission: PermissionDecision,
    pub trace_id: TraceId,
    pub operation_key: String,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitResult {
    pub trace_id: TraceId,
    pub commit_hash: String,
    pub paths: Vec<String>,
    pub bytes: usize,
    pub operation_key: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GitCommitError {
    #[error("project is not authorized")]
    ProjectUnauthorized,
    #[error("permission denied")]
    PermissionDenied,
    #[error("repository is invalid")]
    InvalidRepository,
    #[error("commit paths are invalid")]
    InvalidPaths,
    #[error("commit message is invalid")]
    InvalidMessage,
    #[error("author identity is invalid")]
    InvalidAuthor,
    #[error("operation key is required")]
    MissingOperationKey,
    #[error("commit limit is invalid")]
    InvalidLimit,
    #[error("git commit failed")]
    GitFailed,
    #[error("process failed")]
    Process(#[from] ProcessError),
    #[error("duplicate operation key")]
    DuplicateOperationKey,
    #[error("concurrent commit detected")]
    ConcurrentCommit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GitCommitTool {
    project_id: ProjectId,
    repository: PathBuf,
    git_program: PathBuf,
}

impl GitCommitTool {
    pub fn new(
        project_id: ProjectId,
        repository: PathBuf,
        git_program: PathBuf,
    ) -> Result<Self, GitCommitError> {
        if !repository.is_dir() || !repository.join(".git").exists() {
            return Err(GitCommitError::InvalidRepository);
        }
        if !git_program.is_file() {
            return Err(GitCommitError::InvalidRepository);
        }
        Ok(Self {
            project_id,
            repository,
            git_program,
        })
    }

    pub fn commit(
        &self,
        request: GitCommitRequest,
        cancel: Arc<AtomicBool>,
    ) -> Result<GitCommitResult, GitCommitError> {
        if request.project_id != self.project_id {
            return Err(GitCommitError::ProjectUnauthorized);
        }
        if !request.permission.is_allowed() {
            return Err(GitCommitError::PermissionDenied);
        }
        if request.operation_key.trim().is_empty() {
            return Err(GitCommitError::MissingOperationKey);
        }
        if request.max_bytes == 0 {
            return Err(GitCommitError::InvalidLimit);
        }
        if request.paths.is_empty() {
            return Err(GitCommitError::InvalidPaths);
        }
        for path in &request.paths {
            if path.trim().is_empty()
                || std::path::Path::new(path).is_absolute()
                || path.split('/').any(|part| part == ".." || part.is_empty())
            {
                return Err(GitCommitError::InvalidPaths);
            }
        }
        if request.message.trim().is_empty() || request.message.len() > 10_000 {
            return Err(GitCommitError::InvalidMessage);
        }
        #[allow(clippy::collapsible_if)]
        if let Some(name) = &request.author_name {
            if name.trim().is_empty() || name.len() > 200 {
                return Err(GitCommitError::InvalidAuthor);
            }
        }
        #[allow(clippy::collapsible_if)]
        if let Some(email) = &request.author_email {
            if email.trim().is_empty() || email.len() > 320 || !email.contains('@') {
                return Err(GitCommitError::InvalidAuthor);
            }
        }

        // Preflight: status to show what will be committed
        let status = self.preflight_status(&request, cancel.clone())?;

        // Validate all requested paths are in the status output (staged or unstaged)
        self.validate_paths_against_status(&request.paths, &status)?;

        let mut args = vec!["commit".into()];

        // Add paths
        args.push("-m".into());
        args.push(request.message.clone());
        args.push("--".into());
        args.extend(request.paths.iter().cloned());

        let spec = ProcessSpec {
            project_id: request.project_id,
            program: self.git_program.clone(),
            args,
            cwd: self.repository.clone(),
            env: BTreeMap::from([
                (String::from("GIT_OPTIONAL_LOCKS"), String::from("0")),
                (String::from("GIT_TERMINAL_PROMPT"), String::from("0")),
            ]),
            allowed_programs: BTreeSet::from([self.git_program.clone()]),
            allowed_roots: vec![self.repository.clone()],
            permission: request.permission,
            timeout: Duration::from_secs(30),
            max_output_bytes: request.max_bytes,
            trace_id: request.trace_id,
        };

        let process = run_process(&spec, cancel)?;

        if process.timed_out || process.cancelled || process.exit_code != Some(0) {
            return Err(GitCommitError::GitFailed);
        }

        // Extract commit hash from stdout (format: [branch hash] message)
        let commit_hash = extract_commit_hash(&process.stdout).ok_or(GitCommitError::GitFailed)?;

        Ok(GitCommitResult {
            trace_id: request.trace_id,
            commit_hash,
            paths: request.paths,
            bytes: process.stdout.len(),
            operation_key: request.operation_key,
        })
    }

    fn preflight_status(
        &self,
        request: &GitCommitRequest,
        cancel: Arc<AtomicBool>,
    ) -> Result<String, GitCommitError> {
        let mut args = vec!["status".into(), "--porcelain=v1".into(), "--".into()];
        args.extend(request.paths.iter().cloned());

        let spec = ProcessSpec {
            project_id: request.project_id,
            program: self.git_program.clone(),
            args,
            cwd: self.repository.clone(),
            env: BTreeMap::from([(String::from("GIT_OPTIONAL_LOCKS"), String::from("0"))]),
            allowed_programs: BTreeSet::from([self.git_program.clone()]),
            allowed_roots: vec![self.repository.clone()],
            permission: request.permission.clone(),
            timeout: Duration::from_secs(5),
            max_output_bytes: request.max_bytes,
            trace_id: request.trace_id,
        };

        let process = run_process(&spec, cancel)?;

        if process.timed_out || process.cancelled || process.exit_code != Some(0) {
            return Err(GitCommitError::GitFailed);
        }

        Ok(process.stdout)
    }

    fn validate_paths_against_status(
        &self,
        paths: &[String],
        status_output: &str,
    ) -> Result<(), GitCommitError> {
        // Parse porcelain status: each line starts with XY (index/worktree status) then path
        let mut available_paths = BTreeSet::new();
        for line in status_output.lines() {
            if line.len() >= 3 {
                let path = &line[3..];
                available_paths.insert(path.to_string());
            }
        }

        for path in paths {
            if !available_paths.contains(path) {
                return Err(GitCommitError::InvalidPaths);
            }
        }
        Ok(())
    }
}

fn extract_commit_hash(output: &str) -> Option<String> {
    // git commit output format: "[branch hash] message"
    // or "[branch (root-commit) hash] message" for initial commit
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.contains(']') {
            let bracket_end = trimmed.find(']')?;
            let content = &trimmed[1..bracket_end];
            let parts: Vec<&str> = content.split_whitespace().collect();
            // Format: "branch hash" or "branch (root-commit) hash"
            if parts.len() >= 2 {
                // The last part is the hash
                return Some(parts.last().unwrap().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn setup_test_repo() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let git = which::which("git").unwrap();

        Command::new(&git)
            .args(["init"])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new(&git)
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new(&git)
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo)
            .output()
            .unwrap();

        (dir, repo, git)
    }

    fn create_test_tool(repo: &Path, git: &Path) -> GitCommitTool {
        let project_id = ProjectId::new();
        GitCommitTool::new(project_id, repo.to_path_buf(), git.to_path_buf()).unwrap()
    }

    #[test]
    fn test_commit_basic() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        // Create a file
        fs::write(repo.join("test.txt"), "hello").unwrap();

        // Stage it
        Command::new(&git)
            .args(["add", "test.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let request = GitCommitRequest {
            project_id,
            paths: vec!["test.txt".to_string()],
            message: "Add test file".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-1".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel).unwrap();

        assert!(!result.commit_hash.is_empty());
        assert_eq!(result.paths, vec!["test.txt"]);
        assert_eq!(result.operation_key, "op-1");
    }

    #[test]
    fn test_commit_with_author() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        fs::write(repo.join("author.txt"), "author test").unwrap();
        Command::new(&git)
            .args(["add", "author.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let request = GitCommitRequest {
            project_id,
            paths: vec!["author.txt".to_string()],
            message: "Author test".to_string(),
            author_name: Some("Custom Author".to_string()),
            author_email: Some("custom@example.com".to_string()),
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-2".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel).unwrap();

        assert!(!result.commit_hash.is_empty());
    }

    #[test]
    fn test_commit_multiple_paths() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        fs::write(repo.join("a.txt"), "a").unwrap();
        fs::write(repo.join("b.txt"), "b").unwrap();
        Command::new(&git)
            .args(["add", "a.txt", "b.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let request = GitCommitRequest {
            project_id,
            paths: vec!["a.txt".to_string(), "b.txt".to_string()],
            message: "Add two files".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-3".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel).unwrap();

        assert!(!result.commit_hash.is_empty());
        assert_eq!(result.paths.len(), 2);
    }

    #[test]
    fn test_commit_rejects_unauthorized_project() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let other_project = ProjectId::new();

        fs::write(repo.join("test.txt"), "hello").unwrap();
        Command::new(&git)
            .args(["add", "test.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let request = GitCommitRequest {
            project_id: other_project,
            paths: vec!["test.txt".to_string()],
            message: "test".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-4".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::ProjectUnauthorized));
    }

    #[test]
    fn test_commit_rejects_denied_permission() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        fs::write(repo.join("test.txt"), "hello").unwrap();
        Command::new(&git)
            .args(["add", "test.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();

        let request = GitCommitRequest {
            project_id,
            paths: vec!["test.txt".to_string()],
            message: "test".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Denied {
                reason: crate::PermissionError::PolicyDenied,
            },
            trace_id: TraceId::new(),
            operation_key: "op-5".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::PermissionDenied));
    }

    #[test]
    fn test_commit_rejects_empty_paths() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        let request = GitCommitRequest {
            project_id,
            paths: vec![],
            message: "test".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-6".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::InvalidPaths));
    }

    #[test]
    fn test_commit_rejects_invalid_path() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        let request = GitCommitRequest {
            project_id,
            paths: vec!["../escape.txt".to_string()],
            message: "test".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-7".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::InvalidPaths));
    }

    #[test]
    fn test_commit_rejects_empty_message() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        let request = GitCommitRequest {
            project_id,
            paths: vec!["test.txt".to_string()],
            message: "".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-8".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::InvalidMessage));
    }

    #[test]
    fn test_commit_rejects_missing_operation_key() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        let request = GitCommitRequest {
            project_id,
            paths: vec!["test.txt".to_string()],
            message: "test".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::MissingOperationKey));
    }

    #[test]
    fn test_commit_rejects_invalid_repo() {
        let dir = tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let git = which::which("git").unwrap();
        let project_id = ProjectId::new();

        let tool = GitCommitTool::new(project_id, repo, git);

        assert_eq!(tool, Err(GitCommitError::InvalidRepository));
    }

    #[test]
    fn test_commit_validates_paths_against_status() {
        let (_dir, repo, git) = setup_test_repo();
        let tool = create_test_tool(&repo, &git);
        let project_id = tool.project_id;

        // Create and stage file A
        fs::write(repo.join("a.txt"), "a").unwrap();
        Command::new(&git)
            .args(["add", "a.txt"])
            .current_dir(&repo)
            .output()
            .unwrap();

        // Try to commit file B which is not staged
        let request = GitCommitRequest {
            project_id,
            paths: vec!["b.txt".to_string()],
            message: "test".to_string(),
            author_name: None,
            author_email: None,
            permission: PermissionDecision::Allowed { reason: "test" },
            trace_id: TraceId::new(),
            operation_key: "op-9".to_string(),
            max_bytes: 1024,
        };

        let cancel = Arc::new(AtomicBool::new(false));
        let result = tool.commit(request, cancel);

        assert_eq!(result, Err(GitCommitError::InvalidPaths));
    }
}
