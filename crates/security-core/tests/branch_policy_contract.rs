use security_core::{
    BranchDecision, BranchMutation, BranchPolicy, BranchPolicyError, BranchPolicyRequest,
};

fn policy() -> BranchPolicy {
    BranchPolicy::new(
        "project-1",
        "repo-1",
        "policy-1",
        "agent/",
        vec!["main".into(), "release".into()],
    )
    .unwrap()
}

fn request(operation: BranchMutation) -> BranchPolicyRequest {
    BranchPolicyRequest::new(
        "project-1",
        "repo-1",
        "task-1",
        "owner-1",
        "owner-1",
        "agent/task-1",
        "main",
        "policy-1",
        operation,
    )
}

#[test]
// @spec:AC-1313
fn allows_only_owned_task_branch_local_commit_and_push() {
    let policy = policy();

    for operation in [BranchMutation::LocalCommit, BranchMutation::Push] {
        assert_eq!(
            policy.evaluate(&request(operation)),
            Ok(BranchDecision::Allowed {
                policy_revision: "policy-1".into(),
                operation,
            })
        );
    }
}

#[test]
// @spec:AC-1314
fn denies_protected_branch_force_push_and_merge_without_fallback() {
    let policy = policy();

    let mut protected = request(BranchMutation::LocalCommit);
    protected.branch = "main".into();
    assert_eq!(
        policy.evaluate(&protected),
        Err(BranchPolicyError::ProtectedBranch)
    );

    assert_eq!(
        policy.evaluate(&request(BranchMutation::ForcePush)),
        Err(BranchPolicyError::ForcePushDenied)
    );
    assert_eq!(
        policy.evaluate(&request(BranchMutation::Merge)),
        Err(BranchPolicyError::MergeDenied)
    );
}

#[test]
// @spec:AC-1315
fn rejects_scope_owner_branch_and_stale_policy_without_mutating_policy() {
    let policy = policy();
    let before = policy.clone();

    let mut foreign_project = request(BranchMutation::Push);
    foreign_project.project_id = "project-2".into();
    assert_eq!(
        policy.evaluate(&foreign_project),
        Err(BranchPolicyError::ScopeMismatch)
    );

    let mut foreign_repository = request(BranchMutation::Push);
    foreign_repository.repository_id = "repo-2".into();
    assert_eq!(
        policy.evaluate(&foreign_repository),
        Err(BranchPolicyError::ScopeMismatch)
    );

    let mut foreign_actor = request(BranchMutation::Push);
    foreign_actor.actor_id = "owner-2".into();
    assert_eq!(
        policy.evaluate(&foreign_actor),
        Err(BranchPolicyError::ActorNotOwner)
    );

    let mut stale = request(BranchMutation::Push);
    stale.policy_revision = "policy-0".into();
    assert_eq!(
        policy.evaluate(&stale),
        Err(BranchPolicyError::PolicyRevisionMismatch)
    );

    let mut wrong_branch = request(BranchMutation::Push);
    wrong_branch.branch = "agent/other-task".into();
    assert_eq!(
        policy.evaluate(&wrong_branch),
        Err(BranchPolicyError::BranchTaskMismatch)
    );

    for invalid_branch in [
        "-agent/task-1",
        "agent/task-1/",
        "agent/task-1..other",
        "agent/task 1",
        "agent/task~1",
        "agent/task^1",
        "agent/task:1",
        "agent/task?1",
        "agent/task*1",
        "agent/task[1",
        "agent/task\\\\1",
    ] {
        let mut invalid = request(BranchMutation::Push);
        invalid.branch = invalid_branch.into();
        assert_eq!(
            policy.evaluate(&invalid),
            Err(BranchPolicyError::InvalidRequest)
        );
    }

    let invalid_policy = BranchPolicy::new(
        "project-1",
        "repo-1",
        "policy-1",
        "agent/",
        vec!["bad branch".into()],
    );
    assert_eq!(invalid_policy, Err(BranchPolicyError::InvalidPolicy));

    assert_eq!(policy, before);
}
