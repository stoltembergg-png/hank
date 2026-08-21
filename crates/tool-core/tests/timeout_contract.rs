use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tool_core::{ToolExecutionStatus, ToolExecutionWindow, ToolTerminalState};

#[test]
// @spec:AC-665
fn deadline_is_monotonic_and_remaining_is_bounded() {
    let window = ToolExecutionWindow::new(Duration::from_millis(50)).unwrap();
    let remaining = window.remaining();

    assert!(remaining <= Duration::from_millis(50));
    assert!(remaining > Duration::ZERO);
    assert!(window.deadline() > std::time::Instant::now());
    assert_eq!(window.poll(), ToolExecutionStatus::Active);
}

#[test]
// @spec:AC-666
fn cancellation_wins_race_and_terminal_transition_is_idempotent() {
    let window = ToolExecutionWindow::new(Duration::from_millis(1)).unwrap();
    thread::sleep(Duration::from_millis(10));
    window.cancel();

    assert_eq!(
        window.poll(),
        ToolExecutionStatus::Terminal(ToolTerminalState::Cancelled)
    );
    assert_eq!(window.finish(), ToolTerminalState::Cancelled);
    assert_eq!(
        window.poll(),
        ToolExecutionStatus::Terminal(ToolTerminalState::Cancelled)
    );
}

#[test]
// @spec:AC-667
fn timeout_claims_one_terminal_state_without_duplicate_effects() {
    let window = ToolExecutionWindow::new(Duration::from_millis(1)).unwrap();
    thread::sleep(Duration::from_millis(10));

    assert_eq!(
        window.poll(),
        ToolExecutionStatus::Terminal(ToolTerminalState::TimedOut)
    );
    assert_eq!(window.finish(), ToolTerminalState::TimedOut);
    assert!(!window.is_active());
}

#[test]
// @spec:AC-668
fn shared_cancellation_flag_is_observed_by_the_execution_window() {
    let flag = Arc::new(AtomicBool::new(false));
    let window = ToolExecutionWindow::with_cancellation(
        Duration::from_secs(1),
        tool_core::ToolCancellation::from_flag(Arc::clone(&flag)),
    )
    .unwrap();
    flag.store(true, Ordering::SeqCst);

    assert_eq!(
        window.poll(),
        ToolExecutionStatus::Terminal(ToolTerminalState::Cancelled)
    );
}
