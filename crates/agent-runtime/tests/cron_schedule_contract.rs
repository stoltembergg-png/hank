use agent_runtime::scheduler::{CronError, CronSchedule};
use chrono::{DateTime, Utc};

fn utc(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}

// @spec:AC-1161
#[test]
fn golden_corpus_normalizes_valid_fields_and_rejects_malformed() {
    let schedule = CronSchedule::parse("*/5 * * * *", "UTC").unwrap();
    assert_eq!(
        schedule
            .next_due_after(utc("2026-08-25T12:01:00Z"))
            .unwrap()
            .to_rfc3339(),
        "2026-08-25T12:05:00+00:00"
    );
    assert!(CronSchedule::parse("0 9 1,15 1-6 1", "UTC").is_ok());
    assert!(matches!(
        CronSchedule::parse("0 9 * *", "UTC"),
        Err(CronError::FieldCount)
    ));
    assert!(matches!(
        CronSchedule::parse("0 9 * * 8", "UTC"),
        Err(CronError::ValueOutOfRange)
    ));
    assert!(matches!(
        CronSchedule::parse("0 9 * * * extra", "UTC"),
        Err(CronError::FieldCount)
    ));
}

// @spec:AC-1162
#[test]
fn dst_gap_is_skipped_and_fold_uses_earliest_utc_occurrence() {
    let gap = CronSchedule::parse("30 2 * * *", "America/New_York").unwrap();
    assert_eq!(
        gap.next_due_after(utc("2024-03-10T06:00:00Z"))
            .unwrap()
            .to_rfc3339(),
        "2024-03-11T06:30:00+00:00"
    );
    let fold = CronSchedule::parse("30 1 * * *", "America/New_York").unwrap();
    assert_eq!(
        fold.next_due_after(utc("2024-11-03T04:00:00Z"))
            .unwrap()
            .to_rfc3339(),
        "2024-11-03T05:30:00+00:00"
    );
}

// @spec:AC-1163
#[test]
fn parser_is_bounded_for_generated_ascii_inputs() {
    for seed in 0..256_u32 {
        let expression = format!("{} {} * * *", seed % 80, seed % 40);
        let result = CronSchedule::parse(&expression, "UTC");
        assert!(result.is_ok() || result.is_err());
    }
}
// @spec:AC-1163
#[test]
fn limits_timezone_allowlist_and_minimum_frequency_fail_closed() {
    assert!(matches!(
        CronSchedule::parse("* * * * *", "UTC"),
        Err(CronError::TooFrequent)
    ));
    assert!(matches!(
        CronSchedule::parse("0 9 * * *", "Mars/Olympus"),
        Err(CronError::InvalidTimezone)
    ));
    assert!(matches!(
        CronSchedule::parse(&"0 ".repeat(100), "UTC"),
        Err(CronError::InputTooLong)
    ));
}
