use agent_core::workspace::{WorkspaceManager, WorkspaceRegistration};
use agent_core::DomainError;

fn registration(id: &str, project: &str, repository: &str, root: &str) -> WorkspaceRegistration {
    WorkspaceRegistration::new(id, project, repository, root)
}

#[test]
// @spec:AC-1301
fn registration_preserves_project_repository_and_canonical_root() {
    let mut manager = WorkspaceManager::new();
    manager
        .register(registration(
            "ws-1",
            "proj-a",
            "repo-a",
            "/srv/hank/project-a",
        ))
        .unwrap();

    let workspace = manager.get("ws-1").unwrap();
    assert_eq!(workspace.project_id(), "proj-a");
    assert_eq!(workspace.repository_id(), "repo-a");
    assert_eq!(workspace.canonical_root(), "/srv/hank/project-a");
}

#[test]
// @spec:AC-1302
fn invalid_or_traversing_roots_are_rejected_before_registration() {
    let invalid_roots = vec![
        String::new(),
        "relative/root".to_owned(),
        "/srv/hank/../outside".to_owned(),
        "/srv/hank/./project".to_owned(),
        "/srv/hank/\0project".to_owned(),
        format!("/srv/hank/{}", "r".repeat(4097)),
    ];

    for root in invalid_roots {
        let mut manager = WorkspaceManager::new();
        let result = manager.register(registration("ws-1", "proj-a", "repo-a", &root));
        assert!(
            matches!(result, Err(DomainError::Validation(_))),
            "root={root:?}"
        );
        assert!(manager.get("ws-1").is_none());
        assert!(manager.is_empty());
    }
}

#[test]
// @spec:AC-1303
fn concurrent_lease_conflict_is_deterministic() {
    let mut manager = WorkspaceManager::new();
    manager
        .register(registration(
            "ws-1",
            "proj-a",
            "repo-a",
            "/srv/hank/project-a",
        ))
        .unwrap();

    let first = manager.acquire_lease("ws-1", "agent-a").unwrap();
    assert_eq!(first.epoch(), 1);
    assert!(matches!(
        manager.acquire_lease("ws-1", "agent-b"),
        Err(DomainError::ConcurrencyConflict { .. })
    ));
    assert_eq!(manager.active_holder("ws-1"), Some("agent-a"));
}

#[test]
// @spec:AC-1304
fn release_requires_exact_token_and_reacquisition_increments_epoch() {
    let mut manager = WorkspaceManager::new();
    manager
        .register(registration(
            "ws-1",
            "proj-a",
            "repo-a",
            "/srv/hank/project-a",
        ))
        .unwrap();
    manager
        .register(registration(
            "ws-2",
            "proj-a",
            "repo-b",
            "/srv/hank/project-b",
        ))
        .unwrap();

    let first = manager.acquire_lease("ws-1", "agent-a").unwrap();
    let wrong_workspace_token = manager.acquire_lease("ws-2", "agent-b").unwrap();
    assert!(manager.release_lease(&wrong_workspace_token).is_ok());
    assert!(manager.release_lease(&wrong_workspace_token).is_err());
    assert!(manager.release_lease(&first).is_ok());

    let second = manager.acquire_lease("ws-1", "agent-c").unwrap();
    assert_eq!(second.epoch(), 2);
    assert!(manager.release_lease(&first).is_err());
}

#[test]
// @spec:AC-1305
fn duplicate_workspace_and_cross_project_root_fail_without_mutation() {
    let mut manager = WorkspaceManager::new();
    manager
        .register(registration(
            "ws-1",
            "proj-a",
            "repo-a",
            "/srv/hank/project-a",
        ))
        .unwrap();

    assert!(matches!(
        manager.register(registration(
            "ws-1",
            "proj-a",
            "repo-a",
            "/srv/hank/project-a"
        )),
        Err(DomainError::Duplicate(_))
    ));
    assert!(matches!(
        manager.register(registration(
            "ws-2",
            "proj-b",
            "repo-b",
            "/srv/hank/project-a"
        )),
        Err(DomainError::Validation(_))
    ));
    assert_eq!(manager.len(), 1);
    assert_eq!(manager.get("ws-1").unwrap().project_id(), "proj-a");
}
