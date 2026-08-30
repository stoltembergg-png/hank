use plugin_core::lifecycle::*;

fn valid() -> LifecycleRequest {
    LifecycleRequest::new("plugin-a", "digest-1", "api-1", true, true, 2).unwrap()
}

#[test]
// @spec:AC-1393
fn approved_plugin_reaches_ready_and_stops_idempotently() {
    let mut lifecycle = PluginLifecycle::new(valid());
    assert_eq!(lifecycle.state(), LifecycleState::Pending);
    lifecycle.apply(LifecycleEvent::Start).unwrap();
    assert_eq!(lifecycle.state(), LifecycleState::Ready);
    lifecycle.apply(LifecycleEvent::Stop).unwrap();
    lifecycle.apply(LifecycleEvent::Stop).unwrap();
    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
}

#[test]
// @spec:AC-1394
fn failures_quarantine_and_restart_is_bounded() {
    let mut lifecycle = PluginLifecycle::new(valid());
    lifecycle.apply(LifecycleEvent::Start).unwrap();
    lifecycle.apply(LifecycleEvent::Crash).unwrap();
    assert_eq!(lifecycle.state(), LifecycleState::Quarantined);
    assert!(matches!(
        lifecycle.apply(LifecycleEvent::Start),
        Err(LifecycleError::Quarantined)
    ));

    let mut limited = PluginLifecycle::new(
        LifecycleRequest::new("plugin-b", "digest-2", "api-1", true, true, 0).unwrap(),
    );
    assert!(matches!(
        limited.apply(LifecycleEvent::Start),
        Err(LifecycleError::RestartLimit)
    ));
}
