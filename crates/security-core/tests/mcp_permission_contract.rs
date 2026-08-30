use security_core::mcp_permission::*;

fn request(id: u64, action: PermissionAction) -> PermissionRequest {
    PermissionRequest::new(
        id,
        "policy-1",
        "server-a",
        "tool-b",
        "origin-a",
        "project-a",
        "agent-a",
        action,
    )
}

fn engine() -> PermissionEngine {
    PermissionEngine::new("policy-1").unwrap()
}

// @spec:AC-1383
#[test]
fn default_deny_and_scope_action_isolation() {
    let mut engine = engine();
    assert_eq!(
        engine.evaluate(request(1, PermissionAction::Discovery), 10),
        Ok(PermissionDecision::Denied)
    );
    engine
        .grant(
            Grant::new(
                "server-a",
                "tool-b",
                "origin-a",
                "project-a",
                "agent-a",
                PermissionAction::Discovery,
                GrantDuration::Persistent,
                20,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        engine.evaluate(request(2, PermissionAction::Discovery), 10),
        Ok(PermissionDecision::Allowed)
    );
    assert_eq!(
        engine.evaluate(request(3, PermissionAction::Execution), 10),
        Ok(PermissionDecision::Denied)
    );
    let mut foreign = request(4, PermissionAction::Discovery);
    foreign.project = "project-b".into();
    assert_eq!(engine.evaluate(foreign, 10), Ok(PermissionDecision::Denied));
}

// @spec:AC-1384
#[test]
fn grant_lifecycle_revoke_expiry_and_replay_are_fail_closed() {
    let mut engine = engine();
    engine
        .grant(
            Grant::new(
                "server-a",
                "tool-b",
                "origin-a",
                "project-a",
                "agent-a",
                PermissionAction::Execution,
                GrantDuration::OneShot,
                11,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        engine.evaluate(request(5, PermissionAction::Execution), 10),
        Ok(PermissionDecision::Allowed)
    );
    assert_eq!(
        engine.evaluate(request(6, PermissionAction::Execution), 10),
        Ok(PermissionDecision::Denied)
    );
    assert_eq!(
        engine.evaluate(request(5, PermissionAction::Execution), 10),
        Err(PermissionError::Replay)
    );
    engine
        .grant(
            Grant::new(
                "server-a",
                "tool-b",
                "origin-a",
                "project-a",
                "agent-a",
                PermissionAction::Execution,
                GrantDuration::Persistent,
                100,
            )
            .unwrap(),
        )
        .unwrap();
    engine.revoke("server-a", "tool-b").unwrap();
    assert_eq!(
        engine.evaluate(request(7, PermissionAction::Execution), 10),
        Ok(PermissionDecision::Denied)
    );
    let stale = PermissionRequest::new(
        8,
        "policy-old",
        "server-a",
        "tool-b",
        "origin-a",
        "project-a",
        "agent-a",
        PermissionAction::Execution,
    );
    assert_eq!(
        engine.evaluate(stale, 10),
        Err(PermissionError::PolicyStale)
    );
}
