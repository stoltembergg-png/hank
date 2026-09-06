use test_support::agent_loop::{run_loop, LoopError, LoopPolicy, LoopStatus, LoopStep};

fn policy() -> LoopPolicy {
    LoopPolicy {
        max_turns: 8,
        max_depth: 2,
        budget: 10,
        cancelled: false,
    }
}

// @spec:AC-2501
#[test]
fn success_and_trace_are_deterministic() {
    let steps = [
        LoopStep::Tool {
            key: "read".into(),
            cost: 2,
            allowed: true,
        },
        LoopStep::Finish,
    ];
    let first = run_loop(&steps, policy()).unwrap();
    let second = run_loop(&steps, policy()).unwrap();
    assert_eq!(first.status, LoopStatus::Success);
    assert_eq!(first.trace_digest, second.trace_digest);
}

// @spec:AC-2502
#[test]
fn duplicate_tool_is_not_charged_twice() {
    let steps = [
        LoopStep::Tool {
            key: "write".into(),
            cost: 4,
            allowed: true,
        },
        LoopStep::Tool {
            key: "write".into(),
            cost: 4,
            allowed: true,
        },
        LoopStep::Finish,
    ];
    let report = run_loop(&steps, policy()).unwrap();
    assert_eq!(report.status, LoopStatus::Success);
    assert!(report.events.iter().any(|event| event.kind == "duplicate"));
    assert_eq!(report.spent, 4);
}

// @spec:AC-2503
#[test]
fn permission_cycle_depth_and_budget_stop_fail_closed() {
    assert_eq!(
        run_loop(
            &[LoopStep::Tool {
                key: "x".into(),
                cost: 1,
                allowed: false
            }],
            policy()
        )
        .unwrap()
        .status,
        LoopStatus::PermissionDenied
    );
    assert_eq!(
        run_loop(
            &[
                LoopStep::Delegate {
                    target: "a".into(),
                    depth: 1
                },
                LoopStep::Delegate {
                    target: "a".into(),
                    depth: 1
                }
            ],
            policy()
        )
        .unwrap()
        .status,
        LoopStatus::CycleDenied
    );
    assert_eq!(
        run_loop(
            &[LoopStep::Delegate {
                target: "b".into(),
                depth: 3
            }],
            policy()
        )
        .unwrap()
        .status,
        LoopStatus::DepthDenied
    );
    let collision = run_loop(
        &[
            LoopStep::Tool {
                key: "same".into(),
                cost: 1,
                allowed: true,
            },
            LoopStep::Delegate {
                target: "same".into(),
                depth: 1,
            },
        ],
        policy(),
    )
    .unwrap();
    assert_eq!(collision.status, LoopStatus::TurnLimit);
    assert_eq!(collision.events.len(), 2);
    assert_eq!(
        run_loop(
            &[
                LoopStep::Retry { cost: u32::MAX },
                LoopStep::Retry { cost: 1 },
            ],
            LoopPolicy {
                budget: u32::MAX,
                ..policy()
            }
        )
        .unwrap()
        .status,
        LoopStatus::BudgetExceeded
    );
}

// @spec:AC-2504
#[test]
fn cancellation_and_stale_event_do_not_advance() {
    let mut cancelled = policy();
    cancelled.cancelled = true;
    let report = run_loop(
        &[LoopStep::Tool {
            key: "x".into(),
            cost: 1,
            allowed: true,
        }],
        cancelled,
    )
    .unwrap();
    assert_eq!(report.status, LoopStatus::Cancelled);
    assert_eq!(
        run_loop(&[LoopStep::MissingEvent], policy())
            .unwrap()
            .status,
        LoopStatus::StaleEvent
    );
}

// @spec:AC-2505
#[test]
fn invalid_policy_and_turn_bound_fail_closed() {
    assert_eq!(
        run_loop(
            &[],
            LoopPolicy {
                max_turns: 0,
                ..policy()
            }
        ),
        Err(LoopError::InvalidPolicy)
    );
    let steps = vec![LoopStep::Retry { cost: 0 }; 9];
    assert_eq!(
        run_loop(&steps, policy()).unwrap().status,
        LoopStatus::TurnLimit
    );
}
