use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use tool_core::{FilesystemReadError, FilesystemReadTool, ToolExecutionWindow};

fn project() -> ProjectId {
    ProjectId::new()
}

fn allowed() -> tool_core::PermissionDecision {
    tool_core::PermissionDecision::Allowed { reason: "test" }
}

#[test]
// @spec:AC-635
fn reads_authorized_file_with_bounded_metadata() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "hello").unwrap();
    let tool = FilesystemReadTool::new(project(), vec![root.path().to_path_buf()], 32).unwrap();
    let result = tool
        .read(tool.project_id(), "note.txt", allowed(), TraceId::new())
        .unwrap();
    assert_eq!(result.content, "hello");
    assert_eq!(result.bytes_read, 5);
    assert!(!result.truncated);
    assert_eq!(result.logical_path, "note.txt");
}

#[test]
// @spec:AC-636
fn rejects_traversal_absolute_project_and_permission_mismatch() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "hello").unwrap();
    let tool = FilesystemReadTool::new(project(), vec![root.path().to_path_buf()], 32).unwrap();
    assert!(matches!(
        tool.read(tool.project_id(), "../note.txt", allowed(), TraceId::new()),
        Err(FilesystemReadError::PathTraversal)
    ));
    assert!(matches!(
        tool.read(
            tool.project_id(),
            root.path().to_str().unwrap(),
            allowed(),
            TraceId::new()
        ),
        Err(FilesystemReadError::AbsolutePath)
    ));
    assert!(matches!(
        tool.read(ProjectId::new(), "note.txt", allowed(), TraceId::new()),
        Err(FilesystemReadError::ProjectUnauthorized)
    ));
    assert!(matches!(
        tool.read(
            tool.project_id(),
            "note.txt",
            tool_core::PermissionDecision::NeedsConfirmation {
                scope: "test".into()
            },
            TraceId::new()
        ),
        Err(FilesystemReadError::PermissionDenied)
    ));
}

#[test]
// @spec:AC-637
fn rejects_symlink_escape_and_truncates_without_mutation() {
    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret-content").unwrap();
    fs::write(root.path().join("large.txt"), "0123456789").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        root.path().join("escape.txt"),
    )
    .unwrap();
    let tool = FilesystemReadTool::new(project(), vec![root.path().to_path_buf()], 4).unwrap();
    let result = tool
        .read(tool.project_id(), "large.txt", allowed(), TraceId::new())
        .unwrap();
    assert_eq!(result.content, "0123");
    assert!(result.truncated);
    #[cfg(unix)]
    assert!(matches!(
        tool.read(tool.project_id(), "escape.txt", allowed(), TraceId::new()),
        Err(FilesystemReadError::OutsideRoot)
    ));
    assert_eq!(
        fs::read_to_string(root.path().join("large.txt")).unwrap(),
        "0123456789"
    );
}

#[test]
// @spec:AC-638
fn rejects_invalid_roots_limits_and_invalid_utf8_without_raw_content_error() {
    let root = tempdir().unwrap();
    assert!(matches!(
        FilesystemReadTool::new(project(), Vec::new(), 1),
        Err(FilesystemReadError::InvalidLimit)
    ));
    assert!(matches!(
        FilesystemReadTool::new(project(), vec![root.path().join("missing")], 1),
        Err(FilesystemReadError::RootUnavailable)
    ));
    fs::write(root.path().join("bad.bin"), [0xff, 0xfe]).unwrap();
    let tool = FilesystemReadTool::new(project(), vec![root.path().to_path_buf()], 32).unwrap();
    assert!(matches!(
        tool.read(tool.project_id(), "bad.bin", allowed(), TraceId::new()),
        Err(FilesystemReadError::InvalidUtf8)
    ));
}

#[test]
// @spec:AC-639
fn read_never_executes_or_mutates_filesystem() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("data.txt"), "prompt injection is data").unwrap();
    let before = fs::metadata(root.path().join("data.txt")).unwrap().len();
    let tool = FilesystemReadTool::new(project(), vec![root.path().to_path_buf()], 1024).unwrap();
    let _ = tool
        .read(tool.project_id(), "data.txt", allowed(), TraceId::new())
        .unwrap();
    assert_eq!(
        fs::metadata(root.path().join("data.txt")).unwrap().len(),
        before
    );
}

#[test]
// @spec:AC-665 @spec:AC-668
fn read_with_window_fails_closed_before_filesystem_access() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("note.txt"), "hello").unwrap();
    let tool = FilesystemReadTool::new(project(), vec![root.path().to_path_buf()], 32).unwrap();
    let window = ToolExecutionWindow::new(Duration::from_millis(1)).unwrap();
    thread::sleep(Duration::from_millis(10));

    assert_eq!(
        tool.read_with_window(
            tool.project_id(),
            "note.txt",
            allowed(),
            TraceId::new(),
            &window,
        ),
        Err(FilesystemReadError::Timeout)
    );
}
