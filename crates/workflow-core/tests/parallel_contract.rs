use workflow_core::parallel::{
    BranchResult, JoinDecision, JoinPolicy, ParallelError, ParallelPlan,
};

// @spec:AC-995
#[test]
fn fanout_and_concurrency_are_bounded_and_unique() {
    assert!(ParallelPlan::new(vec!["a".into(), "b".into()], 2, 1).is_ok());
    assert!(matches!(
        ParallelPlan::new(vec![], 1, 1),
        Err(ParallelError::InvalidLimit)
    ));
    assert!(matches!(
        ParallelPlan::new(vec!["a".into(), "a".into()], 2, 1),
        Err(ParallelError::DuplicateBranch)
    ));
    assert!(matches!(
        ParallelPlan::new(vec!["a".into(), "b".into()], 1, 1),
        Err(ParallelError::FanoutExceeded)
    ));
}

// @spec:AC-996
#[test]
fn joins_return_declared_order_and_deterministic_decisions() {
    let plan = ParallelPlan::new(vec!["a".into(), "b".into(), "c".into()], 3, 2).unwrap();
    let results = vec![
        BranchResult::Success("c".into()),
        BranchResult::Success("a".into()),
        BranchResult::Failed("b".into()),
    ];
    let joined = plan.join(JoinPolicy::Any, results.clone()).unwrap();
    assert_eq!(
        joined.ordered,
        vec![
            BranchResult::Success("a".into()),
            BranchResult::Failed("b".into()),
            BranchResult::Success("c".into())
        ]
    );
    assert_eq!(joined.decision, JoinDecision::Satisfied);
    assert_eq!(
        plan.join(JoinPolicy::All, results).unwrap().decision,
        JoinDecision::Failed
    );
}

// @spec:AC-997
#[test]
fn quorum_and_cancel_are_typed_without_orphan_work() {
    let mut plan = ParallelPlan::new(vec!["a".into(), "b".into(), "c".into()], 3, 2).unwrap();
    assert!(plan.cancel());
    assert!(!plan.cancel());
    assert_eq!(
        plan.join(
            JoinPolicy::Quorum(2),
            vec![
                BranchResult::Cancelled("a".into()),
                BranchResult::Success("b".into()),
                BranchResult::Success("c".into())
            ]
        )
        .unwrap()
        .decision,
        JoinDecision::Cancelled
    );
}
