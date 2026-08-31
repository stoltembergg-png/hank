use security_core::plugin_permission::*;

fn request(id: u64, digest: &str, version: &str, capability: &str) -> PluginPermissionRequest {
    PluginPermissionRequest::new(
        id,
        "policy-1",
        "plugin-a",
        digest,
        version,
        capability,
        "project-a",
        "agent-a",
        PluginAction::Use,
    )
}

fn engine() -> PluginPermissionEngine {
    PluginPermissionEngine::new("policy-1").unwrap()
}

#[test]
// @spec:AC-1395
fn plugin_permissions_are_default_deny_and_exactly_bound() {
    let mut engine = engine();
    assert_eq!(
        engine.evaluate(request(1, "digest-1", "1.0.0", "read")),
        Ok(PluginPermissionDecision::Denied)
    );
    engine
        .grant(
            PluginGrant::new(
                "plugin-a",
                "digest-1",
                "1.0.0",
                "read",
                "project-a",
                "agent-a",
                GrantDuration::Persistent,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        engine.evaluate(request(2, "digest-1", "1.0.0", "read")),
        Ok(PluginPermissionDecision::Allowed)
    );
    assert_eq!(
        engine.evaluate(request(3, "digest-other", "1.0.0", "read")),
        Ok(PluginPermissionDecision::Denied)
    );
    assert_eq!(
        engine.evaluate(request(4, "digest-1", "1.0.0", "network")),
        Ok(PluginPermissionDecision::Denied)
    );
}

#[test]
// @spec:AC-1396
fn revoke_and_new_capability_upgrade_require_reconsent() {
    let mut engine = engine();
    engine
        .grant(
            PluginGrant::new(
                "plugin-a",
                "digest-1",
                "1.0.0",
                "read",
                "project-a",
                "agent-a",
                GrantDuration::Persistent,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .revoke("plugin-a", "digest-1", "1.0.0", "read")
        .unwrap();
    assert_eq!(
        engine.evaluate(request(5, "digest-1", "1.0.0", "read")),
        Ok(PluginPermissionDecision::Denied)
    );
    assert_eq!(
        engine.evaluate(request(6, "digest-2", "2.0.0", "network")),
        Ok(PluginPermissionDecision::Denied)
    );
    assert!(matches!(
        engine.evaluate(request(5, "digest-1", "1.0.0", "read")),
        Err(PluginPermissionError::Replay)
    ));
}
