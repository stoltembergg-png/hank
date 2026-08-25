use workflow_core::delay::{DelayError, DelayPlan, DelayState};

// @spec:AC-1004
#[test]
fn fake_clock_releases_only_at_deadline() {
    let mut delay = DelayPlan::new(100, 50, 1_000).unwrap();
    assert_eq!(delay.state(), DelayState::Waiting);
    assert_eq!(delay.poll(149), DelayState::Waiting);
    assert_eq!(delay.poll(150), DelayState::Ready);
    assert_eq!(delay.poll(999), DelayState::Ready);
}

// @spec:AC-1005
#[test]
fn zero_is_ready_excess_fails_and_cancel_is_terminal() {
    assert_eq!(
        DelayPlan::new(100, 0, 1_000).unwrap().state(),
        DelayState::Ready
    );
    assert!(matches!(
        DelayPlan::new(100, 1_001, 1_000),
        Err(DelayError::DurationExceeded)
    ));
    let mut delay = DelayPlan::new(100, 50, 1_000).unwrap();
    assert!(delay.cancel());
    assert!(!delay.cancel());
    assert_eq!(delay.state(), DelayState::Cancelled);
    assert_eq!(delay.poll(1_000), DelayState::Cancelled);
}

// @spec:AC-1006
#[test]
fn pause_resume_preserves_remaining_duration() {
    let mut delay = DelayPlan::new(100, 100, 1_000).unwrap();
    assert_eq!(delay.pause(140), DelayState::Paused);
    assert_eq!(delay.resume(900), Ok(DelayState::Waiting));
    assert_eq!(delay.poll(959), DelayState::Waiting);
    assert_eq!(delay.poll(960), DelayState::Ready);
}
