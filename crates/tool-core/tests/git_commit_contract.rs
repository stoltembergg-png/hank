use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::tempdir;
use tool_core::{GitCommitError, GitCommitRequest, GitCommitTool, PermissionDecision};

fn git() -> std::path::PathBuf {
    find_git().unwrap_or_else(|| panic!("git executable not found in PATH"))
}

fn find_git() -> Option<std::path::PathBuf> {
    let executable = if cfg!(windows) { "git.exe" } else { "git" };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(path.as_os_str())
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
}

fn repo() -> (tempfile::TempDir, ProjectId) {
    let dir = tempdir().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        assert!(
            Command::new(git())
                .args(args)
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(dir.path().join("note.txt"), "old\n").unwrap();
    assert!(
        Command::new(git())
            .args(["add", "note.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(git())
            .args(["commit", "-qm", "initial"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    (dir, ProjectId::new())
}

fn request(project: ProjectId, paths: Vec<String>, max_bytes: usize) -> GitCommitRequest {
    GitCommitRequest {
        project_id: project,
        paths,
        message: "test commit".to_string(),
        author_name: None,
        author_email: None,
        permission: PermissionDecision::Allowed { reason: "test" },
        trace_id: TraceId::new(),
        operation_key: "op-1".to_string(),
        max_bytes,
    }
}

#[test]
// @spec:AC-659
fn commits_staged_files_and_returns_hash() {
    let (dir, project) = repo();
    fs::write(dir.path().join("note.txt"), "new\n").unwrap();
    assert!(
        Command::new(git())
            .args(["add", "note.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    let tool = GitCommitTool::new(project, dir.path().to_path_buf(), git()).unwrap();
    let result = tool
        .commit(
            request(project, vec!["note.txt".to_string()], 4096),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    assert!(!result.commit_hash.is_empty());
    assert_eq!(result.paths, vec!["note.txt"]);
    assert_eq!(result.operation_key, "op-1");
}

#[test]
// @spec:AC-660
fn rejects_wrong_project_permission_paths_and_message() {
    let (dir, project) = repo();
    fs::write(dir.path().join("note.txt"), "new\n").unwrap();
    assert!(
        Command::new(git())
            .args(["add", "note.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    let tool = GitCommitTool::new(project, dir.path().to_path_buf(), git()).unwrap();

    assert!(matches!(
        tool.commit(
            request(ProjectId::new(), vec!["note.txt".to_string()], 10),
            Arc::new(AtomicBool::new(false))
        ),
        Err(GitCommitError::ProjectUnauthorized)
    ));

    let mut denied = request(project, vec!["note.txt".to_string()], 10);
    denied.permission = PermissionDecision::NeedsConfirmation {
        scope: "git".into(),
    };
    assert!(matches!(
        tool.commit(denied, Arc::new(AtomicBool::new(false))),
        Err(GitCommitError::PermissionDenied)
    ));

    let invalid = request(project, vec!["../outside".to_string()], 10);
    assert!(matches!(
        tool.commit(invalid, Arc::new(AtomicBool::new(false))),
        Err(GitCommitError::InvalidPaths)
    ));

    let empty = request(project, vec!["note.txt".to_string()], 10);
    let mut empty = empty;
    empty.message = "".to_string();
    assert!(matches!(
        tool.commit(empty, Arc::new(AtomicBool::new(false))),
        Err(GitCommitError::InvalidMessage)
    ));

    let missing_key = request(project, vec!["note.txt".to_string()], 10);
    let mut missing_key = missing_key;
    missing_key.operation_key = "".to_string();
    assert!(matches!(
        tool.commit(missing_key, Arc::new(AtomicBool::new(false))),
        Err(GitCommitError::MissingOperationKey)
    ));
}

#[test]
// @spec:AC-661
fn validates_paths_against_status() {
    let (dir, project) = repo();
    fs::write(dir.path().join("staged.txt"), "staged\n").unwrap();
    assert!(
        Command::new(git())
            .args(["add", "staged.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    // unstaged file
    fs::write(dir.path().join("unstaged.txt"), "unstaged\n").unwrap();

    let tool = GitCommitTool::new(project, dir.path().to_path_buf(), git()).unwrap();

    // Try to commit unstaged file - should fail
    assert!(matches!(
        tool.commit(
            request(project, vec!["unstaged.txt".to_string()], 10),
            Arc::new(AtomicBool::new(false))
        ),
        Err(GitCommitError::InvalidPaths)
    ));

    // Commit staged file - should succeed
    let result = tool
        .commit(
            request(project, vec!["staged.txt".to_string()], 4096),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    assert!(!result.commit_hash.is_empty());
}
