use agent_core::ids::{AgentId, SessionId};
use agent_core::session::{Message, MessageProvenance, MessageRole, MessageStatus};
use agent_runtime::cancellation::{
    cancel_turn, CancellationError, CancellationOutcome, CancellationRegistry,
};
use agent_runtime::execution::{Execution, ExecutionEvent, ExecutionState};

fn turn() -> (Execution, Message) {
    let session_id = SessionId::new();
    (
        Execution::new("exec-1", session_id, AgentId::new(), "corr-1", 1, 100, 1000).unwrap(),
        Message::new(
            session_id,
            MessageRole::Assistant,
            MessageProvenance::Provider,
            0,
            1,
            "",
        )
        .unwrap(),
    )
}

#[test]
fn registry_registers_cancels_idempotently_and_unregisters() {
    let registry = CancellationRegistry::new(1).unwrap();
    let handle = registry.register("exec-1").unwrap();
    assert!(!handle.is_cancelled());
    assert_eq!(
        registry.cancel("exec-1").unwrap(),
        CancellationOutcome::Applied
    );
    assert!(handle.is_cancelled());
    assert_eq!(
        registry.cancel("exec-1").unwrap(),
        CancellationOutcome::AlreadyCancelled
    );
    registry.unregister("exec-1").unwrap();
    assert!(matches!(
        registry.cancel("exec-1"),
        Err(CancellationError::UnknownExecution)
    ));
}

#[test]
fn registry_capacity_and_identity_are_bounded() {
    let registry = CancellationRegistry::new(1).unwrap();
    registry.register("one").unwrap();
    assert!(matches!(
        registry.register("two"),
        Err(CancellationError::Capacity)
    ));
    assert!(matches!(
        registry.register(""),
        Err(CancellationError::InvalidIdentity)
    ));
}

#[test]
fn cancel_turn_transitions_execution_and_message_once() {
    let (mut execution, mut message) = turn();
    execution.apply(ExecutionEvent::Start).unwrap();
    let token = provider_core::CancellationToken::new();
    assert_eq!(
        cancel_turn(&mut execution, &mut message, &token).unwrap(),
        CancellationOutcome::Applied
    );
    assert!(token.is_cancelled());
    assert_eq!(execution.state(), ExecutionState::Cancelled);
    assert_eq!(message.status, MessageStatus::Cancelled);
    assert_eq!(
        cancel_turn(&mut execution, &mut message, &token).unwrap(),
        CancellationOutcome::AlreadyCancelled
    );
}

#[test]
fn completion_wins_race_and_cancellation_does_not_overwrite_terminal() {
    let (mut execution, mut message) = turn();
    execution.apply(ExecutionEvent::Start).unwrap();
    execution.apply(ExecutionEvent::Completed).unwrap();
    message.start_stream().unwrap();
    message.complete().unwrap();
    let token = provider_core::CancellationToken::new();
    assert_eq!(
        cancel_turn(&mut execution, &mut message, &token).unwrap(),
        CancellationOutcome::AlreadyTerminal
    );
    assert_eq!(execution.state(), ExecutionState::Completed);
    assert_eq!(message.status, MessageStatus::Complete);
}

#[test]
fn cancellation_debug_contains_no_secret_or_raw_payload() {
    let registry = CancellationRegistry::new(2).unwrap();
    registry.register("exec-2").unwrap();
    registry.cancel("exec-2").unwrap();
    let debug = format!("{registry:?}");
    assert!(!debug.contains("api_key"));
    assert!(!debug.contains("secret"));
}

#[test]
fn concurrent_registry_operations_are_thread_safe() {
    let registry = std::sync::Arc::new(CancellationRegistry::new(16).unwrap());
    let mut workers = Vec::new();
    for index in 0..8 {
        let registry = registry.clone();
        workers.push(std::thread::spawn(move || {
            let id = format!("exec-{index}");
            registry.register(&id).unwrap();
            registry.cancel(&id).unwrap();
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(registry.len(), 8);
}
