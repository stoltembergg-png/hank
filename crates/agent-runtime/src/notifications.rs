use std::collections::HashSet;

const MAX_TEXT_BYTES: usize = 160;
const MAX_DEDUP_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Success,
    Failure,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEvent {
    pub project_id: String,
    pub run_id: String,
    pub event_id: String,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPreferences {
    pub enabled: bool,
    pub max_per_window: usize,
}

impl NotificationPreferences {
    pub fn enabled(max_per_window: usize) -> Self {
        Self {
            enabled: true,
            max_per_window,
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            max_per_window: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub project_id: String,
    pub run_id: String,
    pub event_id: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub deep_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationDecision {
    Deliver(Notification),
    Suppressed(&'static str),
}

#[derive(Debug)]
pub struct NotificationPolicy {
    preferences: NotificationPreferences,
    seen: HashSet<String>,
    delivered: usize,
}

impl NotificationPolicy {
    pub fn new(preferences: NotificationPreferences) -> Self {
        Self {
            preferences,
            seen: HashSet::new(),
            delivered: 0,
        }
    }

    pub fn evaluate(&mut self, event: NotificationEvent) -> NotificationDecision {
        if !self.preferences.enabled {
            return NotificationDecision::Suppressed("disabled");
        }
        if !valid_id(&event.project_id) || !valid_id(&event.run_id) || !valid_id(&event.event_id) {
            return NotificationDecision::Suppressed("invalid_scope");
        }
        if self.seen.contains(&event.event_id) {
            return NotificationDecision::Suppressed("duplicate");
        }
        if self.delivered >= self.preferences.max_per_window {
            return NotificationDecision::Suppressed("rate_limited");
        }
        let severity = match event.kind {
            NotificationKind::Success => "success",
            NotificationKind::Failure => "failure",
            NotificationKind::Approval => "approval",
        };
        let notification = Notification {
            project_id: event.project_id.clone(),
            run_id: event.run_id.clone(),
            event_id: event.event_id.clone(),
            severity: severity.into(),
            title: redact(&event.title),
            body: redact(&event.body),
            deep_link: format!("hank://runs/{}/{}", event.project_id, event.run_id),
        };
        self.seen.insert(event.event_id);
        if self.seen.len() > MAX_DEDUP_ENTRIES {
            self.seen.clear();
        }
        self.delivered += 1;
        NotificationDecision::Deliver(notification)
    }

    pub fn deep_link(
        project_id: &str,
        run_id: &str,
        allowed_project_id: &str,
        allowed_run_id: &str,
        extra_parameters: &[&str],
    ) -> Option<String> {
        if project_id != allowed_project_id
            || run_id != allowed_run_id
            || !extra_parameters.is_empty()
            || !valid_id(project_id)
            || !valid_id(run_id)
        {
            return None;
        }
        Some(format!("hank://runs/{project_id}/{run_id}"))
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn redact(value: &str) -> String {
    let mut result = value
        .replace(['<', '>'], "")
        .replace("token=", "[REDACTED]=")
        .replace("secret", "[REDACTED]");
    if result.len() > MAX_TEXT_BYTES {
        result.truncate(MAX_TEXT_BYTES);
    }
    result
}
