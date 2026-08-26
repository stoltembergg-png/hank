use agent_runtime::notifications::{
    NotificationDecision, NotificationEvent, NotificationKind, NotificationPolicy,
    NotificationPreferences,
};

fn event(kind: NotificationKind, body: &str) -> NotificationEvent {
    NotificationEvent {
        project_id: "project-a".into(),
        run_id: "run-1".into(),
        event_id: "event-1".into(),
        kind,
        title: "Job completed".into(),
        body: body.into(),
    }
}

#[test]
// @spec:AC-1285
fn allowed_terminal_events_have_explicit_severity_and_redacted_content() {
    let mut policy = NotificationPolicy::new(NotificationPreferences::enabled(10));
    let decision = policy.evaluate(event(
        NotificationKind::Success,
        "ignore <script>token=secret</script>",
    ));
    let NotificationDecision::Deliver(notification) = decision else {
        panic!("terminal event must be delivered");
    };
    assert_eq!(notification.severity, "success");
    assert!(!notification.title.contains("<"));
    assert!(!notification.body.contains("token="));
    assert!(!notification.body.contains("secret"));
}

#[test]
// @spec:AC-1286
fn duplicate_event_is_suppressed_without_changing_first_decision() {
    let mut policy = NotificationPolicy::new(NotificationPreferences::enabled(10));
    let first = policy.evaluate(event(NotificationKind::Failure, "failure details"));
    let second = policy.evaluate(event(NotificationKind::Failure, "different details"));
    assert!(matches!(first, NotificationDecision::Deliver(_)));
    assert_eq!(second, NotificationDecision::Suppressed("duplicate"));
}

#[test]
// @spec:AC-1287
fn disabled_preferences_and_rate_limit_fail_closed() {
    let mut disabled = NotificationPolicy::new(NotificationPreferences::disabled());
    assert_eq!(
        disabled.evaluate(event(NotificationKind::Approval, "approve")),
        NotificationDecision::Suppressed("disabled")
    );

    let mut limited = NotificationPolicy::new(NotificationPreferences::enabled(1));
    assert!(matches!(
        limited.evaluate(event(NotificationKind::Success, "one")),
        NotificationDecision::Deliver(_)
    ));
    let mut second = event(NotificationKind::Failure, "two");
    second.event_id = "event-2".into();
    assert_eq!(
        limited.evaluate(second),
        NotificationDecision::Suppressed("rate_limited")
    );
}

#[test]
// @spec:AC-1288
fn deep_link_rejects_foreign_scope_and_unknown_data() {
    let safe = NotificationPolicy::deep_link("project-a", "run-1", "project-a", "run-1", &[]);
    assert_eq!(safe.as_deref(), Some("hank://runs/project-a/run-1"));
    assert!(
        NotificationPolicy::deep_link("project-b", "run-1", "project-a", "run-1", &[]).is_none()
    );
    assert!(
        NotificationPolicy::deep_link("project-a", "run-1", "project-a", "run-1", &["token"])
            .is_none()
    );
}
