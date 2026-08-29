use agent_core::task_mapping::{
    MappingObservation, MappingRebindAuthorization, MappingState, TaskWorkspaceMapping,
    TaskWorkspaceMappingRegistry, MAX_TASK_MAPPINGS, MAX_TASK_MAPPING_BRANCH_LEN,
    MAX_TASK_MAPPING_REPOSITORY_ID_LEN,
};
use agent_core::{ProjectId, RunId, TaskId, TraceId};

fn mapping(
    project_id: ProjectId,
    task_id: TaskId,
    worktree_id: &str,
    branch: &str,
) -> TaskWorkspaceMapping {
    TaskWorkspaceMapping::new(
        project_id,
        task_id,
        "repo-1",
        worktree_id,
        branch,
        RunId::new(),
        Some("pr-207".into()),
        TraceId::new(),
        "policy-r1",
    )
    .unwrap()
}

#[test]
fn unique_task_worktree_and_branch_mappings_are_project_scoped() {
    // @spec:AC-1317
    let project = ProjectId::new();
    let task = TaskId::new();
    let first = mapping(project, task, "wt-1", "agent/task-1");
    let mut registry = TaskWorkspaceMappingRegistry::new(8).unwrap();

    registry.register(first.clone()).unwrap();
    assert_eq!(registry.get(project, task), Some(&first));
    assert_eq!(registry.list(project).unwrap().len(), 1);

    let duplicate_task = mapping(project, task, "wt-2", "agent/task-2");
    assert!(registry.register(duplicate_task).is_err());

    let duplicate_worktree = mapping(project, TaskId::new(), "wt-1", "agent/task-3");
    assert!(registry.register(duplicate_worktree).is_err());

    let duplicate_branch = mapping(project, TaskId::new(), "wt-3", "agent/task-1");
    assert!(registry.register(duplicate_branch).is_err());

    let foreign = mapping(ProjectId::new(), task, "wt-1", "agent/task-1");
    registry.register(foreign).unwrap();
    assert_eq!(registry.list(project).unwrap().len(), 1);
}

#[test]
fn lifecycle_requires_revision_and_explicit_rebind_authorization() {
    // @spec:AC-1319
    let project = ProjectId::new();
    let task = TaskId::new();
    let first = mapping(project, task, "wt-1", "agent/task-1");
    let mut registry = TaskWorkspaceMappingRegistry::new(8).unwrap();
    registry.register(first).unwrap();

    let detached = registry.detach(project, task, 1, 100).unwrap();
    assert_eq!(detached.state(), MappingState::Detached);
    assert_eq!(detached.revision(), 2);
    assert!(registry.detach(project, task, 1, 101).is_err());

    let resumed = registry.resume(project, task, 2, 200).unwrap();
    assert_eq!(resumed.state(), MappingState::Active);
    assert_eq!(resumed.last_resumed_at_ms(), Some(200));

    assert!(registry
        .rebind(
            project,
            task,
            3,
            "repo-2",
            "wt-2",
            "agent/task-1-rebound",
            None,
            MappingRebindAuthorization::new("policy-r0", "explicit approval"),
            300,
        )
        .is_err());

    let rebound = registry
        .rebind(
            project,
            task,
            3,
            "repo-2",
            "wt-2",
            "agent/task-1-rebound",
            Some("pr-207".into()),
            MappingRebindAuthorization::new("policy-r1", "explicit approval"),
            301,
        )
        .unwrap();
    assert_eq!(rebound.revision(), 4);
    assert_eq!(rebound.worktree_id(), "wt-2");
    assert_eq!(rebound.branch(), "agent/task-1-rebound");
}

#[test]
fn reconcile_preserves_observation_and_never_executes_external_effects() {
    // @spec:AC-1320
    let project = ProjectId::new();
    let task = TaskId::new();
    let first = mapping(project, task, "wt-1", "agent/task-1");
    let mut registry = TaskWorkspaceMappingRegistry::new(8).unwrap();
    registry.register(first).unwrap();

    let mismatch =
        MappingObservation::new("repo-1", "wt-foreign", "agent/task-1", 400, TraceId::new())
            .unwrap();
    let reconciled = registry
        .reconcile(project, task, 1, mismatch.clone())
        .unwrap();
    assert_eq!(reconciled.state(), MappingState::ReconcileRequired);
    assert_eq!(reconciled.observation(), Some(&mismatch));
    assert_eq!(
        reconciled.reconcile_reason(),
        Some("observed identity mismatch")
    );

    let other = mapping(ProjectId::new(), TaskId::new(), "wt-2", "agent/task-2");
    registry.register(other.clone()).unwrap();
    let matching = MappingObservation::new(
        other.repository_id(),
        other.worktree_id(),
        other.branch(),
        401,
        TraceId::new(),
    )
    .unwrap();
    let consistent = registry
        .reconcile(other.project_id(), other.task_id(), 1, matching.clone())
        .unwrap();
    assert_eq!(consistent.state(), MappingState::Active);
    assert_eq!(consistent.observation(), Some(&matching));
}

#[test]
fn bounded_metadata_and_released_mappings_fail_closed() {
    // @spec:AC-1318
    let project = ProjectId::new();
    let task = TaskId::new();
    assert!(TaskWorkspaceMapping::new(
        project,
        task,
        "repo-1",
        "wt\n1",
        "agent/task-1",
        RunId::new(),
        None,
        TraceId::new(),
        "policy-r1",
    )
    .is_err());
    let invalid = mapping(project, task, "wt-1", "agent/task-1");
    assert_eq!(invalid.worktree_id(), "wt-1");

    assert!(TaskWorkspaceMapping::new(
        project,
        task,
        "r".repeat(MAX_TASK_MAPPING_REPOSITORY_ID_LEN + 1),
        "wt-1",
        "agent/task-1",
        RunId::new(),
        None,
        TraceId::new(),
        "policy-r1",
    )
    .is_err());
    assert!(MappingObservation::new(
        "repo-1",
        "wt-1",
        format!("agent/{}", "b".repeat(MAX_TASK_MAPPING_BRANCH_LEN)),
        1,
        TraceId::new(),
    )
    .is_err());

    let mut registry = TaskWorkspaceMappingRegistry::new(1).unwrap();
    let invalid = mapping(project, task, "wt-1", "agent/task-1");
    registry.register(invalid).unwrap();
    assert_eq!(registry.len(), 1);
    assert!(registry
        .register(mapping(project, TaskId::new(), "wt-2", "agent/task-2"))
        .is_err());
    assert!(TaskWorkspaceMappingRegistry::new(0).is_err());
    assert!(TaskWorkspaceMappingRegistry::new(MAX_TASK_MAPPINGS + 1).is_err());

    let released = registry.release(project, task, 1, 500).unwrap();
    assert_eq!(released.state(), MappingState::Released);
    assert!(registry.resume(project, task, 2, 501).is_err());
}
