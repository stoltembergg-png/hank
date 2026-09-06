//! Deterministic, bounded agent-loop contract model.
use ring::digest::{digest, SHA256};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_TURNS: u32 = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LoopStep {
    Tool {
        key: String,
        cost: u32,
        allowed: bool,
    },
    Delegate {
        target: String,
        depth: u32,
    },
    Retry {
        cost: u32,
    },
    Finish,
    MissingEvent,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopStatus {
    Success,
    PermissionDenied,
    DuplicateReplay,
    CycleDenied,
    DepthDenied,
    BudgetExceeded,
    Cancelled,
    StaleEvent,
    TurnLimit,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopEvent {
    pub turn: u32,
    pub kind: String,
    pub key: String,
    pub cost: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopReport {
    pub status: LoopStatus,
    pub turns: u32,
    pub spent: u32,
    pub events: Vec<LoopEvent>,
    pub trace_digest: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopPolicy {
    pub max_turns: u32,
    pub max_depth: u32,
    pub budget: u32,
    pub cancelled: bool,
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LoopError {
    #[error("invalid loop policy")]
    InvalidPolicy,
}

pub fn run_loop(steps: &[LoopStep], policy: LoopPolicy) -> Result<LoopReport, LoopError> {
    if policy.max_turns == 0 || policy.max_turns > MAX_TURNS || policy.max_depth == 0 {
        return Err(LoopError::InvalidPolicy);
    }
    let mut events = Vec::new();
    let mut spent: u32 = 0;
    let mut seen = Vec::new();
    let mut status = LoopStatus::TurnLimit;
    if policy.cancelled {
        status = LoopStatus::Cancelled;
    }
    for (index, step) in steps.iter().enumerate().take(policy.max_turns as usize) {
        if policy.cancelled {
            break;
        }
        let turn = index as u32 + 1;
        match step {
            LoopStep::Finish => {
                status = LoopStatus::Success;
                events.push(LoopEvent {
                    turn,
                    kind: "finish".into(),
                    key: "finish".into(),
                    cost: 0,
                });
                break;
            }
            LoopStep::Tool { key, cost, allowed } => {
                if !allowed {
                    status = LoopStatus::PermissionDenied;
                    break;
                }
                if seen.iter().any(|value: &String| value == key) {
                    status = LoopStatus::DuplicateReplay;
                    events.push(LoopEvent {
                        turn,
                        kind: "duplicate".into(),
                        key: key.clone(),
                        cost: 0,
                    });
                    continue;
                }
                if spent.saturating_add(*cost) > policy.budget {
                    status = LoopStatus::BudgetExceeded;
                    break;
                }
                seen.push(key.clone());
                spent += *cost;
                events.push(LoopEvent {
                    turn,
                    kind: "tool".into(),
                    key: key.clone(),
                    cost: *cost,
                });
            }
            LoopStep::Delegate { target, depth } => {
                if *depth > policy.max_depth {
                    status = LoopStatus::DepthDenied;
                    break;
                }
                if seen.iter().any(|value| value == target) {
                    status = LoopStatus::CycleDenied;
                    break;
                }
                seen.push(target.clone());
                events.push(LoopEvent {
                    turn,
                    kind: "delegate".into(),
                    key: target.clone(),
                    cost: 0,
                });
            }
            LoopStep::Retry { cost } => {
                if spent.saturating_add(*cost) > policy.budget {
                    status = LoopStatus::BudgetExceeded;
                    break;
                }
                spent += *cost;
                events.push(LoopEvent {
                    turn,
                    kind: "retry".into(),
                    key: "retry".into(),
                    cost: *cost,
                });
            }
            LoopStep::MissingEvent => {
                status = LoopStatus::StaleEvent;
                break;
            }
        }
    }
    let encoded = serde_json::to_vec(&events).map_err(|_| LoopError::InvalidPolicy)?;
    let trace_digest = digest(&SHA256, &encoded)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(LoopReport {
        status,
        turns: events.last().map_or(0, |event| event.turn),
        spent,
        events,
        trace_digest,
    })
}
