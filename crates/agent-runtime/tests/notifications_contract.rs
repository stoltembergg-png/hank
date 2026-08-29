use agent_runtime::notifications::{
    DeliveryOutcome, NotificationDecision, NotificationEvent, NotificationKind, NotificationPolicy,
    NotificationPreferences, NotificationSink, NotificationWorker, PermissionState,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
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

struct FixtureSink {
    permission: PermissionState,
    calls: Arc<AtomicUsize>,
    succeeds: bool,
}

impl NotificationSink for FixtureSink {
    fn permission(&self) -> PermissionState {
        self.permission
    }

    fn deliver(&mut self, _notification: &agent_runtime::notifications::Notification) -> bool {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.succeeds
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
// @spec:AC-1286
fn deduplication_key_includes_project_and_run_scope() {
    let mut policy = NotificationPolicy::new(NotificationPreferences::enabled(10));
    assert!(matches!(
        policy.evaluate(event(NotificationKind::Success, "project a")),
        NotificationDecision::Deliver(_)
    ));

    let mut other_scope = event(NotificationKind::Success, "project b");
    other_scope.project_id = "project-b".into();
    other_scope.run_id = "run-2".into();
    assert!(matches!(
        policy.evaluate(other_scope),
        NotificationDecision::Deliver(_)
    ));

    assert_eq!(
        policy.evaluate(event(NotificationKind::Failure, "same scope")),
        NotificationDecision::Suppressed("duplicate")
    );
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
// @spec:AC-1287
fn denied_or_unavailable_sink_is_safe_and_non_blocking() {
    for permission in [PermissionState::Denied, PermissionState::Unavailable] {
        let mut worker = NotificationWorker::new(FixtureSink {
            permission,
            calls: Arc::new(AtomicUsize::new(0)),
            succeeds: true,
        });
        let notification = match NotificationPolicy::new(NotificationPreferences::enabled(1))
            .evaluate(event(NotificationKind::Success, "safe"))
        {
            NotificationDecision::Deliver(notification) => notification,
            _ => panic!("event should be deliverable before sink policy"),
        };
        assert_eq!(
            worker.deliver(&notification),
            DeliveryOutcome::Suppressed("permission")
        );
    }
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

#[test]
// @spec:AC-1297
fn granted_sink_is_called_once_and_reports_delivery() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut worker = NotificationWorker::new(FixtureSink {
        permission: PermissionState::Granted,
        calls: Arc::clone(&calls),
        succeeds: true,
    });
    let notification = match NotificationPolicy::new(NotificationPreferences::enabled(1))
        .evaluate(event(NotificationKind::Success, "safe"))
    {
        NotificationDecision::Deliver(notification) => notification,
        _ => panic!("event should be deliverable"),
    };

    assert_eq!(worker.deliver(&notification), DeliveryOutcome::Delivered);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
// @spec:AC-1298
fn failed_sink_returns_controlled_outcome_and_worker_remains_usable() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut worker = NotificationWorker::new(FixtureSink {
        permission: PermissionState::Granted,
        calls: Arc::clone(&calls),
        succeeds: false,
    });
    let notification = match NotificationPolicy::new(NotificationPreferences::enabled(2))
        .evaluate(event(NotificationKind::Failure, "safe"))
    {
        NotificationDecision::Deliver(notification) => notification,
        _ => panic!("event should be deliverable"),
    };

    assert_eq!(worker.deliver(&notification), DeliveryOutcome::Failed);
    assert_eq!(worker.deliver(&notification), DeliveryOutcome::Failed);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
