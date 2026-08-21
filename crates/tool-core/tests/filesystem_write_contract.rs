use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;
use tool_core::{
    FilesystemWriteError, FilesystemWriteTool, PermissionDecision, ToolCancellation,
    ToolExecutionWindow,
};

fn allowed() -> PermissionDecision {
    PermissionDecision::Allowed { reason: "test" }
}

#[test]
// @spec:AC-640 @spec:AC-641
fn writes_new_file_and_rolls_back() {
    let root = tempdir().unwrap();
    let tool =
        FilesystemWriteTool::new(ProjectId::new(), vec![root.path().to_path_buf()], 32).unwrap();
    let result = tool
        .write(
            tool.project_id(),
            "new.txt",
            b"hello",
            allowed(),
            TraceId::new(),
            "op-1",
        )
        .unwrap();
    assert_eq!(result.bytes_written, 5);
    assert_eq!(
        fs::read_to_string(root.path().join("new.txt")).unwrap(),
        "hello"
    );
    tool.rollback("op-1").unwrap();
    assert!(!root.path().join("new.txt").exists());
}

#[test]
// @spec:AC-640 @spec:AC-641
fn existing_file_snapshot_is_restored() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("data.txt"), "old").unwrap();
    let tool =
        FilesystemWriteTool::new(ProjectId::new(), vec![root.path().to_path_buf()], 32).unwrap();
    tool.write(
        tool.project_id(),
        "data.txt",
        b"new",
        allowed(),
        TraceId::new(),
        "op-2",
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("data.txt")).unwrap(),
        "new"
    );
    tool.rollback("op-2").unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("data.txt")).unwrap(),
        "old"
    );
}

#[test]
// @spec:AC-642
fn rejects_permission_path_project_payload_and_duplicate_without_mutation() {
    let root = tempdir().unwrap();
    let project = ProjectId::new();
    let tool = FilesystemWriteTool::new(project, vec![root.path().to_path_buf()], 3).unwrap();
    assert!(matches!(
        tool.write(project, "x.txt", b"1234", allowed(), TraceId::new(), "op"),
        Err(FilesystemWriteError::PayloadTooLarge)
    ));
    assert!(matches!(
        tool.write(project, "../x.txt", b"x", allowed(), TraceId::new(), "op"),
        Err(FilesystemWriteError::InvalidPath)
    ));
    assert!(matches!(
        tool.write(
            ProjectId::new(),
            "x.txt",
            b"x",
            allowed(),
            TraceId::new(),
            "op"
        ),
        Err(FilesystemWriteError::ProjectUnauthorized)
    ));
    assert!(matches!(
        tool.write(
            project,
            "x.txt",
            b"x",
            PermissionDecision::NeedsConfirmation { scope: "x".into() },
            TraceId::new(),
            "op"
        ),
        Err(FilesystemWriteError::PermissionDenied)
    ));
    tool.write(project, "x.txt", b"x", allowed(), TraceId::new(), "same")
        .unwrap();
    tool.write(project, "x.txt", b"y", allowed(), TraceId::new(), "same")
        .unwrap();
    assert_eq!(fs::read_to_string(root.path().join("x.txt")).unwrap(), "x");
}

#[test]
// @spec:AC-666 @spec:AC-667
fn write_with_window_honors_cancel_before_mutation() {
    let root = tempdir().unwrap();
    let project = ProjectId::new();
    let tool = FilesystemWriteTool::new(project, vec![root.path().to_path_buf()], 32).unwrap();
    let window = ToolExecutionWindow::with_cancellation(
        std::time::Duration::from_secs(1),
        ToolCancellation::from_flag(Arc::new(AtomicBool::new(true))),
    )
    .unwrap();

    assert_eq!(
        tool.write_with_window(
            project,
            "cancelled.txt",
            b"must not persist",
            allowed(),
            TraceId::new(),
            "cancelled-op",
            &window,
        ),
        Err(FilesystemWriteError::Cancelled)
    );
    assert!(!root.path().join("cancelled.txt").exists());
}
