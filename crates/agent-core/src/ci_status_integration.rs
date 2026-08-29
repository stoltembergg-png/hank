//! Pure, bounded CI status classification bound to one event identity.

use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequiredCheck {
    BuildRust,
    Quality,
    Security,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiEvent {
    PullRequest,
    MergeGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiCheckStatus {
    Pass,
    Fail,
    Skipped,
    Cancelled,
    Timeout,
    Missing,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    Pass,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiPolicyState {
    Enforced,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CiStatusError {
    #[error("invalid CI identity value")]
    InvalidValue,
    #[error("CI evidence is stale or has wrong identity")]
    StaleEvidence,
    #[error("CI input exceeds bounds")]
    BoundsExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiContext {
    pub repository: String,
    pub pull_request: String,
    pub event: CiEvent,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy: String,
}

impl CiContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo: &str,
        pr: &str,
        event: CiEvent,
        head: &str,
        tree: &str,
        policy: &str,
    ) -> Result<Self, CiStatusError> {
        if [repo, pr, head, tree, policy]
            .iter()
            .any(|v| v.is_empty() || v.len() > 256 || v.chars().any(char::is_control))
        {
            return Err(CiStatusError::InvalidValue);
        }
        Ok(Self {
            repository: repo.into(),
            pull_request: pr.into(),
            event,
            head_sha: head.into(),
            tree_sha: tree.into(),
            policy: policy.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiCheckResult {
    pub name: RequiredCheck,
    pub status: CiCheckStatus,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy: String,
    pub run_id: String,
    pub digest: String,
}
impl CiCheckResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: RequiredCheck,
        status: CiCheckStatus,
        head: &str,
        tree: &str,
        policy: &str,
        run: &str,
        digest: &str,
    ) -> Result<Self, CiStatusError> {
        if [head, tree, policy, run, digest]
            .iter()
            .any(|v| v.is_empty() || v.len() > 256 || v.chars().any(char::is_control))
        {
            return Err(CiStatusError::InvalidValue);
        }
        Ok(Self {
            name,
            status,
            head_sha: head.into(),
            tree_sha: tree.into(),
            policy: policy.into(),
            run_id: run.into(),
            digest: digest.into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CiInput {
    pub context: CiContext,
    pub checks: Vec<CiCheckResult>,
}
impl CiInput {
    pub fn new(context: CiContext, checks: Vec<CiCheckResult>) -> Result<Self, CiStatusError> {
        if checks.len() > 32 {
            return Err(CiStatusError::BoundsExceeded);
        }
        Ok(Self { context, checks })
    }
}

#[derive(Debug, Clone)]
pub struct CiReport {
    state: CiState,
    policy_state: CiPolicyState,
    fingerprint: String,
}
impl CiReport {
    pub fn state(&self) -> CiState {
        self.state
    }
    pub fn policy_state(&self) -> CiPolicyState {
        self.policy_state
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn can_merge(&self) -> bool {
        false
    }
}

pub struct CiStatusIntegration;
impl CiStatusIntegration {
    pub fn evaluate(input: &CiInput) -> Result<CiReport, CiStatusError> {
        let mut names = BTreeSet::new();
        let mut unknown = false;
        for check in &input.checks {
            if check.head_sha != input.context.head_sha || check.tree_sha != input.context.tree_sha
            {
                return Err(CiStatusError::StaleEvidence);
            }
            let expected_policy =
                input.context.policy == "not-applicable" || check.policy == input.context.policy;
            if !expected_policy {
                return Err(CiStatusError::StaleEvidence);
            }
            if !names.insert(check.name) {
                unknown = true;
            }
            match check.status {
                CiCheckStatus::Pass => {}
                CiCheckStatus::Fail
                | CiCheckStatus::Skipped
                | CiCheckStatus::Cancelled
                | CiCheckStatus::Timeout
                | CiCheckStatus::Missing
                | CiCheckStatus::Malformed => unknown = true,
            }
        }
        for required in [
            RequiredCheck::BuildRust,
            RequiredCheck::Quality,
            RequiredCheck::Security,
        ] {
            if !names.contains(&required) {
                unknown = true;
            }
        }
        let policy_state = if input.context.policy == "not-applicable" {
            CiPolicyState::NotApplicable
        } else {
            CiPolicyState::Enforced
        };
        let mut text = format!(
            "{}:{}:{}:{:?}:{}",
            input.context.repository,
            input.context.pull_request,
            input.context.head_sha,
            input.context.event,
            input.context.policy
        );
        for check in &input.checks {
            text.push_str(&format!(
                "{:?}:{:?}:{}",
                check.name, check.status, check.digest
            ));
        }
        Ok(CiReport {
            state: if unknown {
                CiState::Unknown
            } else {
                CiState::Pass
            },
            policy_state,
            fingerprint: stable_digest(&text),
        })
    }
}

fn stable_digest(value: &str) -> String {
    let mut state = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}
