//! Contrato puro e bounded para gerar propostas de PR draft.
//!
//! Este módulo não acessa Git/GitHub, filesystem, rede, providers, processos ou
//! credenciais. Adapters externos podem consumir o plano declarativo.

use crate::task_mapping::{MappingState, TaskWorkspaceMapping};
use crate::{DomainError, ProjectId, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const PR_GENERATION_SCHEMA_VERSION: u32 = 1;
const MAX_TEXT: usize = 512;
const MAX_ID: usize = 128;
const MAX_PATH: usize = 512;
const MAX_ITEMS: usize = 64;
const MAX_CHECKS: usize = 16;
const MAX_BODY_BYTES: usize = 16_384;
const REQUIRED_CHECKS: [PrGenerationCheckKind; 4] = [
    PrGenerationCheckKind::Tests,
    PrGenerationCheckKind::Security,
    PrGenerationCheckKind::Scope,
    PrGenerationCheckKind::Evidence,
];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PrGenerationError {
    #[error("PR generation profile is invalid: {0}")]
    InvalidProfile(String),
    #[error("PR generation mapping is inactive")]
    MappingInactive,
    #[error("PR generation handoff identity is stale or mismatched")]
    IdentityMismatch,
    #[error("PR generation handoff is invalid: {0}")]
    InvalidHandoff(String),
    #[error("PR generation evidence is incomplete or failed")]
    EvidenceIncomplete,
    #[error("PR generation publication or merge is denied")]
    AuthorityDenied,
}

impl From<PrGenerationError> for DomainError {
    fn from(value: PrGenerationError) -> Self {
        DomainError::Validation(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrGenerationCheckKind {
    Tests,
    Security,
    Scope,
    Evidence,
}

impl PrGenerationCheckKind {
    pub fn required() -> &'static [Self; 4] {
        &REQUIRED_CHECKS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrGenerationCheckStatus {
    Passed,
    Failed,
    Skipped,
    NoRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrGenerationCheck {
    pub kind: PrGenerationCheckKind,
    pub status: PrGenerationCheckStatus,
    pub digest: String,
}

impl PrGenerationCheck {
    pub fn required() -> &'static [PrGenerationCheckKind; 4] {
        &REQUIRED_CHECKS
    }

    pub fn new(
        kind: PrGenerationCheckKind,
        status: PrGenerationCheckStatus,
        digest: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            status,
            digest: digest.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrGenerationPlanKind {
    CreateDraft,
    UpdateDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrGenerationProfile {
    pub schema_version: u32,
    pub policy_revision: String,
    pub max_body_bytes: usize,
    pub allow_publish: bool,
    pub allow_merge: bool,
}

impl Default for PrGenerationProfile {
    fn default() -> Self {
        Self {
            schema_version: PR_GENERATION_SCHEMA_VERSION,
            policy_revision: "pr-generation-v1".into(),
            max_body_bytes: MAX_BODY_BYTES,
            allow_publish: false,
            allow_merge: false,
        }
    }
}

impl PrGenerationProfile {
    pub fn validate(
        &self,
        handoff: &PrGenerationHandoff,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<(), PrGenerationError> {
        self.validate_profile()?;
        self.validate_handoff(handoff, mapping)
    }

    fn validate_profile(&self) -> Result<(), PrGenerationError> {
        if self.schema_version != PR_GENERATION_SCHEMA_VERSION
            || !bounded_text(&self.policy_revision, MAX_ID)
            || self.max_body_bytes == 0
            || self.max_body_bytes > MAX_BODY_BYTES
            || self.allow_publish
            || self.allow_merge
        {
            return Err(PrGenerationError::InvalidProfile(
                "schema, policy, bounds or authority are invalid".into(),
            ));
        }
        Ok(())
    }

    pub fn plan(
        &self,
        handoff: &PrGenerationHandoff,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<PrGenerationPlan, PrGenerationError> {
        self.validate_profile()?;
        self.validate_handoff(handoff, mapping)?;
        if handoff
            .checks
            .iter()
            .any(|check| check.status != PrGenerationCheckStatus::Passed)
        {
            return Err(PrGenerationError::EvidenceIncomplete);
        }
        let fingerprint = handoff.fingerprint();
        let common = (
            handoff.project_id,
            handoff.task_id,
            handoff.repository_id.clone(),
            handoff.branch.clone(),
            handoff.head_sha.clone(),
            handoff.tree_sha.clone(),
            handoff.idempotency_key.clone(),
            fingerprint.clone(),
        );
        Ok(match handoff.existing_draft_id.clone() {
            Some(draft_id) => PrGenerationPlan::UpdateDraft {
                project_id: common.0,
                task_id: common.1,
                repository_id: common.2,
                branch: common.3,
                head_sha: common.4,
                tree_sha: common.5,
                idempotency_key: common.6,
                draft_id,
                fingerprint: common.7,
            },
            None => PrGenerationPlan::CreateDraft {
                project_id: common.0,
                task_id: common.1,
                repository_id: common.2,
                branch: common.3,
                head_sha: common.4,
                tree_sha: common.5,
                idempotency_key: common.6,
                fingerprint: common.7,
            },
        })
    }

    pub fn validate_handoff(
        &self,
        handoff: &PrGenerationHandoff,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<(), PrGenerationError> {
        self.validate_profile()?;
        if mapping.state() != MappingState::Active {
            return Err(PrGenerationError::MappingInactive);
        }
        if handoff.project_id != mapping.project_id()
            || handoff.task_id != mapping.task_id()
            || handoff.repository_id != mapping.repository_id()
            || handoff.worktree_id != mapping.worktree_id()
            || handoff.branch != mapping.branch()
            || handoff.policy_revision != self.policy_revision
        {
            return Err(PrGenerationError::IdentityMismatch);
        }
        handoff.validate_shape(self.max_body_bytes)?;
        validate_checks(&handoff.checks)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrGenerationHandoff {
    pub schema_version: u32,
    pub policy_revision: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub idempotency_key: String,
    pub existing_draft_id: Option<String>,
    pub objective: String,
    pub scope: String,
    pub non_scope: String,
    pub tests: String,
    pub acceptance_criteria: String,
    pub risks: String,
    pub rollback: String,
    pub documentation: String,
    pub changed_paths: Vec<String>,
    pub checks: Vec<PrGenerationCheck>,
}

impl PrGenerationHandoff {
    #[allow(clippy::too_many_arguments)]
    pub fn proposed(
        profile: &PrGenerationProfile,
        mapping: &TaskWorkspaceMapping,
        head_sha: String,
        tree_sha: String,
        idempotency_key: impl Into<String>,
        objective: impl Into<String>,
        scope: impl Into<String>,
        non_scope: impl Into<String>,
        tests: impl Into<String>,
        acceptance_criteria: impl Into<String>,
        risks: impl Into<String>,
        rollback: impl Into<String>,
        documentation: impl Into<String>,
        changed_paths: Vec<String>,
        checks: Vec<PrGenerationCheck>,
    ) -> Result<Self, PrGenerationError> {
        profile.validate_profile()?;
        if mapping.state() != MappingState::Active {
            return Err(PrGenerationError::MappingInactive);
        }
        let value = Self {
            schema_version: PR_GENERATION_SCHEMA_VERSION,
            policy_revision: profile.policy_revision.clone(),
            project_id: mapping.project_id(),
            task_id: mapping.task_id(),
            repository_id: mapping.repository_id().into(),
            worktree_id: mapping.worktree_id().into(),
            branch: mapping.branch().into(),
            head_sha,
            tree_sha,
            idempotency_key: idempotency_key.into(),
            existing_draft_id: None,
            objective: objective.into(),
            scope: scope.into(),
            non_scope: non_scope.into(),
            tests: tests.into(),
            acceptance_criteria: acceptance_criteria.into(),
            risks: risks.into(),
            rollback: rollback.into(),
            documentation: documentation.into(),
            changed_paths,
            checks,
        };
        profile.validate_handoff(&value, mapping)?;
        Ok(value)
    }

    pub fn with_existing_draft_id(mut self, value: impl Into<String>) -> Self {
        self.existing_draft_id = Some(value.into());
        self
    }

    pub fn fingerprint(&self) -> String {
        let mut value = String::new();
        for field in [
            &self.project_id.to_string(),
            &self.task_id.to_string(),
            &self.repository_id,
            &self.worktree_id,
            &self.branch,
            &self.head_sha,
            &self.tree_sha,
            &self.idempotency_key,
        ] {
            value.push_str(field);
            value.push('\n');
        }
        stable_digest(&value)
    }

    fn validate_shape(&self, body_limit: usize) -> Result<(), PrGenerationError> {
        if self.schema_version != PR_GENERATION_SCHEMA_VERSION
            || !bounded_text(&self.policy_revision, MAX_ID)
            || !bounded_text(&self.repository_id, MAX_ID)
            || !bounded_text(&self.worktree_id, MAX_ID)
            || !bounded_text(&self.branch, MAX_PATH)
            || !valid_sha(&self.head_sha, 40)
            || !valid_sha(&self.tree_sha, 64)
            || !bounded_text(&self.idempotency_key, MAX_ID)
            || self
                .existing_draft_id
                .as_deref()
                .is_some_and(|v| !bounded_text(v, MAX_ID))
        {
            return Err(PrGenerationError::InvalidHandoff(
                "identity, SHA, path or idempotency key is invalid".into(),
            ));
        }
        for value in [
            &self.objective,
            &self.scope,
            &self.non_scope,
            &self.tests,
            &self.acceptance_criteria,
            &self.risks,
            &self.rollback,
            &self.documentation,
        ] {
            if !bounded_text(value, MAX_TEXT) {
                return Err(PrGenerationError::InvalidHandoff(
                    "PR metadata is empty, oversized or untrusted".into(),
                ));
            }
        }
        if self.changed_paths.is_empty() || self.changed_paths.len() > MAX_ITEMS {
            return Err(PrGenerationError::InvalidHandoff(
                "changed paths are outside bounded limits".into(),
            ));
        }
        for path in &self.changed_paths {
            if !safe_relative_path(path) {
                return Err(PrGenerationError::InvalidHandoff(
                    "changed path is outside the worktree".into(),
                ));
            }
        }
        let total = self.objective.len()
            + self.scope.len()
            + self.non_scope.len()
            + self.tests.len()
            + self.acceptance_criteria.len()
            + self.risks.len()
            + self.rollback.len()
            + self.documentation.len();
        if total > body_limit {
            return Err(PrGenerationError::InvalidHandoff(
                "PR body exceeds bounded limit".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum PrGenerationPlan {
    CreateDraft {
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: String,
        branch: String,
        head_sha: String,
        tree_sha: String,
        idempotency_key: String,
        fingerprint: String,
    },
    UpdateDraft {
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: String,
        branch: String,
        head_sha: String,
        tree_sha: String,
        idempotency_key: String,
        draft_id: String,
        fingerprint: String,
    },
}

impl PrGenerationPlan {
    pub fn can_publish(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }
}

fn validate_checks(checks: &[PrGenerationCheck]) -> Result<(), PrGenerationError> {
    if checks.is_empty() || checks.len() > MAX_CHECKS {
        return Err(PrGenerationError::EvidenceIncomplete);
    }
    let mut seen = HashSet::new();
    for required in REQUIRED_CHECKS {
        let Some(check) = checks.iter().find(|check| check.kind == required) else {
            return Err(PrGenerationError::EvidenceIncomplete);
        };
        if !seen.insert(check.kind)
            || !valid_sha(&check.digest, 64)
            || check.digest.chars().all(|c| c == '0')
        {
            return Err(PrGenerationError::EvidenceIncomplete);
        }
    }
    Ok(())
}

fn bounded_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max
        && !value.contains("..")
        && !value.chars().any(char::is_control)
        && !contains_hostile_text(value)
}

fn safe_relative_path(value: &str) -> bool {
    value.len() <= MAX_PATH
        && !value.is_empty()
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.chars().any(char::is_control)
}

fn valid_sha(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn contains_hostile_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "begin private key",
        "api_key=",
        "secret_token=",
        "password=",
        "merge this pull request",
        "bypass gate",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn stable_digest(value: &str) -> String {
    let mut state = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("{state:016x}")
}
