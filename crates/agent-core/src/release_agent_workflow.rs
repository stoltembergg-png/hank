//! Pure release-candidate preparation; publishing and signing remain protected boundaries.

use thiserror::Error;

const MAX_VALUE: usize = 256;
const MAX_REASONS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseContext {
    repository: String,
    commit: String,
    tree: String,
    policy: String,
}
impl ReleaseContext {
    pub fn new(
        repository: &str,
        commit: &str,
        tree: &str,
        policy: &str,
    ) -> Result<Self, ReleaseError> {
        validate_values(&[repository, commit, tree, policy])?;
        Ok(Self {
            repository: repository.into(),
            commit: commit.into(),
            tree: tree.into(),
            policy: policy.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEvidence {
    name: String,
    digest: String,
    commit: String,
    tree: String,
    policy: String,
}
impl ArtifactEvidence {
    pub fn new(
        name: &str,
        digest: &str,
        commit: &str,
        tree: &str,
        policy: &str,
    ) -> Result<Self, ReleaseError> {
        validate_values(&[name, digest, commit, tree, policy])?;
        Ok(Self {
            name: name.into(),
            digest: digest.into(),
            commit: commit.into(),
            tree: tree.into(),
            policy: policy.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiEvidence {
    run: String,
    commit: String,
    tree: String,
    policy: String,
}
impl CiEvidence {
    pub fn pass(run: &str, commit: &str, tree: &str, policy: &str) -> Result<Self, ReleaseError> {
        validate_values(&[run, commit, tree, policy])?;
        Ok(Self {
            run: run.into(),
            commit: commit.into(),
            tree: tree.into(),
            policy: policy.into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInput {
    pub context: ReleaseContext,
    pub artifact: ArtifactEvidence,
    pub ci: CiEvidence,
    pub signing_present: bool,
    pub provenance_present: bool,
}
impl ReleaseInput {
    pub fn new(
        context: ReleaseContext,
        artifact: ArtifactEvidence,
        ci: CiEvidence,
        signing_present: bool,
        provenance_present: bool,
    ) -> Result<Self, ReleaseError> {
        Ok(Self {
            context,
            artifact,
            ci,
            signing_present,
            provenance_present,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseState {
    Draft,
    NoGo,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReleaseError {
    #[error("invalid release value")]
    InvalidValue,
    #[error("release evidence is stale")]
    StaleEvidence,
    #[error("too many release reasons")]
    BoundsExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCandidate {
    state: ReleaseState,
    reasons: Vec<String>,
    fingerprint: String,
}
impl ReleaseCandidate {
    pub fn state(&self) -> ReleaseState {
        self.state
    }
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn can_publish(&self) -> bool {
        false
    }
}

pub struct ReleaseAgentWorkflow;
impl ReleaseAgentWorkflow {
    pub fn prepare(input: &ReleaseInput) -> Result<ReleaseCandidate, ReleaseError> {
        let mut reasons = Vec::new();
        if input.artifact.commit != input.context.commit
            || input.artifact.tree != input.context.tree
            || input.artifact.policy != input.context.policy
        {
            reasons.push("artifact identity mismatch".into());
        }
        if input.ci.commit != input.context.commit
            || input.ci.tree != input.context.tree
            || input.ci.policy != input.context.policy
        {
            reasons.push("CI identity mismatch".into());
        }
        if !input.artifact.digest.starts_with("sha256:") || input.artifact.digest.len() <= 7 {
            reasons.push("checksum invalid".into());
        }
        if !input.signing_present {
            reasons.push("signing evidence missing; protected environment required".into());
        }
        if !input.provenance_present {
            reasons.push("provenance evidence missing; protected environment required".into());
        }
        if reasons.len() > MAX_REASONS {
            return Err(ReleaseError::BoundsExceeded);
        }
        let state = if reasons.is_empty() {
            ReleaseState::Draft
        } else {
            ReleaseState::NoGo
        };
        let fingerprint = digest(&format!(
            "{}:{}:{}:{}:{}:{}",
            input.context.repository,
            input.context.commit,
            input.context.tree,
            input.context.policy,
            input.artifact.name,
            input.artifact.digest
        ));
        Ok(ReleaseCandidate {
            state,
            reasons,
            fingerprint,
        })
    }
}

fn validate_values(values: &[&str]) -> Result<(), ReleaseError> {
    if values.iter().any(|value| {
        value.is_empty() || value.len() > MAX_VALUE || value.chars().any(char::is_control)
    }) {
        Err(ReleaseError::InvalidValue)
    } else {
        Ok(())
    }
}
fn digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
