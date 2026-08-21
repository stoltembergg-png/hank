use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::tempdir;
use tool_core::{GitDiffError, GitDiffMode, GitDiffRequest, GitDiffTool, PermissionDecision};

mod support;
use support::git_program;

fn repo() -> (tempfile::TempDir, ProjectId) {
    let dir = tempdir().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test"],
    ] {
        assert!(
            Command::new(git_program())
                .args(args)
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(dir.path().join("note.txt"), "old\n").unwrap();
    assert!(
        Command::new(git_program())
            .args(["add", "note.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(git_program())
            .args(["commit", "-qm", "initial"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    (dir, ProjectId::new())
}

fn request(project: ProjectId, mode: GitDiffMode, max_bytes: usize) -> GitDiffRequest {
    GitDiffRequest {
        project_id: project,
        mode,
        path: None,
        permission: PermissionDecision::Allowed { reason: "test" },
        trace_id: TraceId::new(),
        max_bytes,
    }
}

#[test]
// @spec:AC-656
fn returns_unstaged_diff_and_does_not_mutate_repo() {
    let (dir, project) = repo();
    fs::write(dir.path().join("note.txt"), "new\n").unwrap();
    let tool = GitDiffTool::new(project, dir.path().to_path_buf(), git_program()).unwrap();
    let result = tool
        .diff(
            request(project, GitDiffMode::Unstaged, 4096),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    assert!(result.diff.contains("-old") && result.diff.contains("+new"));
    assert_eq!(
        fs::read_to_string(dir.path().join("note.txt")).unwrap(),
        "new\n"
    );
}

#[test]
// @spec:AC-657
fn supports_staged_and_path_modes_with_explicit_truncation_redaction() {
    let (dir, project) = repo();
    fs::write(dir.path().join("note.txt"), "api_token=secret-value\n").unwrap();
    let tool = GitDiffTool::new(project, dir.path().to_path_buf(), git_program()).unwrap();
    let mut path_request = request(project, GitDiffMode::Path, 16);
    path_request.path = Some("note.txt".into());
    let result = tool
        .diff(path_request, Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert!(result.truncated || result.diff.contains("[redacted]"));
    assert!(
        Command::new(git_program())
            .args(["add", "note.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success()
    );
    let staged = tool
        .diff(
            request(project, GitDiffMode::Staged, 4096),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    assert!(staged.diff.contains("[redacted]"));
}

#[test]
// @spec:AC-658
fn rejects_wrong_project_permission_path_and_limit() {
    let (dir, project) = repo();
    let tool = GitDiffTool::new(project, dir.path().to_path_buf(), git_program()).unwrap();
    assert!(matches!(
        tool.diff(
            request(ProjectId::new(), GitDiffMode::Unstaged, 10),
            Arc::new(AtomicBool::new(false))
        ),
        Err(GitDiffError::ProjectUnauthorized)
    ));
    let mut denied = request(project, GitDiffMode::Unstaged, 10);
    denied.permission = PermissionDecision::NeedsConfirmation {
        scope: "git".into(),
    };
    assert!(matches!(
        tool.diff(denied, Arc::new(AtomicBool::new(false))),
        Err(GitDiffError::PermissionDenied)
    ));
    let mut invalid = request(project, GitDiffMode::Path, 10);
    invalid.path = Some("../outside".into());
    assert!(matches!(
        tool.diff(invalid, Arc::new(AtomicBool::new(false))),
        Err(GitDiffError::InvalidPath)
    ));
}
