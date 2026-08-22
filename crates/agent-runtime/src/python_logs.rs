//! Bounded structured logging for the Python worker sidecar.
//!
//! Logs bind worker/project/session/task/trace correlation, are redacted
//! before being retained, and are treated as untrusted data — a log line is
//! never executed and never changes policy. Capture, retention and rotation
//! are bounded in records and bytes; nothing is written to disk here, the
//! buffer is the in-memory retention with an explicit rotation drain.

use std::collections::VecDeque;

/// Hard caps for worker log lines, retention and disk budget.
pub const MAX_LOG_LINE_BYTES: usize = 2_048;
pub const MAX_MESSAGE_CHARS: usize = 512;
pub const MAX_LOG_RECORDS: usize = 256;
pub const MAX_RETENTION_BYTES: usize = 262_144;
const TRUNCATION_MARKER: &str = "...[truncated]";
const REDACTED: &str = "[redacted]";
const SECRET_KEYS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "api_key",
    "apikey",
    "authorization",
    "auth",
    "bearer",
    "credential",
    "private_key",
];

/// Log level carried by every record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PythonLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Where the line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonLogSource {
    Stdout,
    Stderr,
    Lifecycle,
}

/// Identity bound to every captured record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonLogCorrelation {
    pub worker_id: String,
    pub project_id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub trace_id: String,
}

impl PythonLogCorrelation {
    /// Validates the bounded identity fields (non-empty, <=128, no control).
    pub fn validate(&self) -> bool {
        valid_id(&self.worker_id)
            && valid_id(&self.project_id)
            && valid_id(&self.session_id)
            && valid_id(&self.trace_id)
            && self.task_id.as_deref().is_none_or(valid_id)
    }
}

/// One retained, redacted, bounded log record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonLogRecord {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub level: PythonLogLevel,
    pub source: PythonLogSource,
    pub correlation: PythonLogCorrelation,
    pub message: String,
}

/// Deterministic redaction of untrusted worker output.
#[derive(Debug, Clone, Copy, Default)]
pub struct PythonLogRedactor;

impl PythonLogRedactor {
    /// Redacts a raw line: strips ANSI/control characters, masks secret-like
    /// values, neutralizes path traversal and truncates to the bounded size.
    pub fn redact(raw: &str) -> String {
        let bounded: String = if raw.len() > MAX_LOG_LINE_BYTES {
            let cut = raw
                .char_indices()
                .map(|(index, _)| index)
                .take_while(|index| *index <= MAX_LOG_LINE_BYTES)
                .last()
                .unwrap_or(0);
            let mut truncated = raw[..cut].to_string();
            truncated.push_str(TRUNCATION_MARKER);
            truncated
        } else {
            raw.to_string()
        };

        let cleaned: String = bounded
            .chars()
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect();

        let neutralized = cleaned.replace("..", "_");
        redact_secrets(&neutralized)
    }
}

/// Masks `key=value` / `key: value` pairs whose key looks like a secret and
/// bareword chains like `Authorization: Bearer <token>`. Whitespace is
/// normalized — redacted lines are derived data, not verbatim output.
fn redact_secrets(text: &str) -> String {
    fn bare_key(token: &str) -> String {
        token
            .trim_matches(|character: char| {
                character == ',' || character == '"' || character == '.'
            })
            .to_ascii_lowercase()
    }

    fn is_secret_key(candidate: &str) -> bool {
        SECRET_KEYS.contains(&candidate)
    }

    let mut output: Vec<String> = Vec::new();
    let mut mask_next = false;
    for token in text.split_whitespace() {
        if mask_next {
            let chains = is_secret_key(&bare_key(token));
            output.push(REDACTED.to_string());
            mask_next = chains;
            continue;
        }
        if let Some((key_part, value_part)) = token.split_once(['=', ':']) {
            let key = bare_key(key_part);
            if is_secret_key(&key) {
                if value_part.is_empty() {
                    output.push(token.to_string());
                    mask_next = true;
                } else {
                    let separator = &token[key_part.len()..key_part.len() + 1];
                    output.push(format!("{key_part}{separator}{REDACTED}"));
                }
                continue;
            }
        }
        if is_secret_key(&bare_key(token)) {
            output.push(token.to_string());
            mask_next = true;
            continue;
        }
        output.push(token.to_string());
    }
    output.join(" ")
}

/// Bounded in-memory retention with rotation and byte budget.
#[derive(Debug)]
pub struct PythonLogCapture {
    records: VecDeque<PythonLogRecord>,
    next_sequence: u64,
    total_bytes: usize,
    dropped: usize,
    redacted_count: u64,
    rotations: u64,
    capacity: usize,
    budget_bytes: usize,
}

impl Default for PythonLogCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonLogCapture {
    pub fn new() -> Self {
        Self::with_limits(MAX_LOG_RECORDS, MAX_RETENTION_BYTES)
    }

    pub fn with_limits(capacity: usize, budget_bytes: usize) -> Self {
        Self {
            records: VecDeque::new(),
            next_sequence: 0,
            total_bytes: 0,
            dropped: 0,
            redacted_count: 0,
            rotations: 0,
            capacity: capacity.max(1),
            budget_bytes: budget_bytes.max(1),
        }
    }

