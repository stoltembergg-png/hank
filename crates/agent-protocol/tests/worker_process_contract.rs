use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use agent_protocol::ids::{ProjectId, RequestId, SessionId, TraceId};
use agent_protocol::worker::{WorkerContext, WorkerSession, WORKER_PROTOCOL_SCHEMA_VERSION};
use serde_json::{json, Value};

const HANDSHAKE: &str = r#"{"kind":"handshake","schema_version":1,"protocol_version":1,"worker_id":"runtime-sidecar","capabilities":[{"resource":"tool","action":"execute"}]}"#;

/// Worker harness: spawns the Python sidecar and frames NDJSON lines.
struct WorkerHarness {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    transcript: String,
}

impl WorkerHarness {
    fn spawn(extra_env: &[(&str, &str)], args: &[&str]) -> Option<Self> {
        let python = find_python()?;
        let mut command = Command::new(python);
        command
            .arg("worker.py")
            .args(args)
            .current_dir(runtime_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
        })
    }

    fn send(&mut self, line: &str) {
        self.transcript.push_str(line);
        self.transcript.push('\n');
        let stdin = self.stdin.as_mut().expect("worker stdin");
        stdin
            .write_all(line.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .expect("worker stdin must stay writable");
    }

    fn send_json(&mut self, value: &Value) {
        self.send(&serde_json::to_string(value).expect("message must serialize"));
    }

    fn read_line(&mut self) -> String {
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("worker stdout must stay readable");
        assert!(!line.trim().is_empty(), "worker closed stdout unexpectedly");
        self.transcript.push_str(&line);
        line.trim_end().to_string()
    }

    fn shutdown_and_wait(&mut self) -> std::process::ExitStatus {
        self.send(r#"{"kind":"shutdown","schema_version":1,"reason":"user"}"#);
        let ack = self.read_line();
        assert!(
            ack.contains(r#""kind":"shutdown_ack""#),
            "expected ack, got: {ack}"
        );
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

fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../python/runtime")
        .canonicalize()
        .expect("python/runtime directory must exist")
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
fn handshake_health_and_shutdown_lifecycle_is_deterministic() {
    let Some(mut worker) = WorkerHarness::spawn(&[("HANK_TEST_SENTINEL", "must-not-leak")], &[])
    else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send(HANDSHAKE);
    let accepted = worker.read_line();
    assert!(
        accepted.contains(r#""kind":"handshake_accepted""#),
        "got: {accepted}"
    );
    assert!(
        accepted.contains(r#""protocol_version":1"#),
        "got: {accepted}"
    );

    worker.send(r#"{"kind":"health","schema_version":1}"#);
    let report = worker.read_line();
    assert!(
        report.contains(r#""kind":"health_report""#),
        "got: {report}"
    );
    assert!(report.contains(r#""status":"healthy""#), "got: {report}");

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

    worker.send(r#"{"kind":"health","schema_version":1}"#);
    let rejected = worker.read_line();
    assert!(rejected.contains(r#""kind":"error""#), "got: {rejected}");
    assert!(
        rejected.contains(r#""code":"invalid_state""#),
        "got: {rejected}"
    );
    let status = worker.wait();
    assert_eq!(
        status.code(),
        Some(1),
        "pre-handshake violation must exit 1"
    );

    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        return;
    };
    worker.send(
        r#"{"kind":"handshake","schema_version":9,"protocol_version":1,"worker_id":"runtime-sidecar","capabilities":[]}"#,
    );
    let rejected = worker.read_line();
    assert!(
        rejected.contains(r#""code":"unsupported_version""#),
        "got: {rejected}"
    );
    let status = worker.wait();
    assert_eq!(status.code(), Some(1), "rejected handshake must exit 1");
}

#[test]
// @spec:AC-685
fn unknown_and_malformed_lines_keep_the_channel_bounded() {
    let Some(mut worker) = WorkerHarness::spawn(&[], &[]) else {
        eprintln!("python3 not available; worker process contract skipped (optional runtime)");
        return;
    };

    worker.send(HANDSHAKE);
    assert!(worker.read_line().contains("handshake_accepted"));

    worker.send("not json at all");
    let malformed = worker.read_line();
    assert!(
        malformed.contains(r#""code":"invalid_message""#),
        "got: {malformed}"
    );

    worker.send(r#"{"kind":"teleport","schema_version":1}"#);
    let unknown = worker.read_line();
    assert!(
        unknown.contains(r#""code":"invalid_message""#),
        "got: {unknown}"
    );

    worker.send(r#"{"kind":"health","schema_version":1}"#);
    assert!(worker.read_line().contains(r#""status":"healthy""#));

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

    worker.send(HANDSHAKE);
    assert!(worker.read_line().contains("handshake_accepted"));

    let request = json!({
        "kind": "request",
        "schema_version": WORKER_PROTOCOL_SCHEMA_VERSION,
        "request_id": "req-00000000-0000-4000-8000-000000000404",
        "context": context_json(),
        "capability": {"resource": "tool", "action": "execute"},
        "payload": {"instruction": "print('pwned')", "secret": "must-not-echo"},
    });
    worker.send_json(&request);
    let response = worker.read_line();
    assert!(
        response.contains(r#""result":"not_supported""#),
        "got: {response}"
    );
    assert!(
        !response.contains("must-not-echo") && !response.contains("pwned"),
        "response must not echo the payload: {response}"
    );

    // The reply must deserialize into the protocol contract and validate.
    let parsed: agent_protocol::worker::WorkerMessage =
        serde_json::from_str(&response).expect("response must deserialize as WorkerMessage");
    parsed
        .validate()
        .expect("response must satisfy the contract");

    let status = worker.shutdown_and_wait();
    assert!(status.success());
}

#[test]
// @spec:AC-687
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

    let source = std::fs::read_to_string(runtime.join("worker.py")).expect("worker.py must exist");
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
            "worker source must stay free of '{forbidden}' (no env/fs/exec access)"
        );
    }

    // The core contract validator keeps working with no Python process at all.
    let mut session = WorkerSession::new();
    let context = WorkerContext {
        project_id: ProjectId::parse("proj-00000000-0000-4000-8000-000000000401").unwrap(),
        session_id: SessionId::parse("sess-00000000-0000-4000-8000-000000000402").unwrap(),
        task_id: None,
        trace_id: TraceId::new(),
    };
    session
        .accept(agent_protocol::worker::WorkerMessage::Handshake {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            worker_id: "runtime-sidecar".to_string(),
            protocol_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            capabilities: vec![agent_protocol::capability::Capability::new(
                agent_protocol::capability::Resource::Tool,
                agent_protocol::capability::Action::Execute,
            )],
        })
        .expect("handshake registers without any Python runtime");
    session
        .accept(agent_protocol::worker::WorkerMessage::HandshakeAccepted {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            worker_id: "runtime-sidecar".to_string(),
            protocol_version: WORKER_PROTOCOL_SCHEMA_VERSION,
        })
        .expect("handshake acceptance is protocol state only");
    let request_id = RequestId::new();
    session
        .accept(agent_protocol::worker::WorkerMessage::Request {
            schema_version: WORKER_PROTOCOL_SCHEMA_VERSION,
            request_id,
            context,
            capability: agent_protocol::capability::Capability::new(
                agent_protocol::capability::Resource::Tool,
                agent_protocol::capability::Action::Execute,
            ),
            payload: json!({"task": "noop"}),
        })
        .expect("request registers without Python");
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
