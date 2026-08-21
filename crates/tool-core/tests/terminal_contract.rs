#![cfg(unix)]

use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;
use tool_core::{PermissionDecision, ProcessSpec, TerminalAdapter, TerminalError, TerminalRequest};

fn request(key: &str) -> TerminalRequest {
    let cwd = std::env::current_dir().unwrap();
    let program = PathBuf::from("/bin/printf");
    TerminalRequest {
        process: ProcessSpec {
            project_id: ProjectId::new(),
            program: program.clone(),
            args: vec!["terminal".into()],
            cwd: cwd.clone(),
            env: BTreeMap::new(),
            allowed_programs: BTreeSet::from([program]),
            allowed_roots: vec![cwd],
            permission: PermissionDecision::Allowed { reason: "test" },
            timeout: Duration::from_secs(2),
            max_output_bytes: 64,
            trace_id: TraceId::new(),
        },
        operation_key: key.into(),
        max_rounds: 1,
    }
}

#[test]
// @spec:AC-649
fn terminal_delegates_to_process_primitive_and_returns_round_one() {
    let adapter = TerminalAdapter::new();
    let result = adapter
        .execute(request("terminal-1"), Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(result.process.stdout, "terminal");
    assert_eq!(result.round, 1);
}

#[test]
// @spec:AC-649
fn terminal_rejects_missing_invalid_round_and_duplicate_operation() {
    let adapter = TerminalAdapter::new();
    let mut missing = request("");
    assert!(matches!(
        adapter.execute(missing.clone(), Arc::new(AtomicBool::new(false))),
        Err(TerminalError::MissingOperationKey)
    ));
    missing.operation_key = "bad-round".into();
    missing.max_rounds = 0;
    assert!(matches!(
        adapter.execute(missing, Arc::new(AtomicBool::new(false))),
        Err(TerminalError::InvalidRoundLimit)
    ));
    adapter
        .execute(request("same"), Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert!(matches!(
        adapter.execute(request("same"), Arc::new(AtomicBool::new(false))),
        Err(TerminalError::DuplicateOperation)
    ));
}

#[test]
// @spec:AC-650
fn terminal_preserves_process_permission_and_cancellation_failures() {
    let adapter = TerminalAdapter::new();
    let mut denied = request("denied");
    denied.process.permission = PermissionDecision::NeedsConfirmation {
        scope: "terminal".into(),
    };
    assert!(matches!(
        adapter.execute(denied, Arc::new(AtomicBool::new(false))),
        Err(TerminalError::Process(_))
    ));
}