    /// Captures one raw worker line (stdout/stderr). Empty or invalid
    /// correlation lines are skipped; oversized/noisy input is truncated,
    /// redacted and retained bounded.
    pub fn capture_line(
        &mut self,
        source: PythonLogSource,
        raw: &str,
        correlation: &PythonLogCorrelation,
        timestamp_ms: u64,
    ) -> Option<PythonLogRecord> {
        if raw.trim().is_empty() || !correlation.validate() {
            return None;
        }
        let level = infer_level(source, raw);
        let redacted = PythonLogRedactor::redact(raw);
        if redacted.contains(REDACTED) {
            self.redacted_count += 1;
        }
        let message: String = redacted.chars().take(MAX_MESSAGE_CHARS).collect();
        let record = PythonLogRecord {
            sequence: self.next_sequence,
            timestamp_ms,
            level,
            source,
            correlation: correlation.clone(),
            message,
        };
        self.next_sequence += 1;
        self.push_bounded(record)
    }

    /// Captures a lifecycle event description as an Info record.
    pub fn capture_lifecycle(
        &mut self,
        description: &str,
        correlation: &PythonLogCorrelation,
        timestamp_ms: u64,
    ) -> Option<PythonLogRecord> {
        self.capture_line(
            PythonLogSource::Lifecycle,
            description,
            correlation,
            timestamp_ms,
        )
        .map(|mut record| {
            record.level = PythonLogLevel::Info;
            record
        })
    }

    fn push_bounded(&mut self, record: PythonLogRecord) -> Option<PythonLogRecord> {
        let cost = record.message.len();
        while self.records.len() >= self.capacity || self.total_bytes + cost > self.budget_bytes {
            let Some(evicted) = self.records.pop_front() else {
                // Empty buffer and the record alone exceeds the budget:
                // fail closed by dropping the record entirely.
                if self.total_bytes + cost > self.budget_bytes {
                    self.dropped += 1;
                    return None;
                }
                break;
            };
            self.total_bytes = self.total_bytes.saturating_sub(evicted.message.len());
            self.dropped += 1;
        }
        self.total_bytes += cost;
        self.records.push_back(record.clone());
        Some(record)
    }

    /// Records visible to one project only (isolation by correlation).
    pub fn records_for_project(&self, project_id: &str) -> Vec<&PythonLogRecord> {
        self.records
            .iter()
            .filter(|record| record.correlation.project_id == project_id)
            .collect()
    }

    /// Drains the buffer (rotation) and returns the rotated records.
    pub fn rotate(&mut self) -> Vec<PythonLogRecord> {
        let drained: Vec<PythonLogRecord> = self.records.drain(..).collect();
        self.total_bytes = 0;
        self.rotations += 1;
        drained
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }

    pub fn redacted_count(&self) -> u64 {
        self.redacted_count
    }

    pub fn rotations(&self) -> u64 {
        self.rotations
    }
}

/// Deterministic level inference: explicit prefixes win, stderr defaults to
/// Warn, everything else to Info.
fn infer_level(source: PythonLogSource, raw: &str) -> PythonLogLevel {
    let trimmed = raw.trim_start();
    for (prefix, level) in [
        ("ERROR", PythonLogLevel::Error),
        ("WARN", PythonLogLevel::Warn),
        ("DEBUG", PythonLogLevel::Debug),
        ("INFO", PythonLogLevel::Info),
    ] {
        if trimmed
            .get(..prefix.len())
            .map(|head| head.eq_ignore_ascii_case(prefix))
            .unwrap_or(false)
        {
            return level;
        }
    }
    match source {
        PythonLogSource::Stderr => PythonLogLevel::Warn,
        _ => PythonLogLevel::Info,
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn correlation() -> PythonLogCorrelation {
        PythonLogCorrelation {
            worker_id: "worker-1".into(),
            project_id: "proj-1".into(),
            session_id: "sess-1".into(),
            task_id: None,
            trace_id: "trace-1".into(),
        }
    }

    #[test]
    fn redactor_strips_control_chars_and_paths() {
        let redacted = PythonLogRedactor::redact("\u{1b}[31merror\u{1b}[0m ../etc/passwd");
        assert!(
            !redacted.contains('\u{1b}'),
            "ANSI must be stripped: {redacted}"
        );
        assert!(
            !redacted.contains(".."),
            "traversal neutralized: {redacted}"
        );
        assert!(redacted.contains("error"));
    }

    #[test]
    fn redactor_masks_secret_like_values() {
        for sample in [
            "token=abc123 payload",
            "Authorization: Bearer eyJhb",
            "api_key: supersecret",
            "password=hunter2",
        ] {
            let redacted = PythonLogRedactor::redact(sample);
            assert!(!redacted.contains("abc123"), "{sample} -> {redacted}");
            assert!(!redacted.contains("eyJhb"), "{sample} -> {redacted}");
            assert!(!redacted.contains("supersecret"), "{sample} -> {redacted}");
            assert!(!redacted.contains("hunter2"), "{sample} -> {redacted}");
            assert!(redacted.contains(REDACTED), "{sample} -> {redacted}");
        }
    }

    #[test]
    fn capture_is_bounded_correlated_and_rotates() {
        let mut capture = PythonLogCapture::with_limits(4, 1_024);
        let correlation = correlation();
        for index in 0..6 {
            capture
                .capture_line(
                    PythonLogSource::Stdout,
                    &format!("line {index}"),
                    &correlation,
                    index,
                )
                .expect("captured");
        }
        assert_eq!(capture.len(), 4, "capacity bound");
        assert_eq!(capture.dropped(), 2, "oldest dropped with counter");
        assert_eq!(capture.records_for_project("proj-1").len(), 4);
        assert!(
            capture.records_for_project("proj-other").is_empty(),
            "project isolation"
        );

        let rotated = capture.rotate();
        assert_eq!(rotated.len(), 4);
        assert!(capture.is_empty());
        assert_eq!(capture.rotations(), 1);
        assert_eq!(capture.total_bytes(), 0);
    }
}
