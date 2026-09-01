use agent_core::ids::ProjectId;
use agent_core::worktree::{WorktreeMode, WorktreeRequest};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::TempDir;
use tool_core::{GitWorktreeError, GitWorktreeTool, PermissionDecision};

fn setup_repository() -> (TempDir, PathBuf, PathBuf, ProjectId) {
    let dir = tempfile::tempdir().unwrap();
    let repository = dir.path().to_path_buf();
    let git = find_git();
    run_git(&git, &repository, ["init", "-q"]);
    run_git(&git, &repository, ["config", "user.name", "Contract Test"]);
    run_git(
        &git,
        &repository,
        ["config", "user.email", "contract@example.com"],
    );
    fs::write(repository.join("README.md"), "contract\n").unwrap();
    run_git(&git, &repository, ["add", "README.md"]);
    run_git(&git, &repository, ["commit", "-qm", "initial"]);
    (dir, repository, git, ProjectId::new())
}

fn find_git() -> PathBuf {
    let executable = if cfg!(windows) { "git.exe" } else { "git" };
    find_program(executable)
}

fn find_program(executable: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{executable} executable not found"))
}

fn run_git<const N: usize>(git: &Path, cwd: &Path, args: [&str; N]) {
    let output = Command::new(git)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success(), "git setup failed: {:?}", output);
}

fn request(project_id: &ProjectId, repository: &Path) -> WorktreeRequest {
    let worktree = repository.join("worktrees").join("task-1");
    WorktreeRequest::new(
        "wt-task-1",
        "workspace-1",
        project_id.to_string(),
        "task-1",
        "owner-1",
        repository.to_string_lossy(),
        worktree.to_string_lossy(),
        WorktreeMode::Detached,
    )
}

#[test]
// @spec:AC-1309
fn add_materializes_detached_worktree_inside_authorized_workspace() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let worktree = repository.join("worktrees").join("task-1");

    let result = tool
        .add(
            request(&project_id, &repository),
            PermissionDecision::Allowed { reason: "contract" },
            agent_protocol::ids::TraceId::new(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();

    assert_eq!(result.worktree_path, worktree.to_string_lossy());
    assert!(worktree.join(".git").is_file());
}

#[test]
// @spec:AC-1310
fn list_parses_porcelain_worktree_records_after_add() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let request = request(&project_id, &repository);
    tool.add(
        request,
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();

    let listed = tool
        .list(
            project_id,
            PermissionDecision::Allowed { reason: "contract" },
            agent_protocol::ids::TraceId::new(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();

    assert_eq!(listed.entries.len(), 2);
    assert!(
        listed
            .entries
            .iter()
            .any(|entry| entry.path.ends_with("worktrees/task-1"))
    );
    assert!(listed.entries.iter().all(|entry| !entry.head.is_empty()));
}

#[test]
// @spec:AC-1310
fn remove_deletes_only_the_validated_worktree_without_force() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let request = request(&project_id, &repository);
    let worktree = PathBuf::from(&request.worktree_path);
    tool.add(
        request.clone(),
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert!(worktree.exists());

    let result = tool
        .remove(
            request,
            PermissionDecision::Allowed { reason: "contract" },
            agent_protocol::ids::TraceId::new(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();

    assert_eq!(result.worktree_path, worktree.to_string_lossy());
    assert!(!worktree.exists());
}

#[test]
// @spec:AC-1309
fn add_rejects_foreign_project_before_creating_the_worktree() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let mut request = request(&project_id, &repository);
    request.project_id = ProjectId::new().to_string();
    let worktree = PathBuf::from(&request.worktree_path);

    let result = tool.add(
        request,
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(result, Err(GitWorktreeError::ProjectUnauthorized));
    assert!(!worktree.exists());
}

#[test]
// @spec:AC-1309
fn add_rejects_denied_permission_before_creating_the_worktree() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let request = request(&project_id, &repository);
    let worktree = PathBuf::from(&request.worktree_path);

    let result = tool.add(
        request,
        PermissionDecision::NeedsConfirmation {
            scope: "project".into(),
        },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(result, Err(GitWorktreeError::PermissionDenied));
    assert!(!worktree.exists());
}

#[test]
// @spec:AC-1309
fn add_materializes_a_branch_worktree_without_detached_mode() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let mut request = request(&project_id, &repository);
    request.mode = WorktreeMode::Branch {
        branch: "feature/task-1".into(),
    };

    tool.add(
        request,
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let listed = tool
        .list(
            project_id,
            PermissionDecision::Allowed { reason: "contract" },
            agent_protocol::ids::TraceId::new(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();

    assert!(
        listed
            .entries
            .iter()
            .any(|entry| { entry.branch.as_deref() == Some("feature/task-1") && !entry.detached })
    );
}

#[test]
// @spec:AC-1309
fn add_rejects_a_worktree_path_outside_the_configured_workspace() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository.clone(),
        git,
        64 * 1024,
    )
    .unwrap();
    let mut request = request(&project_id, &repository);
    request.worktree_path = repository
        .parent()
        .unwrap()
        .join("outside-worktree")
        .to_string_lossy()
        .into_owned();

    let result = tool.add(
        request,
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(result, Err(GitWorktreeError::InvalidRequest));
}

#[test]
// @spec:AC-1310
fn list_fails_closed_when_process_output_is_truncated() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool =
        GitWorktreeTool::new(project_id, repository.clone(), repository.clone(), git, 1).unwrap();

    let result = tool.list(
        project_id,
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(result, Err(GitWorktreeError::OutputTruncated));
}

#[cfg(unix)]
#[test]
// @spec:AC-1310
fn list_rejects_malformed_porcelain_instead_of_returning_partial_state() {
    let (_dir, repository, _git, project_id) = setup_repository();
    let printf = find_program("printf");
    let tool = GitWorktreeTool::new(
        project_id,
        repository.clone(),
        repository,
        printf,
        64 * 1024,
    )
    .unwrap();

    let result = tool.list(
        project_id,
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(result, Err(GitWorktreeError::MalformedOutput));
}

#[test]
// @spec:AC-1310
fn list_rejects_a_foreign_project_before_running_git() {
    let (_dir, repository, git, project_id) = setup_repository();
    let tool =
        GitWorktreeTool::new(project_id, repository.clone(), repository, git, 64 * 1024).unwrap();

    let result = tool.list(
        ProjectId::new(),
        PermissionDecision::Allowed { reason: "contract" },
        agent_protocol::ids::TraceId::new(),
        Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(result, Err(GitWorktreeError::ProjectUnauthorized));
}
