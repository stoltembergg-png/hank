//! Pure, deterministic branch mutation policy boundary.
//!
//! This crate does not execute Git, access GitHub, read credentials, or mutate
//! policy state. Callers provide bounded identity and policy-revision data and
//! receive an explicit allow/deny result.

pub mod mcp_permission;
pub mod plugin_permission;

pub mod security_profile;
pub use security_profile::{
    SecurityAgentProfile, SecurityEvidence, SecurityEvidenceStatus, SecurityFinding,
    SecurityFindingClassification, SecurityFindingSeverity, SecurityFindingStatus, SecurityHandoff,
    SecurityHandoffStatus, SecurityPermit, SecurityProfileError, SecurityReport,
    SecurityReportStatus, SecurityThreatCase, SecurityThreatManifest,
};

use std::collections::BTreeSet;
use thiserror::Error;

pub const MAX_BRANCH_POLICY_ID_LEN: usize = 128;
pub const MAX_BRANCH_POLICY_REVISION_LEN: usize = 128;
pub const MAX_BRANCH_POLICY_PREFIX_LEN: usize = 64;
pub const MAX_BRANCH_NAME_LEN: usize = 256;
pub const MAX_PROTECTED_BRANCHES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchMutation {
    LocalCommit,
    Push,
    ForcePush,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchPolicyRequest {
    pub project_id: String,
    pub repository_id: String,
    pub task_id: String,
    pub owner_id: String,
    pub actor_id: String,
    pub branch: String,
    pub base_branch: String,
    pub policy_revision: String,
    pub operation: BranchMutation,
}

impl BranchPolicyRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: impl Into<String>,
        repository_id: impl Into<String>,
        task_id: impl Into<String>,
        owner_id: impl Into<String>,
        actor_id: impl Into<String>,
        branch: impl Into<String>,
        base_branch: impl Into<String>,
        policy_revision: impl Into<String>,
        operation: BranchMutation,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            repository_id: repository_id.into(),
            task_id: task_id.into(),
            owner_id: owner_id.into(),
            actor_id: actor_id.into(),
            branch: branch.into(),
            base_branch: base_branch.into(),
            policy_revision: policy_revision.into(),
            operation,
        }
    }

    fn validate(&self) -> Result<(), BranchPolicyError> {
        validate_request_id(&self.project_id)?;
        validate_request_id(&self.repository_id)?;
        validate_request_id(&self.task_id)?;
        validate_request_id(&self.owner_id)?;
        validate_request_id(&self.actor_id)?;
        validate_request_revision(&self.policy_revision)?;
        validate_branch(&self.branch)?;
        validate_branch(&self.base_branch)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchPolicy {
    project_id: String,
    repository_id: String,
    policy_revision: String,
    branch_prefix: String,
    protected_branches: BTreeSet<String>,
}

impl BranchPolicy {
    pub fn new(
        project_id: impl Into<String>,
        repository_id: impl Into<String>,
        policy_revision: impl Into<String>,
        branch_prefix: impl Into<String>,
        protected_branches: Vec<String>,
    ) -> Result<Self, BranchPolicyError> {
        let project_id = project_id.into();
        let repository_id = repository_id.into();
        let policy_revision = policy_revision.into();
        let branch_prefix = branch_prefix.into();
        validate_policy_id(&project_id)?;
        validate_policy_id(&repository_id)?;
        validate_policy_revision(&policy_revision)?;
        validate_prefix(&branch_prefix)?;
        if protected_branches.len() > MAX_PROTECTED_BRANCHES {
            return Err(BranchPolicyError::InvalidPolicy);
        }

        let mut protected = BTreeSet::new();
        for branch in protected_branches {
            validate_policy_branch(&branch)?;
            if !protected.insert(branch) {
                return Err(BranchPolicyError::InvalidPolicy);
            }
        }

        Ok(Self {
            project_id,
            repository_id,
            policy_revision,
            branch_prefix,
            protected_branches: protected,
        })
    }

    pub fn evaluate(
        &self,
        request: &BranchPolicyRequest,
    ) -> Result<BranchDecision, BranchPolicyError> {
        request.validate()?;
        if request.policy_revision != self.policy_revision {
            return Err(BranchPolicyError::PolicyRevisionMismatch);
        }
        if request.project_id != self.project_id || request.repository_id != self.repository_id {
            return Err(BranchPolicyError::ScopeMismatch);
        }
        if self.protected_branches.contains(&request.branch) {
            return Err(BranchPolicyError::ProtectedBranch);
        }
        if request.actor_id != request.owner_id {
            return Err(BranchPolicyError::ActorNotOwner);
        }
        if request.branch != format!("{}{}", self.branch_prefix, request.task_id) {
            return Err(BranchPolicyError::BranchTaskMismatch);
        }

        match request.operation {
            BranchMutation::LocalCommit | BranchMutation::Push => Ok(BranchDecision::Allowed {
                policy_revision: self.policy_revision.clone(),
                operation: request.operation,
            }),
            BranchMutation::ForcePush => Err(BranchPolicyError::ForcePushDenied),
            BranchMutation::Merge => Err(BranchPolicyError::MergeDenied),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchDecision {
    Allowed {
        policy_revision: String,
        operation: BranchMutation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BranchPolicyError {
    #[error("branch policy is invalid")]
    InvalidPolicy,
    #[error("branch policy request is invalid")]
    InvalidRequest,
    #[error("branch policy scope does not match project or repository")]
    ScopeMismatch,
    #[error("branch actor is not the task owner")]
    ActorNotOwner,
    #[error("branch is protected")]
    ProtectedBranch,
    #[error("branch does not match the task binding")]
    BranchTaskMismatch,
    #[error("branch policy revision does not match")]
    PolicyRevisionMismatch,
    #[error("force push is denied by policy")]
    ForcePushDenied,
    #[error("merge is denied by policy")]
    MergeDenied,
}

fn validate_policy_id(value: &str) -> Result<(), BranchPolicyError> {
    if value.trim().is_empty()
        || value.len() > MAX_BRANCH_POLICY_ID_LEN
        || value.chars().any(char::is_control)
    {
        return Err(BranchPolicyError::InvalidPolicy);
    }
    Ok(())
}

fn validate_request_id(value: &str) -> Result<(), BranchPolicyError> {
    if value.trim().is_empty()
        || value.len() > MAX_BRANCH_POLICY_ID_LEN
        || value.chars().any(char::is_control)
    {
        return Err(BranchPolicyError::InvalidRequest);
    }
    Ok(())
}

fn validate_policy_revision(value: &str) -> Result<(), BranchPolicyError> {
    if value.trim().is_empty()
        || value.len() > MAX_BRANCH_POLICY_REVISION_LEN
        || value.chars().any(char::is_control)
    {
        return Err(BranchPolicyError::InvalidPolicy);
    }
    Ok(())
}

fn validate_request_revision(value: &str) -> Result<(), BranchPolicyError> {
    if value.trim().is_empty()
        || value.len() > MAX_BRANCH_POLICY_REVISION_LEN
        || value.chars().any(char::is_control)
    {
        return Err(BranchPolicyError::InvalidRequest);
    }
    Ok(())
}

fn validate_prefix(value: &str) -> Result<(), BranchPolicyError> {
    if value.is_empty()
        || value.len() > MAX_BRANCH_POLICY_PREFIX_LEN
        || value.chars().any(char::is_control)
        || value.chars().any(char::is_whitespace)
        || !value.ends_with('/')
        || value.contains("..")
    {
        return Err(BranchPolicyError::InvalidPolicy);
    }
    Ok(())
}

fn validate_policy_branch(value: &str) -> Result<(), BranchPolicyError> {
    validate_branch_with_error(value, BranchPolicyError::InvalidPolicy)
}

fn validate_branch(value: &str) -> Result<(), BranchPolicyError> {
    validate_branch_with_error(value, BranchPolicyError::InvalidRequest)
}

fn validate_branch_with_error(
    value: &str,
    error: BranchPolicyError,
) -> Result<(), BranchPolicyError> {
    if value.trim().is_empty()
        || value.len() > MAX_BRANCH_NAME_LEN
        || value.chars().any(char::is_control)
        || value.chars().any(char::is_whitespace)
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.ends_with('.')
        || value.contains("..")
        || value.contains(['~', '^', ':', '?', '*', '[', '\\'])
    {
        return Err(error);
    }
    Ok(())
}
