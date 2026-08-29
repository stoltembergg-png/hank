//! Perfil puro e bounded para agentes de coding.
//!
//! Este módulo valida apenas identidade, escopo, limites e handoff. Não executa
//! ferramentas, Git, filesystem, rede, providers, processos ou mutações externas.

use crate::task_mapping::{MappingState, TaskWorkspaceMapping};
use crate::{DomainError, ProjectId, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const CODING_PROFILE_SCHEMA_VERSION: u32 = 1;
const MAX_POLICY_REVISION_LEN: usize = 128;
const MAX_PROFILE_TOOLS: usize = 16;
const MAX_PROFILE_CHECKS: usize = 16;
const MAX_PATHS_PER_HANDOFF: usize = 128;
const MAX_PATH_LEN: usize = 512;
const MAX_DIGEST_LEN: usize = 64;
const MAX_CODING_TOKENS: u64 = 1_000_000;
const MAX_CODING_INVOCATIONS: u32 = 64;
const MAX_CODING_WALL_TIME_SECONDS: u64 = 86_400;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodingProfileError {
    #[error("coding profile is invalid: {0}")]
    InvalidProfile(String),
    #[error("coding request scope does not match task mapping")]
    ScopeMismatch,
    #[error("task mapping is not active")]
    MappingInactive,
    #[error("coding request identity contains unsafe instruction-like text")]
    UnsafeIdentity,
    #[error("coding tool is not allowlisted")]
    ToolDenied,
    #[error("coding path is invalid or outside the worktree")]
    PathDenied,
    #[error("network access is denied by coding profile")]
    NetworkDenied,
    #[error("publication is denied by coding profile")]
    PublicationDenied,
    #[error("merge is denied by coding profile")]
    MergeDenied,
    #[error("coding budget exceeded: {0}")]
    BudgetExceeded(String),
    #[error("coding request was cancelled")]
    Cancelled,
    #[error("coding handoff is invalid: {0}")]
    InvalidHandoff(String),
}

impl From<CodingProfileError> for DomainError {
    fn from(error: CodingProfileError) -> Self {
        DomainError::Validation(error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingTool {
    ReadFile,
    WriteFile,
    ApplyPatch,
    RunTests,
    RunArbitraryCommand,
    NetworkRequest,
    PublishChange,
    MergeChange,
}

impl CodingTool {
    fn is_forbidden(&self) -> bool {
        matches!(
            self,
            Self::RunArbitraryCommand
                | Self::NetworkRequest
                | Self::PublishChange
                | Self::MergeChange
        )
    }

    fn requires_path(&self) -> bool {
        matches!(self, Self::ReadFile | Self::WriteFile | Self::ApplyPatch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingBudget {
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub max_invocations: u32,
    pub max_attempts: u32,
    pub max_wall_time_seconds: u64,
}

impl Default for CodingBudget {
    fn default() -> Self {
        Self {
            max_input_tokens: 100_000,
            max_output_tokens: 100_000,
            max_invocations: 16,
            max_attempts: 3,
            max_wall_time_seconds: 900,
        }
    }
}

impl CodingBudget {
    fn validate(&self) -> Result<(), CodingProfileError> {
        if self.max_input_tokens == 0
            || self.max_input_tokens > MAX_CODING_TOKENS
            || self.max_output_tokens == 0
            || self.max_output_tokens > MAX_CODING_TOKENS
            || self.max_invocations == 0
            || self.max_invocations > MAX_CODING_INVOCATIONS
            || self.max_attempts == 0
            || self.max_attempts > 3
            || self.max_wall_time_seconds == 0
            || self.max_wall_time_seconds > MAX_CODING_WALL_TIME_SECONDS
        {
            return Err(CodingProfileError::InvalidProfile(
                "coding budget is outside bounded limits".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingAutonomy {
    allow_network: bool,
    allow_publication: bool,
    allow_merge: bool,
    max_attempts: u32,
}

impl CodingAutonomy {
    pub fn allow_network(&self) -> bool {
        self.allow_network
    }

    pub fn allow_publication(&self) -> bool {
        self.allow_publication
    }

    pub fn allow_merge(&self) -> bool {
        self.allow_merge
    }

    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    fn validate(&self) -> Result<(), CodingProfileError> {
        if self.allow_network || self.allow_publication || self.allow_merge {
            return Err(CodingProfileError::InvalidProfile(
                "coding autonomy cannot grant network, publication, or merge".into(),
            ));
        }
        if self.max_attempts == 0 || self.max_attempts > 3 {
            return Err(CodingProfileError::InvalidProfile(
                "coding autonomy attempts are outside bounded limits".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingCheck {
    #[default]
    Formatting,
    Tests,
    Lint,
    Security,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingAgentProfile {
    pub schema_version: u32,
    pub policy_revision: String,
    pub allowed_tools: Vec<CodingTool>,
    pub budget: CodingBudget,
    pub autonomy: CodingAutonomy,
    pub required_checks: Vec<CodingCheck>,
}

impl Default for CodingAgentProfile {
    fn default() -> Self {
        Self {
            schema_version: CODING_PROFILE_SCHEMA_VERSION,
            policy_revision: "coding-v1".into(),
            allowed_tools: vec![
                CodingTool::ReadFile,
                CodingTool::WriteFile,
                CodingTool::ApplyPatch,
                CodingTool::RunTests,
            ],
            budget: CodingBudget::default(),
            autonomy: CodingAutonomy {
                max_attempts: 3,
                ..Default::default()
            },
            required_checks: vec![
                CodingCheck::Formatting,
                CodingCheck::Tests,
                CodingCheck::Lint,
                CodingCheck::Security,
            ],
        }
    }
}

impl CodingAgentProfile {
    pub fn required_checks(&self) -> &[CodingCheck] {
        &self.required_checks
    }

    pub fn autonomy(&self) -> CodingAutonomy {
        self.autonomy
    }

    pub fn validate(&self) -> Result<(), CodingProfileError> {
        if self.schema_version != CODING_PROFILE_SCHEMA_VERSION
            || self.policy_revision.trim().is_empty()
            || self.policy_revision.len() > MAX_POLICY_REVISION_LEN
            || self.policy_revision.chars().any(char::is_control)
            || contains_instruction_like(&self.policy_revision)
        {
            return Err(CodingProfileError::InvalidProfile(
                "coding profile schema or policy revision is invalid".into(),
            ));
        }
        if self.allowed_tools.is_empty() || self.allowed_tools.len() > MAX_PROFILE_TOOLS {
            return Err(CodingProfileError::InvalidProfile(
                "coding profile tool allowlist is invalid".into(),
            ));
        }
        let mut tools = HashSet::new();
        for tool in &self.allowed_tools {
            if tool.is_forbidden() || !tools.insert(*tool) {
                return Err(CodingProfileError::InvalidProfile(
                    "coding profile contains forbidden or duplicate tool".into(),
                ));
            }
        }
        if self.required_checks.is_empty() || self.required_checks.len() > MAX_PROFILE_CHECKS {
            return Err(CodingProfileError::InvalidProfile(
                "coding profile required checks are invalid".into(),
            ));
        }
        let mut checks = HashSet::new();
        if self
            .required_checks
            .iter()
            .any(|check| !checks.insert(*check))
        {
            return Err(CodingProfileError::InvalidProfile(
                "coding profile contains duplicate required checks".into(),
            ));
        }
        self.budget.validate()?;
        self.autonomy.validate()?;
        if self.autonomy.max_attempts > self.budget.max_attempts {
            return Err(CodingProfileError::InvalidProfile(
                "coding autonomy attempts exceed budget attempts".into(),
            ));
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        mapping: &TaskWorkspaceMapping,
        request: &CodingAgentRequest,
    ) -> Result<CodingPermit, CodingProfileError> {
        self.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(CodingProfileError::MappingInactive);
        }
        if request.project_id != mapping.project_id()
            || request.task_id != mapping.task_id()
            || request.repository_id != mapping.repository_id()
            || request.worktree_id != mapping.worktree_id()
            || request.branch != mapping.branch()
        {
            return Err(CodingProfileError::ScopeMismatch);
        }
        validate_safe_identity(&request.repository_id)?;
        validate_safe_identity(&request.worktree_id)?;
        validate_safe_identity(&request.branch)?;
        if request.cancelled {
            return Err(CodingProfileError::Cancelled);
        }
        if request.network {
            return Err(CodingProfileError::NetworkDenied);
        }
        if request.publication {
            return Err(CodingProfileError::PublicationDenied);
        }
        if request.merge {
            return Err(CodingProfileError::MergeDenied);
        }
        if !self.allowed_tools.contains(&request.tool) || request.tool.is_forbidden() {
            return Err(CodingProfileError::ToolDenied);
        }
        validate_optional_path(request.path.as_deref(), request.tool.requires_path())?;
        validate_usage(&self.budget, &request.usage)?;
        Ok(CodingPermit {
            project_id: mapping.project_id(),
            task_id: mapping.task_id(),
            worktree_id: mapping.worktree_id().to_owned(),
            branch: mapping.branch().to_owned(),
            policy_revision: self.policy_revision.clone(),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingBudgetUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub invocations: u32,
    pub attempts: u32,
    pub wall_time_seconds: u64,
}

impl CodingBudgetUsage {
    pub fn new(
        input_tokens: u64,
        output_tokens: u64,
        invocations: u32,
        attempts: u32,
        wall_time_seconds: u64,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            invocations,
            attempts,
            wall_time_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingAgentRequest {
    project_id: ProjectId,
    task_id: TaskId,
    repository_id: String,
    worktree_id: String,
    branch: String,
    tool: CodingTool,
    path: Option<String>,
    usage: CodingBudgetUsage,
    network: bool,
    publication: bool,
    merge: bool,
    cancelled: bool,
}

impl CodingAgentRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        tool: CodingTool,
        path: Option<String>,
        usage: CodingBudgetUsage,
    ) -> Self {
        Self {
            project_id,
            task_id,
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            tool,
            path,
            usage,
            network: false,
            publication: false,
            merge: false,
            cancelled: false,
        }
    }

    pub fn with_project_id(mut self, project_id: ProjectId) -> Self {
        self.project_id = project_id;
        self
    }

    pub fn with_usage(mut self, usage: CodingBudgetUsage) -> Self {
        self.usage = usage;
        self
    }

    pub fn requesting_network(mut self) -> Self {
        self.network = true;
        self
    }

    pub fn requesting_publication(mut self) -> Self {
        self.publication = true;
        self
    }

    pub fn requesting_merge(mut self) -> Self {
        self.merge = true;
        self
    }

    pub fn cancelled(mut self) -> Self {
        self.cancelled = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingPermit {
    project_id: ProjectId,
    task_id: TaskId,
    worktree_id: String,
    branch: String,
    policy_revision: String,
}

impl CodingPermit {
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn worktree_id(&self) -> &str {
        &self.worktree_id
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn can_publish(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Proposed,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingCheckStatus {
    Passed,
    Failed,
    Skipped,
    NoRun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingCheckResult {
    pub check: CodingCheck,
    pub status: CodingCheckStatus,
    pub digest: String,
}

impl CodingCheckResult {
    pub fn passed(check: CodingCheck, digest: impl Into<String>) -> Self {
        Self {
            check,
            status: CodingCheckStatus::Passed,
            digest: digest.into(),
        }
    }

    pub fn skipped(check: CodingCheck) -> Self {
        Self {
            check,
            status: CodingCheckStatus::Skipped,
            digest: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingAgentHandoff {
    pub schema_version: u32,
    pub profile_revision: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub status: HandoffStatus,
    pub changed_paths: Vec<String>,
    pub patch_digest: String,
    pub report_digest: String,
    pub checks: Vec<CodingCheckResult>,
}

impl CodingAgentHandoff {
    pub fn proposed(
        profile: &CodingAgentProfile,
        mapping: &TaskWorkspaceMapping,
        changed_paths: Vec<String>,
        patch_digest: String,
        report_digest: String,
        checks: Vec<CodingCheckResult>,
    ) -> Result<Self, CodingProfileError> {
        profile.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(CodingProfileError::MappingInactive);
        }
        validate_safe_identity(mapping.repository_id())?;
        validate_safe_identity(mapping.worktree_id())?;
        validate_safe_identity(mapping.branch())?;
        validate_handoff_paths(&changed_paths)?;
        validate_digest(&patch_digest).map_err(CodingProfileError::InvalidHandoff)?;
        validate_digest(&report_digest).map_err(CodingProfileError::InvalidHandoff)?;
        validate_check_shape(&checks)?;
        Ok(Self {
            schema_version: CODING_PROFILE_SCHEMA_VERSION,
            profile_revision: profile.policy_revision.clone(),
            project_id: mapping.project_id(),
            task_id: mapping.task_id(),
            repository_id: mapping.repository_id().into(),
            worktree_id: mapping.worktree_id().into(),
            branch: mapping.branch().into(),
            status: HandoffStatus::Proposed,
            changed_paths,
            patch_digest,
            report_digest,
            checks,
        })
    }

    pub fn with_branch(mut self, branch: String) -> Self {
        self.branch = branch;
        self
    }

    pub fn status(&self) -> HandoffStatus {
        self.status
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }

    pub fn validate(
        &self,
        profile: &CodingAgentProfile,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<(), CodingProfileError> {
        profile.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(CodingProfileError::MappingInactive);
        }
        if self.schema_version != CODING_PROFILE_SCHEMA_VERSION
            || self.profile_revision != profile.policy_revision
            || self.project_id != mapping.project_id()
            || self.task_id != mapping.task_id()
            || self.repository_id != mapping.repository_id()
            || self.worktree_id != mapping.worktree_id()
            || self.branch != mapping.branch()
        {
            return Err(CodingProfileError::InvalidHandoff(
                "handoff identity or profile revision is stale".into(),
            ));
        }
        if self.status != HandoffStatus::Proposed {
            return Err(CodingProfileError::InvalidHandoff(
                "handoff is not a proposal".into(),
            ));
        }
        validate_safe_identity(&self.profile_revision)?;
        validate_safe_identity(&self.repository_id)?;
        validate_safe_identity(&self.worktree_id)?;
        validate_safe_identity(&self.branch)?;
        validate_handoff_paths(&self.changed_paths)?;
        validate_digest(&self.patch_digest).map_err(CodingProfileError::InvalidHandoff)?;
        validate_digest(&self.report_digest).map_err(CodingProfileError::InvalidHandoff)?;
        validate_check_shape(&self.checks)?;
        let results: std::collections::HashMap<_, _> = self
            .checks
            .iter()
            .map(|result| (result.check, result))
            .collect();
        for required in profile.required_checks() {
            let Some(result) = results.get(required) else {
                return Err(CodingProfileError::InvalidHandoff(
                    "required check result is missing".into(),
                ));
            };
            if result.status != CodingCheckStatus::Passed {
                return Err(CodingProfileError::InvalidHandoff(
                    "required check did not pass".into(),
                ));
            }
            validate_digest(&result.digest).map_err(CodingProfileError::InvalidHandoff)?;
        }
        for result in &self.checks {
            if result.status == CodingCheckStatus::Passed {
                validate_digest(&result.digest).map_err(CodingProfileError::InvalidHandoff)?;
            }
        }
        Ok(())
    }
}

fn validate_usage(
    budget: &CodingBudget,
    usage: &CodingBudgetUsage,
) -> Result<(), CodingProfileError> {
    if usage.input_tokens > budget.max_input_tokens
        || usage.output_tokens > budget.max_output_tokens
        || usage.invocations > budget.max_invocations
        || usage.attempts > budget.max_attempts
        || usage.wall_time_seconds > budget.max_wall_time_seconds
    {
        return Err(CodingProfileError::BudgetExceeded(
            "usage exceeds profile limits".into(),
        ));
    }
    Ok(())
}

fn validate_optional_path(path: Option<&str>, required: bool) -> Result<(), CodingProfileError> {
    if required && path.is_none() {
        return Err(CodingProfileError::PathDenied);
    }
    if let Some(path) = path {
        if path.is_empty()
            || path.len() > MAX_PATH_LEN
            || path.starts_with('/')
            || path.starts_with('\\')
            || path.contains("..")
            || path.contains('\\')
            || path.contains(':')
            || path.contains('\n')
            || path.contains('\r')
            || path.chars().any(char::is_control)
            || contains_instruction_like(path)
        {
            return Err(CodingProfileError::PathDenied);
        }
    }
    Ok(())
}

fn validate_handoff_paths(paths: &[String]) -> Result<(), CodingProfileError> {
    if paths.is_empty() || paths.len() > MAX_PATHS_PER_HANDOFF {
        return Err(CodingProfileError::InvalidHandoff(
            "changed path list is outside bounds".into(),
        ));
    }
    for path in paths {
        validate_optional_path(Some(path), true)
            .map_err(|_| CodingProfileError::InvalidHandoff("changed path is invalid".into()))?;
    }
    Ok(())
}

fn validate_digest(digest: &str) -> Result<(), String> {
    if digest.len() != MAX_DIGEST_LEN
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("digest must be 64 lowercase hexadecimal characters".into());
    }
    Ok(())
}

fn validate_safe_identity(value: &str) -> Result<(), CodingProfileError> {
    if value.trim().is_empty()
        || value.chars().any(char::is_control)
        || contains_instruction_like(value)
    {
        return Err(CodingProfileError::UnsafeIdentity);
    }
    Ok(())
}

fn contains_instruction_like(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "system prompt",
        "approve merge",
        "grant capability",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn validate_check_shape(checks: &[CodingCheckResult]) -> Result<(), CodingProfileError> {
    if checks.is_empty() || checks.len() > MAX_PROFILE_CHECKS {
        return Err(CodingProfileError::InvalidHandoff(
            "check result list is outside bounds".into(),
        ));
    }
    let mut seen = HashSet::new();
    if checks.iter().any(|result| !seen.insert(result.check)) {
        return Err(CodingProfileError::InvalidHandoff(
            "duplicate check result".into(),
        ));
    }
    Ok(())
}
