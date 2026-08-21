use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool};
use tempfile::tempdir;
use tool_core::{GitStatusError, GitStatusTool, PermissionDecision};

fn git() -> std::path::PathBuf {
    "/usr/bin/git".into()
}

fn repo() -> (tempfile::TempDir, ProjectId) {
    let dir = tempdir().unwrap();
    Command::new(git())
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    (dir, ProjectId::new())
}

#[test]
// @spec:AC-654
fn reports_branch_and_bounded_dirty_entries_without_mutation() {
    let (dir, project) = repo();
    fs::write(dir.path().join("note.txt"), "data").unwrap();
    let tool = GitStatusTool::new(project, dir.path().to_path_buf(), git(), 10).unwrap();
    let result = tool
        .status(
            project,
            PermissionDecision::Allowed { reason: "test" },
            TraceId::new(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
    assert!(!result.branch.is_empty());
    assert_eq!(result.entries[0].path, "note.txt");
    assert_eq!(
        fs::read_to_string(dir.path().join("note.txt")).unwrap(),
        "data"
    );
}

#[test]
// @spec:AC-655
fn rejects_project_permission_invalid_repo_and_limit() {
    let (dir, project) = repo();
    assert!(matches!(
        GitStatusTool::new(project, dir.path().to_path_buf(), git(), 0),
        Err(GitStatusError::InvalidLimit)
    ));
    let tool = GitStatusTool::new(project, dir.path().to_path_buf(), git(), 10).unwrap();
    assert!(matches!(
        tool.status(
            ProjectId::new(),
            PermissionDecision::Allowed { reason: "test" },
            TraceId::new(),
            Arc::new(AtomicBool::new(false))
        ),
        Err(GitStatusError::ProjectUnauthorized)
    ));
    assert!(matches!(
        tool.status(
            project,
            PermissionDecision::NeedsConfirmation {
                scope: "git".into()
            },
            TraceId::new(),
            Arc::new(AtomicBool::new(false))
        ),
        Err(GitStatusError::PermissionDenied)
    ));
}
