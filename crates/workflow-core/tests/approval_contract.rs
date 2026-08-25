use workflow_core::approval::{
    ApprovalBinding, ApprovalDecision, ApprovalError, ApprovalLedger, ApprovalState,
};

fn binding(generation: u64) -> ApprovalBinding {
    ApprovalBinding::new("project-1", "workflow-1", "run-1", "node-1", generation).unwrap()
}

// @spec:AC-1013
#[test]
fn allow_deny_expiry_and_cancel_are_terminal() {
    let ledger = ApprovalLedger::new(8).unwrap();
    let allow = ledger.submit(binding(1), "alice", 100, 50, 1_000).unwrap();
    let token = ledger
        .decide(
            allow.request_id,
            &binding(1),
            "alice",
            ApprovalDecision::Allow,
            120,
        )
        .unwrap()
        .expect("allow decision must issue token");
    assert_eq!(
        ledger.state(allow.request_id),
        Some(ApprovalState::Approved)
    );
    assert!(ledger
        .resume(allow.request_id, &binding(1), &token, 121)
        .is_ok());

    let deny = ledger.submit(binding(2), "alice", 100, 50, 1_000).unwrap();
    assert_eq!(
        ledger
            .decide(
                deny.request_id,
                &binding(2),
                "alice",
                ApprovalDecision::Deny,
                120
            )
            .unwrap(),
        None
    );
    assert_eq!(ledger.state(deny.request_id), Some(ApprovalState::Denied));

    let expired = ledger.submit(binding(3), "alice", 100, 10, 1_000).unwrap();
    assert!(matches!(
        ledger.decide(
            expired.request_id,
            &binding(3),
            "alice",
            ApprovalDecision::Allow,
            110
        ),
        Err(ApprovalError::Expired)
    ));
    let cancelled = ledger.submit(binding(4), "alice", 100, 50, 1_000).unwrap();
    ledger.cancel(cancelled.request_id, &binding(4)).unwrap();
    assert_eq!(
        ledger.state(cancelled.request_id),
        Some(ApprovalState::Cancelled)
    );
}

// @spec:AC-1014
#[test]
fn wrong_actor_and_stale_binding_do_not_mutate_pending_state() {
    let ledger = ApprovalLedger::new(4).unwrap();
    let request = ledger.submit(binding(7), "alice", 100, 50, 1_000).unwrap();
    assert!(matches!(
        ledger.decide(
            request.request_id,
            &binding(7),
            "mallory",
            ApprovalDecision::Allow,
            110
        ),
        Err(ApprovalError::ActorMismatch)
    ));
    assert!(matches!(
        ledger.decide(
            request.request_id,
            &binding(8),
            "alice",
            ApprovalDecision::Allow,
            110
        ),
        Err(ApprovalError::BindingMismatch)
    ));
    assert_eq!(
        ledger.state(request.request_id),
        Some(ApprovalState::Pending)
    );
}

// @spec:AC-1015
#[test]
fn resume_is_one_time_and_capacity_is_bounded() {
    let ledger = ApprovalLedger::new(1).unwrap();
    let first = ledger.submit(binding(1), "alice", 100, 50, 1_000).unwrap();
    assert!(matches!(
        ledger.submit(binding(2), "alice", 100, 50, 1_000),
        Err(ApprovalError::CapacityFull)
    ));
    let token = ledger
        .decide(
            first.request_id,
            &binding(1),
            "alice",
            ApprovalDecision::Allow,
            110,
        )
        .unwrap()
        .expect("allow decision must issue token");
    assert!(ledger
        .resume(first.request_id, &binding(1), &token, 111)
        .is_ok());
    assert!(matches!(
        ledger.resume(first.request_id, &binding(1), &token, 112),
        Err(ApprovalError::Replay)
    ));
}
