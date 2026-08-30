//! Pure, bounded last-known-good rollback decision artifact.
use thiserror::Error;
const MAX_TEXT: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackTrigger {
    Crash,
    Regression,
    PermissionRevoked,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackRequest {
    pub active_version: String,
    pub previous_version: String,
    pub policy_revision: String,
    pub trigger: RollbackTrigger,
}
impl RollbackRequest {
    pub fn new(
        active: &str,
        previous: &str,
        policy: &str,
        trigger: RollbackTrigger,
    ) -> Result<Self, RollbackError> {
        if active.is_empty() || active.len() > MAX_TEXT {
            return Err(RollbackError::InvalidIdentity);
        }
        if previous.is_empty() {
            return Err(RollbackError::NoLastKnownGood);
        }
        if policy.is_empty() || policy.len() > MAX_TEXT {
            return Err(RollbackError::InvalidIdentity);
        }
        Ok(Self {
            active_version: active.into(),
            previous_version: previous.into(),
            policy_revision: policy.into(),
            trigger,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackStatus {
    Recovered,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RollbackError {
    #[error("rollback identity is invalid")]
    InvalidIdentity,
    #[error("no last-known-good version exists")]
    NoLastKnownGood,
    #[error("rollback policy revision does not match")]
    PolicyMismatch,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rollback {
    status: RollbackStatus,
    previous_version: String,
    rollback_id: String,
    quarantined: bool,
}
impl Rollback {
    pub fn execute(request: RollbackRequest) -> Result<Self, RollbackError> {
        if request.policy_revision != "policy-1" {
            return Err(RollbackError::PolicyMismatch);
        }
        let material = format!(
            "{}|{}|{}|{:?}",
            request.active_version,
            request.previous_version,
            request.policy_revision,
            request.trigger
        );
        Ok(Self {
            status: RollbackStatus::Recovered,
            previous_version: request.previous_version,
            rollback_id: digest(&material),
            quarantined: true,
        })
    }
    pub fn status(&self) -> RollbackStatus {
        self.status
    }
    pub fn previous_version(&self) -> &str {
        &self.previous_version
    }
    pub fn rollback_id(&self) -> &str {
        &self.rollback_id
    }
    pub fn quarantined(&self) -> bool {
        self.quarantined
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}
fn digest(v: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in v.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
