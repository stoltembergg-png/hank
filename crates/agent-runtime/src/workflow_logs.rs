use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use thiserror::Error;

const MAX_ID_BYTES: usize = 128;
const ALLOWED_FIELDS: [&str; 5] = [
    "status",
    "error_code",
    "recovery_class",
    "attempt",
    "sequence",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EventKind {
    Start,
    Transition,
    End,
    Error,
    Recovery,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RetentionClass {
    Short,
    Standard,
    Long,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowLogEvent {
    pub project_id: String,
    pub run_id: String,
    pub node_id: String,
    pub event_id: String,
    pub kind: EventKind,
    pub severity: Severity,
    pub retention: RetentionClass,
    pub timestamp_ms: u64,
    pub fields: BTreeMap<String, String>,
}
impl WorkflowLogEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project: &str,
        run: &str,
        node: &str,
        event: &str,
        kind: EventKind,
        severity: Severity,
        retention: RetentionClass,
        timestamp_ms: u64,
    ) -> Result<Self, LogError> {
        for value in [project, run, node, event] {
            validate_id(value)?;
        }
        Ok(Self {
            project_id: project.into(),
            run_id: run.into(),
            node_id: node.into(),
            event_id: event.into(),
            kind,
            severity,
            retention,
            timestamp_ms,
            fields: BTreeMap::new(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LogError {
    #[error("workflow log identity is invalid")]
    InvalidIdentity,
    #[error("workflow log event is duplicated")]
    Duplicate,
    #[error("workflow log timestamp is out of order")]
    OutOfOrder,
    #[error("workflow log project scope is invalid")]
    ProjectScope,
    #[error("workflow log query/export budget is invalid")]
    Budget,
    #[error("workflow log sink is unavailable")]
    Sink,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LogMetrics {
    pub dropped: u64,
    pub redacted: u64,
}
#[derive(Default)]
struct Inner {
    events: Vec<WorkflowLogEvent>,
    ids: HashMap<(String, String, String), ()>,
    last_ts: HashMap<(String, String), u64>,
    metrics: LogMetrics,
}
#[derive(Clone)]
pub struct WorkflowLogStore {
    inner: Arc<Mutex<Inner>>,
    max_events: usize,
    max_export_bytes: usize,
}
impl WorkflowLogStore {
    pub fn new(max_events: usize, max_export_bytes: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            max_events: max_events.max(1),
            max_export_bytes: max_export_bytes.max(64),
        }
    }
    pub fn append(&self, mut event: WorkflowLogEvent) -> Result<(), LogError> {
        let mut inner = self.inner.lock().map_err(|_| LogError::Sink)?;
        let id = (
            event.project_id.clone(),
            event.run_id.clone(),
            event.event_id.clone(),
        );
        if inner.ids.contains_key(&id) {
            return Err(LogError::Duplicate);
        }
        let scope = (event.project_id.clone(), event.run_id.clone());
        if inner
            .last_ts
            .get(&scope)
            .is_some_and(|last| event.timestamp_ms < *last)
        {
            return Err(LogError::OutOfOrder);
        }
        let mut redacted = false;
        event.fields.retain(|key, value| {
            if !ALLOWED_FIELDS.contains(&key.as_str()) {
                redacted = true;
                return false;
            }
            if contains_sensitive(value) {
                redacted = true;
                return false;
            }
            true
        });
        if redacted {
            inner.metrics.redacted += 1;
        }
        if inner.events.len() >= self.max_events {
            let removed = inner.events.remove(0);
            inner
                .ids
                .remove(&(removed.project_id, removed.run_id, removed.event_id));
            inner.metrics.dropped += 1;
        }
        inner.last_ts.insert(scope, event.timestamp_ms);
        inner.ids.insert(id, ());
        inner.events.push(event);
        Ok(())
    }
    pub fn query(
        &self,
        project: &str,
        run: &str,
        limit: usize,
    ) -> Result<Vec<WorkflowLogEvent>, LogError> {
        validate_id(project)?;
        validate_id(run)?;
        if limit == 0 || limit > self.max_events {
            return Err(LogError::Budget);
        }
        let inner = self.inner.lock().map_err(|_| LogError::Sink)?;
        Ok(inner
            .events
            .iter()
            .filter(|e| e.project_id == project && e.run_id == run)
            .take(limit)
            .cloned()
            .collect())
    }
    pub fn export(&self, project: &str, run: &str, limit: usize) -> Result<String, LogError> {
        let mut values = self.query(project, run, limit)?;
        loop {
            let encoded = serde_json::to_string(&values).map_err(|_| LogError::Sink)?;
            if encoded.len() <= self.max_export_bytes {
                return Ok(encoded);
            }
            if values.pop().is_none() {
                return Ok("[]".into());
            }
        }
    }
    pub fn metrics(&self) -> LogMetrics {
        self.inner
            .lock()
            .map(|inner| inner.metrics)
            .unwrap_or_default()
    }
}
fn validate_id(value: &str) -> Result<(), LogError> {
    if value.trim().is_empty() || value.len() > MAX_ID_BYTES || value.chars().any(char::is_control)
    {
        Err(LogError::InvalidIdentity)
    } else {
        Ok(())
    }
}
fn contains_sensitive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("https://")
        || lower.contains("http://")
        || lower.contains("/")
        || lower.contains("page")
}
