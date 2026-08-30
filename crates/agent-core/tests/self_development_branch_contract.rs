use agent_core::self_development_branch::*;

fn valid() -> BranchRequest {
    BranchRequest::new(
        "issue-1",
        "candidate-1",
        "v1",
        "base-sha",
        "policy-1",
        "/srv/hank-worktrees",
        "project-1",
        "feature/task-1",
        false,
        true,
        true,
    )
    .unwrap()
}

// @spec:AC-1373
#[test]
fn valid_mapping_is_deterministic_and_fail_closed() {
    let mapping = BranchMapping::create(valid()).unwrap();
    assert_eq!(mapping.key(), BranchMapping::create(valid()).unwrap().key());
    assert_eq!(mapping.branch(), "feature/task-1");
    assert!(matches!(
        BranchMapping::create({
            let mut r = valid();
            r.policy_allowed = false;
            r
        }),
        Err(BranchError::PolicyDenied)
    ));
    assert!(matches!(
        BranchMapping::create({
            let mut r = valid();
            r.issue_present = false;
            r
        }),
        Err(BranchError::IssueMissing)
    ));
}

// @spec:AC-1374
#[test]
fn protected_unknown_and_expired_lifecycle_are_bounded() {
    let mut protected = valid();
    protected.protected_branch = true;
    assert!(matches!(
        BranchMapping::create(protected),
        Err(BranchError::ProtectedBranch)
    ));
    let mut outside = valid();
    outside.root = "/tmp/other".into();
    assert!(matches!(
        BranchMapping::create(outside),
        Err(BranchError::RootNotAllowed)
    ));
    let mapping = BranchMapping::create(valid()).unwrap();
    assert_eq!(
        mapping.cleanup(CleanupTarget::Unknown),
        CleanupAction::Preserve
    );
    assert_eq!(
        mapping.cleanup(CleanupTarget::ExpiredRegistered),
        CleanupAction::CleanupRegistered
    );
}
