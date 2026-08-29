use agent_core::error::DomainError;
use agent_core::worktree::{
    WorktreeMode, WorktreeRecoveryAction, WorktreeRegistry, WorktreeRequest, MAX_WORKTREE_PATH_LEN,
};

fn request() -> WorktreeRequest {
    WorktreeRequest::new(
        "wt-task-1",
        "workspace-1",
        "project-1",
        "task-1",
        "owner-1",
        "/srv/hank/workspaces/project-1",
        "/srv/hank/workspaces/project-1/tasks/wt-task-1",
        WorktreeMode::Branch {
            branch: "agent/task-1".into(),
        },
    )
}

#[test]
// @spec:AC-1306
fn registers_and_lists_a_bounded_worktree_request() {
    let mut registry = WorktreeRegistry::new(8);
    let request = request();

    let record = registry.register(request.clone()).unwrap();

    assert_eq!(record.request(), &request);
    assert_eq!(registry.get("wt-task-1"), Some(&record));
    assert_eq!(registry.list(), vec![record]);
}

#[test]
// @spec:AC-1308
fn rejects_relative_and_outside_worktree_paths_without_mutation() {
    let mut registry = WorktreeRegistry::new(8);
    let mut invalid = request();
    invalid.worktree_path = "relative/path".into();
    assert!(matches!(
        registry.register(invalid),
        Err(DomainError::Validation(_))
    ));

    let mut outside = request();
    outside.worktree_path = "/srv/hank/other/task-1".into();
    assert!(matches!(
        registry.register(outside),
        Err(DomainError::Validation(_))
    ));
    assert!(registry.is_empty());
}

#[test]
// @spec:AC-1307
fn identical_registration_is_idempotent_but_collisions_do_not_replace_owner() {
    let mut registry = WorktreeRegistry::new(8);
    let request = request();
    let first = registry.register(request.clone()).unwrap();
    let same = registry.register(request.clone()).unwrap();
    assert_eq!(same, first);

    let mut collision = request;
    collision.owner_id = "owner-2".into();
    assert!(matches!(
        registry.register(collision),
        Err(DomainError::Duplicate(_))
    ));
    assert_eq!(registry.get("wt-task-1").unwrap().owner_id(), "owner-1");
}

#[test]
// @spec:AC-1307
fn registry_capacity_rejects_new_records_without_mutating_existing_state() {
    let mut registry = WorktreeRegistry::new(1);
    let first = registry.register(request()).unwrap();

    let mut second = request();
    second.worktree_id = "wt-task-2".into();
    second.task_id = "task-2".into();
    second.worktree_path = "/srv/hank/workspaces/project-1/tasks/wt-task-2".into();
    second.mode = WorktreeMode::Detached;

    assert!(matches!(
        registry.register(second),
        Err(DomainError::Validation(_))
    ));
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.get(first.worktree_id()), Some(&first));
}

#[test]
// @spec:AC-1311
fn recovery_plan_removes_only_owned_registered_paths_and_preserves_unknown_paths() {
    let mut registry = WorktreeRegistry::new(8);
    let owned = registry.register(request()).unwrap();
    let mut foreign = request();
    foreign.worktree_id = "wt-task-2".into();
    foreign.task_id = "task-2".into();
    foreign.owner_id = "owner-2".into();
    foreign.worktree_path = "/srv/hank/workspaces/project-1/tasks/wt-task-2".into();
    foreign.mode = WorktreeMode::Branch {
        branch: "agent/task-2".into(),
    };
    registry.register(foreign.clone()).unwrap();

    let plan = registry
        .recovery_plan(
            "project-1",
            "owner-1",
            &[
                foreign.worktree_path.clone(),
                "/srv/hank/workspaces/project-1/tasks/unknown".into(),
                owned.worktree_path().into(),
            ],
        )
        .unwrap();

    assert_eq!(
        plan,
        vec![
            WorktreeRecoveryAction::PreserveUnknown {
                worktree_path: "/srv/hank/workspaces/project-1/tasks/unknown".into(),
            },
            WorktreeRecoveryAction::PreserveUnknown {
                worktree_path: foreign.worktree_path,
            },
            WorktreeRecoveryAction::RemoveRegistered {
                worktree_id: owned.worktree_id().into(),
                worktree_path: owned.worktree_path().into(),
            },
        ]
    );
    assert_eq!(registry.len(), 2);
}

#[test]
// @spec:AC-1312
fn recovery_plan_rejects_invalid_observed_paths_without_partial_actions() {
    let mut registry = WorktreeRegistry::new(8);
    registry.register(request()).unwrap();

    for invalid in ["relative/path", "/srv/hank/workspaces/project-1/../escape"] {
        assert!(matches!(
            registry.recovery_plan("project-1", "owner-1", &[invalid.into()]),
            Err(DomainError::Validation(_))
        ));
    }
    let control = "/srv/hank/workspaces/project-1/tasks/bad\u{7f}".into();
    assert!(matches!(
        registry.recovery_plan("project-1", "owner-1", &[control]),
        Err(DomainError::Validation(_))
    ));
    let oversized = format!("/{}", "a".repeat(MAX_WORKTREE_PATH_LEN));
    assert!(matches!(
        registry.recovery_plan("project-1", "owner-1", &[oversized]),
        Err(DomainError::Validation(_))
    ));
    assert_eq!(registry.len(), 1);
}
