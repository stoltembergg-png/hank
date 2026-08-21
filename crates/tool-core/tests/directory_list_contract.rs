use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::fs;
use tempfile::tempdir;
use tool_core::{DirectoryFilter, DirectoryListError, DirectoryListTool, PermissionDecision};

fn allowed() -> PermissionDecision {
    PermissionDecision::Allowed { reason: "test" }
}

#[test]
// @spec:AC-643
fn lists_sorted_bounded_entries_and_hides_dotfiles_by_default() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("b.txt"), "bb").unwrap();
    fs::write(root.path().join("a.txt"), "a").unwrap();
    fs::write(root.path().join(".secret"), "secret").unwrap();
    let tool =
        DirectoryListTool::new(ProjectId::new(), vec![root.path().to_path_buf()], 1).unwrap();
    let result = tool
        .list(
            tool.project_id(),
            "",
            DirectoryFilter::default(),
            allowed(),
            TraceId::new(),
        )
        .unwrap();
    assert_eq!(result.entries[0].name, "a.txt");
    assert!(result.truncated);
    assert!(
        result
            .entries
            .iter()
            .all(|entry| !entry.name.starts_with('.'))
    );
}

#[test]
// @spec:AC-644
fn filters_and_rejects_invalid_project_path_permission_and_filter() {
    let root = tempdir().unwrap();
    fs::write(root.path().join("a.rs"), "a").unwrap();
    fs::write(root.path().join("a.txt"), "a").unwrap();
    let project = ProjectId::new();
    let tool = DirectoryListTool::new(project, vec![root.path().to_path_buf()], 10).unwrap();
    let result = tool
        .list(
            project,
            "",
            DirectoryFilter {
                prefix: Some("a".into()),
                suffix: Some(".rs".into()),
                include_hidden: false,
            },
            allowed(),
            TraceId::new(),
        )
        .unwrap();
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a.rs"]
    );
    assert!(matches!(
        tool.list(
            project,
            "../",
            DirectoryFilter::default(),
            allowed(),
            TraceId::new()
        ),
        Err(DirectoryListError::InvalidPath)
    ));
    assert!(matches!(
        tool.list(
            ProjectId::new(),
            "",
            DirectoryFilter::default(),
            allowed(),
            TraceId::new()
        ),
        Err(DirectoryListError::ProjectUnauthorized)
    ));
    assert!(matches!(
        tool.list(
            project,
            "",
            DirectoryFilter::default(),
            PermissionDecision::NeedsConfirmation { scope: "x".into() },
            TraceId::new()
        ),
        Err(DirectoryListError::PermissionDenied)
    ));
    assert!(matches!(
        tool.list(
            project,
            "",
            DirectoryFilter {
                prefix: Some("".into()),
                ..Default::default()
            },
            allowed(),
            TraceId::new()
        ),
        Err(DirectoryListError::InvalidFilter)
    ));
}

#[cfg(unix)]
#[test]
// @spec:AC-645
fn rejects_symlink_escape_and_returns_metadata_only() {
    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        root.path().join("escape.txt"),
    )
    .unwrap();
    let tool =
        DirectoryListTool::new(ProjectId::new(), vec![root.path().to_path_buf()], 10).unwrap();
    assert!(matches!(
        tool.list(
            tool.project_id(),
            "",
            DirectoryFilter {
                include_hidden: true,
                ..Default::default()
            },
            allowed(),
            TraceId::new()
        ),
        Err(DirectoryListError::OutsideRoot)
    ));
}
