use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedAction {
    Skip,
    CatchUp,
    Coalesce,
    Pause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissedPolicy {
    pub action: MissedAction,
    pub interval_ms: u64,
    pub lateness_window_ms: u64,
    pub catch_up_cap: u32,
    pub policy_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissedDecision {
    pub occurrence_at_ms: u64,
    pub action: MissedAction,
    pub reason: String,
    pub coalesce_key: Option<String>,
    pub policy_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PolicyError {
    #[error("missed-run policy bounds are invalid")]
    InvalidBounds,
}

pub fn evaluate(
    policy: &MissedPolicy,
    due_at_ms: u64,
    now_ms: u64,
    enabled: bool,
) -> Result<Vec<MissedDecision>, PolicyError> {
    if policy.interval_ms == 0
        || policy.catch_up_cap == 0
        || policy.catch_up_cap > 64
        || policy.policy_version.is_empty()
        || policy.policy_version.len() > 32
    {
        return Err(PolicyError::InvalidBounds);
    }
    if !enabled || now_ms < due_at_ms {
        return Ok(Vec::new());
    }
    let elapsed = now_ms - due_at_ms;
    let occurrences = elapsed / policy.interval_ms + 1;
    if elapsed > policy.lateness_window_ms {
        return Ok(vec![decision(
            policy,
            due_at_ms,
            MissedAction::Skip,
            "outside_window",
            None,
        )]);
    }
    match policy.action {
        MissedAction::Coalesce => Ok(vec![decision(
            policy,
            due_at_ms,
            MissedAction::Coalesce,
            "coalesced",
            Some(format!("missed:{}", due_at_ms)),
        )]),
        MissedAction::Pause => Ok(vec![decision(
            policy,
            due_at_ms,
            MissedAction::Pause,
            "policy_pause",
            None,
        )]),
        action => Ok((0..occurrences.min(u64::from(policy.catch_up_cap)))
            .map(|index| {
                decision(
                    policy,
                    due_at_ms + index * policy.interval_ms,
                    action,
                    if action == MissedAction::CatchUp {
                        "catch_up"
                    } else {
                        "skip"
                    },
                    None,
                )
            })
            .collect()),
    }
}

fn decision(
    policy: &MissedPolicy,
    occurrence_at_ms: u64,
    action: MissedAction,
    reason: &str,
    coalesce_key: Option<String>,
) -> MissedDecision {
    MissedDecision {
        occurrence_at_ms,
        action,
        reason: reason.into(),
        coalesce_key,
        policy_version: policy.policy_version.clone(),
    }
}
