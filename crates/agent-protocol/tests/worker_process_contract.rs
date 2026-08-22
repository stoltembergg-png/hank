use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

/// Serializes worker spawns across test threads: the sidecar lifecycle is
/// deterministic and process startup stays bounded on loaded runners.
fn spawn_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Worker harness speaking JSON-RPC 2.0 with Content-Length framing.
struct WorkerHarness {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    transcript: String,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl WorkerHarness {
    fn spawn(extra_env: &[(&str, &str)], args: &[&str]) -> Option<Self> {
        let guard = spawn_lock();
        let python = find_python()?;
        let mut command = Command::new(python);
        command
            .arg("-m")
            .arg("runtime")
            .args(args)
            .current_dir(python_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Inherited so a worker crash surfaces its traceback in test logs
            // instead of dying in an unread pipe.
            .stderr(Stdio::inherit());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("worker process must spawn");
        let stdin = child.stdin.take().expect("worker stdin");
        let stdout = child.stdout.take().expect("worker stdout");
        Some(Self {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
            transcript: String::new(),
            _guard: guard,
        })
    }

    fn send_raw(&mut self, bytes: &[u8]) {
        self.transcript.push_str(&String::from_utf8_lossy(bytes));
        self.stdin
            .as_mut()
            .expect("worker stdin")
            .write_all(bytes)
            .and_then(|_| self.stdin.as_mut().expect("worker stdin").flush())
            .expect("worker stdin must stay writable");
    }

    fn send_message(&mut self, message: &Value) {
        let payload = serde_json::to_string(message).expect("message must serialize");
        let frame = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
        self.send_raw(frame.as_bytes());
    }

    /// Reads one framed message; ``None`` when the worker closes stdout.
    fn read_message(&mut self) -> Option<Value> {
        let mut length: Option<usize> = None;
        loop {
            let mut line = String::new();
            let read = self.stdout.read_line(&mut line).expect("stdout readable");
            if read == 0 {
                return None;
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                if length.is_some() {
                    break;
                }
                continue;
            }
            if let Some((name, value)) = trimmed.split_once(':') {
                if name.trim().eq_ignore_ascii_case("content-length") {
                    length = Some(value.trim().parse().expect("content-length numeric"));
                }
            }
        }
        let length = length.expect("content-length header before separator");
        let mut payload = vec![0u8; length];
        self.stdout
            .read_exact(&mut payload)
            .expect("payload complete");
        self.transcript.push_str(&String::from_utf8_lossy(&payload));
        let message: Value = serde_json::from_slice(&payload).expect("payload is JSON");
        Some(message)
    }

    fn shutdown_and_wait(&mut self) -> std::process::ExitStatus {
        self.send_message(&json!({
            "jsonrpc": "2.0", "id": 9_999, "method": "shutdown", "params": {}
        }));
        let ack = self.read_message().expect("shutdown ack");
        assert_eq!(ack["id"], json!(9_999), "ack must correlate the id");
        assert_eq!(ack["result"]["kind"], json!("shutdown_ack"));
        self.wait()
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        drop(self.stdin.take());
        self.child.wait().expect("worker must exit")
    }
}

fn find_python() -> Option<String> {
    for candidate in ["python3", "python"] {
        let probe = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if probe.map(|status| status.success()).unwrap_or(false) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn python_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../python")
        .canonicalize()
        .expect("python directory must exist")
}

fn runtime_dir() -> PathBuf {
    python_dir().join("runtime")
}

fn handshake_params() -> Value {
    json!({
        "schema_version": 1,
        "protocol_version": 1,
        "worker_id": "runtime-sidecar",
        "capabilities": [{"resource": "tool", "action": "execute"}]
    })
}

fn context_json() -> Value {
    json!({
        "project_id": "proj-00000000-0000-4000-8000-000000000401",
        "session_id": "sess-00000000-0000-4000-8000-000000000402",
        "task_id": null,
        "trace_id": "trace-00000000-0000-4000-8000-000000000403",
    })
}

#[test]
// @spec:AC-683
// @spec:AC-691
fn handshake_health_and_shutdown_lifecycle_is_deterministic() {
    let Some(mut worker) = WorkerHarness::spawn(&[("HANK_TEST_SENTINEL", "must-not-leak")], &[])
    else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send_message(
        &json!({"jsonrpc": "2.0", "id": 1, "method": "handshake", "params": handshake_params()}),
    );
    let accepted = worker.read_message().expect("handshake reply");
    assert_eq!(accepted["id"], json!(1), "reply must correlate the id");
    assert_eq!(accepted["result"]["kind"], json!("handshake_accepted"));
    assert_eq!(accepted["result"]["protocol_version"], json!(1));

    worker.send_message(&json!({"jsonrpc": "2.0", "id": 2, "method": "health", "params": {}}));
    let report = worker.read_message().expect("health reply");
    assert_eq!(report["result"]["kind"], json!("health_report"));
    assert_eq!(report["result"]["status"], json!("healthy"));

    let status = worker.shutdown_and_wait();
    assert!(status.success(), "clean shutdown must exit 0, got {status}");

    assert!(
        !worker.transcript.contains("must-not-leak"),
        "worker transcript must not echo environment: {}",
        worker.transcript
    );
}

#[test]
// @spec:AC-684
fn invalid_handshake_and_premature_messages_exit_fail_closed() {
    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send_message(&json!({"jsonrpc": "2.0", "id": 1, "method": "health", "params": {}}));
    assert!(
        worker.read_message().is_none(),
        "pre-handshake violation closes stdout"
    );
    let status = worker.wait();
    assert_eq!(
        status.code(),
        Some(1),
        "pre-handshake violation must exit 1"
    );
    drop(worker);

    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        return;
    };
    let mut bad = handshake_params();
    bad["protocol_version"] = json!(9);
    worker.send_message(&json!({"jsonrpc": "2.0", "id": 1, "method": "handshake", "params": bad}));
    let rejected = worker.read_message().expect("bounded error reply");
    assert_eq!(rejected["id"], json!(1));
    assert_eq!(rejected["error"]["code"], json!(-32700), "got: {rejected}");
    assert!(
        worker.read_message().is_none(),
        "rejected handshake closes the channel"
    );
    let status = worker.wait();
    assert_eq!(status.code(), Some(1), "rejected handshake must exit 1");
}

#[test]
// @spec:AC-685
// @spec:AC-692
fn malformed_frames_and_unknown_methods_keep_the_channel_bounded() {
    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send_message(
        &json!({"jsonrpc": "2.0", "id": 1, "method": "handshake", "params": handshake_params()}),
    );
    assert_eq!(
        worker.read_message().expect("handshake")["result"]["kind"],
        json!("handshake_accepted")
    );

    // Malformed frame: invalid JSON payload with a valid header.
    worker.send_raw(b"Content-Length: 9\r\n\r\nnot json!");
    // Unknown method: bounded JSON-RPC error correlated by id.
    worker.send_message(&json!({"jsonrpc": "2.0", "id": 2, "method": "teleport", "params": {}}));
    let unknown = worker
        .read_message()
        .expect("error reply for unknown method");
    assert_eq!(unknown["id"], json!(2));
    assert_eq!(unknown["error"]["code"], json!(-32601), "got: {unknown}");
    assert!(
        !unknown.to_string().contains("teleport"),
        "error must not echo the method payload: {unknown}"
    );

    // Channel survives both violations.
    worker.send_message(&json!({"jsonrpc": "2.0", "id": 3, "method": "health", "params": {}}));
    assert_eq!(
        worker.read_message().expect("health")["result"]["status"],
        json!("healthy")
    );

    let status = worker.shutdown_and_wait();
    assert!(status.success(), "channel must stay usable, got {status}");
}

#[test]
// @spec:AC-686
fn requests_never_execute_payload_and_reply_not_supported() {
    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send_message(
        &json!({"jsonrpc": "2.0", "id": 1, "method": "handshake", "params": handshake_params()}),
    );
    assert!(worker.read_message().expect("handshake")["result"].is_object());

    worker.send_message(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "request",
        "params": {
            "schema_version": 1,
            "request_id": "req-00000000-0000-4000-8000-000000000404",
            "context": context_json(),
            "capability": {"resource": "tool", "action": "execute"},
            "payload": {"instruction": "print('pwned')", "secret": "must-not-echo"}
        }
    }));
    let response = worker.read_message().expect("request reply");
    assert_eq!(
        response["result"]["result"],
        json!("not_supported"),
        "got: {response}"
    );
    assert!(
        !response.to_string().contains("must-not-echo") && !response.to_string().contains("pwned"),
        "response must not echo the payload: {response}"
    );

