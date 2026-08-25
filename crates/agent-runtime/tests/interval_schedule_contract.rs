use agent_runtime::scheduler::{IntervalSchedule, ScheduleError};

// @spec:AC-1141
#[test]
fn fake_clock_is_deterministic_and_does_not_drift_after_restart() {
    let schedule = IntervalSchedule::new(1_000, 60).unwrap();
    assert_eq!(schedule.next_due(1_000, true).unwrap(), Some(61_000));
    assert_eq!(schedule.next_due(61_001, true).unwrap(), Some(121_000));
    let restarted = IntervalSchedule::new(1_000, 60).unwrap();
    assert_eq!(restarted.next_due(61_001, true).unwrap(), Some(121_000));
}

// @spec:AC-1142
#[test]
fn invalid_frequency_and_arithmetic_overflow_fail_closed() {
    assert!(matches!(
        IntervalSchedule::new(1_000, 0),
        Err(ScheduleError::TooFrequent)
    ));
    assert!(matches!(
        IntervalSchedule::new(1_000, 59),
        Err(ScheduleError::TooFrequent)
    ));
    assert!(matches!(
        IntervalSchedule::new(1_000, u64::MAX),
        Err(ScheduleError::TooLong)
    ));
    let schedule = IntervalSchedule::new(u64::MAX - 10, 60).unwrap();
    assert!(matches!(
        schedule.next_due(u64::MAX - 10, true),
        Err(ScheduleError::Overflow)
    ));
}

// @spec:AC-1143
#[test]
fn disabled_schedule_has_no_next_due() {
    let schedule = IntervalSchedule::new(1_000, 60).unwrap();
    assert_eq!(schedule.next_due(1_000, false).unwrap(), None);
}
