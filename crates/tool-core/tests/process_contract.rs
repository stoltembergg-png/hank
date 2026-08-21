#![cfg(unix)]

use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tool_core::{PermissionDecision, ProcessError, ProcessSpec, run_process};

fn fixture_program() -> PathBuf {
    PathBuf::from("/bin/printf")
}
fn sleep_program() -> PathBuf {
    PathBuf::from("/bin/sleep")
}

fn spec(program: PathBuf) -> ProcessSpec {
    let cwd = std::env::current_dir().unwrap();
    ProcessSpec {
        project_id: ProjectId::new(),
        program: program.clone(),
        args: vec!["hello".into()],
        cwd: cwd.clone(),
        env: BTreeMap::new(),
        allowed_programs: BTreeSet::from([program]),
        allowed_roots: vec![cwd],
        permission: PermissionDecision::Allowed { reason: "test" },
        timeout: Duration::from_secs(2),
        max_output_bytes: 32,
        trace_id: TraceId::new(),
    }
}

#[test]
// @spec:AC-646
fn allowlisted_structured_process_runs_without_shell() {
    let spec = spec(fixture_program());
    let result = run_process(&spec, Arc::new(AtomicBool::new(false))).unwrap();
    assert_eq!(result.stdout, "hello");
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.timed_out && !result.cancelled);
}

#[test]
// @spec:AC-647
fn rejects_shell_program_permission_cwd_and_sensitive_environment() {
    let mut shell = spec(PathBuf::from("/bin/sh"));
    shell.allowed_programs.insert(PathBuf::from("/bin/sh"));
    assert_eq!(shell.validate(), Err(ProcessError::ShellNotAllowed));
    let mut denied = spec(fixture_program());
    denied.permission = PermissionDecision::NeedsConfirmation { scope: "x".into() };
    assert_eq!(denied.validate(), Err(ProcessError::PermissionDenied));
    let mut env = spec(fixture_program());
    env.env.insert("API_TOKEN".into(), "secret".into());
    assert_eq!(env.validate(), Err(ProcessError::EnvironmentNotAllowed));
}

#[test]
// @spec:AC-648
fn timeout_and_cancellation_kill_child_and_bound_output() {
    let mut timeout = spec(sleep_program());
    timeout.args = vec!["2".into()];
    timeout.timeout = Duration::from_millis(20);
    let result = run_process(&timeout, Arc::new(AtomicBool::new(false))).unwrap();
    assert!(result.timed_out);
    let mut cancel = spec(sleep_program());
    cancel.args = vec!["2".into()];
    let flag = Arc::new(AtomicBool::new(false));
    let trigger = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        trigger.store(true, Ordering::SeqCst);
    });
    let result = run_process(&cancel, flag).unwrap();
    assert!(result.cancelled);
}