    // The inner result must deserialize into the worker protocol contract.
    let parsed: agent_protocol::worker::WorkerMessage =
        serde_json::from_value(response["result"].clone()).expect("result must be a WorkerMessage");
    parsed.validate().expect("result must satisfy the contract");

    let status = worker.shutdown_and_wait();
    assert!(status.success());
}

#[test]
// @spec:AC-691
// @spec:AC-692
fn duplicate_request_ids_are_rejected_with_bounded_errors() {
    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send_message(
        &json!({"jsonrpc": "2.0", "id": 7, "method": "handshake", "params": handshake_params()}),
    );
    assert!(worker.read_message().expect("handshake")["result"].is_object());

    worker.send_message(&json!({"jsonrpc": "2.0", "id": 7, "method": "health", "params": {}}));
    let rejected = worker.read_message().expect("duplicate id error");
    assert_eq!(rejected["id"], json!(7));
    assert_eq!(rejected["error"]["code"], json!(-32011), "got: {rejected}");

    let status = worker.shutdown_and_wait();
    assert!(status.success());
}

#[test]
// @spec:AC-687
// @spec:AC-693
fn worker_ships_no_dependencies_and_core_stays_python_free() {
    let runtime = runtime_dir();
    for forbidden in [
        "requirements.txt",
        "pyproject.toml",
        "Pipfile",
        "setup.py",
        "poetry.lock",
    ] {
        assert!(
            !runtime.join(forbidden).exists(),
            "minimal worker must not declare dependencies: {forbidden}"
        );
    }

    let entries = std::fs::read_dir(&runtime).expect("runtime dir readable");
    for entry in entries {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("py") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("worker source readable");
        for forbidden in [
            "environ",
            "subprocess",
            "eval(",
            "exec(",
            "system(",
            "open(",
            "__import__",
        ] {
            assert!(
                !source.contains(forbidden),
                "{} must stay free of '{forbidden}' (no env/fs/exec access)",
                path.display()
            );
        }
    }

    // The transport allowlist and correlation keep working with no Python.
    use agent_protocol::json_rpc::{
        error_code, is_known_method, CompletionState, JsonRpcCorrelation, JsonRpcMessage,
        JsonRpcParseError,
    };
    assert!(is_known_method("handshake"));
    assert!(!is_known_method("teleport"));

    let message = JsonRpcMessage::request(1, "health", json!({"schema_version": 1}));
    message
        .validate()
        .expect("in-process message validates without Python");
    assert_eq!(
        JsonRpcMessage::request(1, "teleport", json!({})).validate(),
        Err(JsonRpcParseError::InvalidMessage),
        "unknown methods must fail closed without any worker"
    );
    assert_eq!(error_code::METHOD_NOT_FOUND, -32_601);

    let mut correlation = JsonRpcCorrelation::new();
    correlation.register(1, 0, 1_000).expect("registers");
    assert_eq!(correlation.complete(1, 500), CompletionState::Completed);
    assert_eq!(correlation.complete(1, 500), CompletionState::UnknownId);
}

#[test]
// @spec:AC-684
fn worker_rejects_forbidden_arguments() {
    let Some(mut worker) = WorkerHarness::spawn(&[], &["--run-anything"]) else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };
    let status = worker.wait();
    assert_eq!(
        status.code(),
        Some(2),
        "arguments must be rejected, got {status}"
    );
}
