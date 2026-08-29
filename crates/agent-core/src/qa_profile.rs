//! Contrato puro e bounded para perfis de QA.
//!
//! O módulo descreve planos e resultados de testes, mas não executa processos,
//! interpreta shell, altera expectativas, desativa gates ou decide releases.
//! Adapters externos são responsáveis por executar apenas um `QaPermit` válido.

use crate::task_mapping::{MappingState, TaskWorkspaceMapping};
use crate::{ProjectId, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const QA_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const MAX_QA_POLICY_REVISION_LEN: usize = 64;
pub const MAX_QA_IDENTIFIER_LEN: usize = 256;
pub const MAX_QA_COMMAND_TEXT_LEN: usize = 256;
pub const MAX_QA_COMMANDS: usize = 16;
pub const MAX_QA_RESULTS: usize = 64;
pub const MAX_QA_TIMEOUT_SECONDS: u64 = 3_600;
pub const MAX_QA_ATTEMPTS: u32 = 3;
pub const MAX_QA_OUTPUT_BYTES: u64 = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum QaProfileError {
    #[error("invalid QA profile: {0}")]
    InvalidProfile(String),
    #[error("QA mapping is not active")]
    MappingInactive,
    #[error("QA plan scope does not match the task mapping")]
    ScopeMismatch,
    #[error("QA plan policy revision is stale")]
    StalePolicy,
    #[error("QA command is not allowlisted")]
    ToolDenied,
    #[error("QA command contains shell or instruction-like text")]
    CommandDenied,
    #[error("invalid QA test plan: {0}")]
    InvalidPlan(String),
    #[error("invalid QA test result: {0}")]
    InvalidResult(String),
    #[error("QA result commit SHA does not match the plan")]
    ShaMismatch,
    #[error("QA result tree SHA does not match the plan")]
    TreeMismatch,
    #[error("QA result is missing, skipped, cancelled, or did not run")]
    MissingEvidence,
    #[error("QA result is stale")]
    StaleEvidence,
    #[error("QA result is malformed")]
    MalformedEvidence,
    #[error("QA report does not contain one result for every planned command")]
    IncompleteEvidence,
    #[error("QA artifact digest is missing")]
    ArtifactMissing,
}

/// Comandos conhecidos pelo contrato QA. Variantes livres existem apenas para
/// que a policy possa rejeitá-las explicitamente; nunca são executáveis aqui.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QaCommand {
    CargoTest,
    CargoCheck,
    CargoClippy,
    CargoFmtCheck,
    NodeTest,
    FeatureRunner,
    OnpVerify,
    Shell(String),
    Arbitrary(String),
}

impl QaCommand {
    fn is_allowlisted_kind(&self) -> bool {
        matches!(
            self,
            Self::CargoTest
                | Self::CargoCheck
                | Self::CargoClippy
                | Self::CargoFmtCheck
                | Self::NodeTest
                | Self::FeatureRunner
                | Self::OnpVerify
        )
    }

    fn validate_shape(&self) -> Result<(), QaProfileError> {
        match self {
            Self::Shell(text) | Self::Arbitrary(text)
                if text.is_empty()
                    || text.len() > MAX_QA_COMMAND_TEXT_LEN
                    || text.chars().any(char::is_control) =>
            {
                Err(QaProfileError::CommandDenied)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QaAgentProfile {
    pub schema_version: u32,
    pub policy_revision: String,
    pub allowed_commands: Vec<QaCommand>,
    pub max_commands: usize,
    pub max_timeout_seconds: u64,
    pub max_attempts: u32,
    pub max_output_bytes: u64,
    pub max_results: usize,
}

impl Default for QaAgentProfile {
    fn default() -> Self {
        Self {
            schema_version: QA_PROFILE_SCHEMA_VERSION,
            policy_revision: "qa-v1".into(),
            allowed_commands: vec![
                QaCommand::CargoTest,
                QaCommand::CargoCheck,
                QaCommand::CargoClippy,
                QaCommand::CargoFmtCheck,
                QaCommand::NodeTest,
                QaCommand::FeatureRunner,
                QaCommand::OnpVerify,
            ],
            max_commands: MAX_QA_COMMANDS,
            max_timeout_seconds: 900,
            max_attempts: 3,
            max_output_bytes: MAX_QA_OUTPUT_BYTES,
            max_results: MAX_QA_RESULTS,
        }
    }
}

impl QaAgentProfile {
    pub fn validate(&self) -> Result<(), QaProfileError> {
        if self.schema_version != QA_PROFILE_SCHEMA_VERSION {
            return Err(QaProfileError::InvalidProfile(
                "unsupported schema version".into(),
            ));
        }
        validate_text(
            &self.policy_revision,
            MAX_QA_POLICY_REVISION_LEN,
            "policy revision",
        )?;
        if self.allowed_commands.is_empty() || self.allowed_commands.len() > MAX_QA_COMMANDS {
            return Err(QaProfileError::InvalidProfile(
                "command allowlist is outside bounds".into(),
            ));
        }
        let mut unique = BTreeSet::new();
        for command in &self.allowed_commands {
            command.validate_shape()?;
            if !command.is_allowlisted_kind() || !unique.insert(command) {
                return Err(QaProfileError::InvalidProfile(
                    "allowlist contains a non-typed or duplicate command".into(),
                ));
            }
        }
        if self.max_commands == 0 || self.max_commands > MAX_QA_COMMANDS {
            return Err(QaProfileError::InvalidProfile(
                "command budget is outside bounds".into(),
            ));
        }
        if self.max_timeout_seconds == 0 || self.max_timeout_seconds > MAX_QA_TIMEOUT_SECONDS {
            return Err(QaProfileError::InvalidProfile(
                "timeout budget is outside bounds".into(),
            ));
        }
        if self.max_attempts == 0 || self.max_attempts > MAX_QA_ATTEMPTS {
            return Err(QaProfileError::InvalidProfile(
                "attempt budget is outside bounds".into(),
            ));
        }
        if self.max_output_bytes == 0 || self.max_output_bytes > MAX_QA_OUTPUT_BYTES {
            return Err(QaProfileError::InvalidProfile(
                "output budget is outside bounds".into(),
            ));
        }
        if self.max_results == 0 || self.max_results > MAX_QA_RESULTS {
            return Err(QaProfileError::InvalidProfile(
                "result budget is outside bounds".into(),
            ));
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        mapping: &TaskWorkspaceMapping,
        plan: &QaTestPlan,
    ) -> Result<QaPermit, QaProfileError> {
        self.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(QaProfileError::MappingInactive);
        }
        plan.validate()?;
        if plan.project_id != mapping.project_id()
            || plan.task_id != mapping.task_id()
            || plan.repository_id != mapping.repository_id()
            || plan.worktree_id != mapping.worktree_id()
            || plan.branch != mapping.branch()
        {
            return Err(QaProfileError::ScopeMismatch);
        }
        if plan.policy_revision != self.policy_revision {
            return Err(QaProfileError::StalePolicy);
        }
        if plan.commands.len() > self.max_commands {
            return Err(QaProfileError::InvalidPlan(
                "plan exceeds command budget".into(),
            ));
        }
        for command in &plan.commands {
            if !self.allowed_commands.contains(command) {
                return Err(QaProfileError::ToolDenied);
            }
        }
        Ok(QaPermit {
            project_id: plan.project_id,
            task_id: plan.task_id,
            worktree_id: plan.worktree_id.clone(),
            branch: plan.branch.clone(),
            head_sha: plan.head_sha.clone(),
            tree_sha: plan.tree_sha.clone(),
            policy_revision: plan.policy_revision.clone(),
            commands: plan.commands.clone(),
            max_timeout_seconds: self.max_timeout_seconds,
            max_attempts: self.max_attempts,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaPermit {
    project_id: ProjectId,
    task_id: TaskId,
    worktree_id: String,
    branch: String,
    head_sha: String,
    tree_sha: String,
    policy_revision: String,
    commands: Vec<QaCommand>,
    max_timeout_seconds: u64,
    max_attempts: u32,
}

impl QaPermit {
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

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    pub fn policy_revision(&self) -> &str {
        &self.policy_revision
    }

    pub fn commands(&self) -> &[QaCommand] {
        &self.commands
    }

    pub fn max_timeout_seconds(&self) -> u64 {
        self.max_timeout_seconds
    }

    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    pub fn can_execute(&self) -> bool {
        true
    }

    pub fn can_disable_checks(&self) -> bool {
        false
    }

    pub fn can_change_expectations(&self) -> bool {
        false
    }

    pub fn can_authorize_release(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QaTestPlan {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_revision: String,
    pub commands: Vec<QaCommand>,
}

impl QaTestPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        commands: Vec<QaCommand>,
    ) -> Result<Self, QaProfileError> {
        let plan = Self {
            project_id,
            task_id,
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            policy_revision: "qa-v1".into(),
            commands,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn commands(&self) -> &[QaCommand] {
        &self.commands
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    fn validate(&self) -> Result<(), QaProfileError> {
        validate_text(&self.repository_id, MAX_QA_IDENTIFIER_LEN, "repository")?;
        validate_text(&self.worktree_id, MAX_QA_IDENTIFIER_LEN, "worktree")?;
        validate_text(&self.branch, MAX_QA_IDENTIFIER_LEN, "branch")?;
        validate_text(
            &self.policy_revision,
            MAX_QA_POLICY_REVISION_LEN,
            "policy revision",
        )?;
        validate_sha(&self.head_sha, "commit SHA")?;
        validate_sha(&self.tree_sha, "tree SHA")?;
        if self.commands.is_empty() || self.commands.len() > MAX_QA_COMMANDS {
            return Err(QaProfileError::InvalidPlan(
                "plan command list is outside bounds".into(),
            ));
        }
        let mut unique = BTreeSet::new();
        for command in &self.commands {
            command.validate_shape()?;
        }
        if self.commands.iter().any(|command| !unique.insert(command)) {
            return Err(QaProfileError::InvalidPlan(
                "plan contains duplicate commands".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QaTestResultStatus {
    Passed,
    Failed,
    Skipped,
    NoRun,
    Cancelled,
    TimedOut,
    Malformed,
    Stale,
}

impl QaTestResultStatus {
    fn is_executed(self) -> bool {
        matches!(self, Self::Passed | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QaTestResult {
    pub command: QaCommand,
    pub head_sha: String,
    pub tree_sha: String,
    pub status: QaTestResultStatus,
    pub output_digest: String,
    pub artifact_digest: Option<String>,
    pub output_bytes: u64,
    pub duration_ms: u64,
    pub attempt: u32,
}

impl QaTestResult {
    pub fn new(
        command: QaCommand,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        status: QaTestResultStatus,
        output_digest: impl Into<String>,
        duration_ms: u64,
    ) -> Result<Self, QaProfileError> {
        let result = Self {
            command,
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            status,
            output_digest: output_digest.into(),
            artifact_digest: None,
            output_bytes: 0,
            duration_ms,
            attempt: 1,
        };
        result.validate_shape()?;
        Ok(result)
    }

    pub fn with_artifact_digest(
        mut self,
        digest: impl Into<String>,
    ) -> Result<Self, QaProfileError> {
        self.artifact_digest = Some(digest.into());
        self.validate_shape()?;
        Ok(self)
    }

    pub fn with_attempt(mut self, attempt: u32) -> Result<Self, QaProfileError> {
        self.attempt = attempt;
        self.validate_shape()?;
        Ok(self)
    }

    pub fn with_output_bytes(mut self, output_bytes: u64) -> Result<Self, QaProfileError> {
        self.output_bytes = output_bytes;
        self.validate_shape()?;
        Ok(self)
    }

    pub fn status(&self) -> QaTestResultStatus {
        self.status
    }

    fn validate_shape(&self) -> Result<(), QaProfileError> {
        self.command.validate_shape()?;
        validate_sha(&self.head_sha, "result commit SHA")?;
        validate_sha(&self.tree_sha, "result tree SHA")?;
        if self.attempt == 0 || self.attempt > MAX_QA_ATTEMPTS {
            return Err(QaProfileError::InvalidResult(
                "attempt is outside bounds".into(),
            ));
        }
        if self.output_bytes > MAX_QA_OUTPUT_BYTES {
            return Err(QaProfileError::InvalidResult(
                "output exceeds global bound".into(),
            ));
        }
        if self.output_digest.len() > 64 {
            return Err(QaProfileError::InvalidResult(
                "output digest exceeds 64 bytes".into(),
            ));
        }
        if self.status.is_executed() {
            validate_digest(&self.output_digest, "output digest")?;
        } else if !self.output_digest.is_empty() || self.artifact_digest.is_some() {
            return Err(QaProfileError::InvalidResult(
                "non-executed result cannot carry digests".into(),
            ));
        }
        if let Some(digest) = &self.artifact_digest {
            validate_digest(digest, "artifact digest")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QaReportStatus {
    Complete,
    Failed,
    Unknown,
    Stale,
    Malformed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QaReport {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_revision: String,
    pub commands: Vec<QaCommand>,
    pub results: Vec<QaTestResult>,
    pub status: QaReportStatus,
}

impl QaReport {
    pub fn new(plan: &QaTestPlan, results: Vec<QaTestResult>) -> Result<Self, QaProfileError> {
        plan.validate()?;
        if results.len() > MAX_QA_RESULTS {
            return Err(QaProfileError::InvalidResult(
                "report result list exceeds bounds".into(),
            ));
        }
        for result in &results {
            result.validate_shape()?;
        }
        let status = report_status(&plan.commands, &results);
        Ok(Self {
            project_id: plan.project_id,
            task_id: plan.task_id,
            repository_id: plan.repository_id.clone(),
            worktree_id: plan.worktree_id.clone(),
            branch: plan.branch.clone(),
            head_sha: plan.head_sha.clone(),
            tree_sha: plan.tree_sha.clone(),
            policy_revision: plan.policy_revision.clone(),
            commands: plan.commands.clone(),
            results,
            status,
        })
    }

    pub fn status(&self) -> QaReportStatus {
        self.status
    }

    pub fn is_success(&self) -> bool {
        self.status == QaReportStatus::Complete
    }

    pub fn can_authorize_release(&self) -> bool {
        false
    }

    pub fn can_disable_checks(&self) -> bool {
        false
    }

    pub fn can_change_expectations(&self) -> bool {
        false
    }

    pub fn failure_handoff(&self) -> Option<QaFailureHandoff> {
        if self.status != QaReportStatus::Failed {
            return None;
        }
        Some(QaFailureHandoff {
            project_id: self.project_id,
            task_id: self.task_id,
            head_sha: self.head_sha.clone(),
            tree_sha: self.tree_sha.clone(),
            status: QaFailureHandoffStatus::Failure,
            failed_commands: self
                .results
                .iter()
                .filter(|result| result.status == QaTestResultStatus::Failed)
                .map(|result| result.command.clone())
                .collect(),
        })
    }

    pub fn validate(
        &self,
        profile: &QaAgentProfile,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<(), QaProfileError> {
        profile.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(QaProfileError::MappingInactive);
        }
        if self.project_id != mapping.project_id()
            || self.task_id != mapping.task_id()
            || self.repository_id != mapping.repository_id()
            || self.worktree_id != mapping.worktree_id()
            || self.branch != mapping.branch()
        {
            return Err(QaProfileError::ScopeMismatch);
        }
        validate_text(&self.repository_id, MAX_QA_IDENTIFIER_LEN, "repository")?;
        validate_text(&self.worktree_id, MAX_QA_IDENTIFIER_LEN, "worktree")?;
        validate_text(&self.branch, MAX_QA_IDENTIFIER_LEN, "branch")?;
        validate_text(
            &self.policy_revision,
            MAX_QA_POLICY_REVISION_LEN,
            "policy revision",
        )?;
        if self.policy_revision != profile.policy_revision {
            return Err(QaProfileError::StalePolicy);
        }
        validate_sha(&self.head_sha, "report commit SHA")?;
        validate_sha(&self.tree_sha, "report tree SHA")?;
        if self.commands.is_empty()
            || self.commands.len() > profile.max_commands
            || self.commands.len() > MAX_QA_COMMANDS
        {
            return Err(QaProfileError::InvalidPlan(
                "report command list is outside profile bounds".into(),
            ));
        }
        for command in &self.commands {
            command.validate_shape()?;
            if !profile.allowed_commands.contains(command) {
                return Err(QaProfileError::ToolDenied);
            }
        }
        if self.results.len() > profile.max_results {
            return Err(QaProfileError::InvalidResult(
                "report exceeds result budget".into(),
            ));
        }
        if self.results.len() != self.commands.len() {
            return Err(QaProfileError::IncompleteEvidence);
        }
        let expected: BTreeSet<_> = self.commands.iter().collect();
        let mut seen = BTreeSet::new();
        for result in &self.results {
            result.validate_shape()?;
            if !expected.contains(&result.command) || !seen.insert(&result.command) {
                return Err(QaProfileError::InvalidResult(
                    "result command is unexpected or duplicated".into(),
                ));
            }
            if result.head_sha != self.head_sha {
                return Err(QaProfileError::ShaMismatch);
            }
            if result.tree_sha != self.tree_sha {
                return Err(QaProfileError::TreeMismatch);
            }
            if result.attempt > profile.max_attempts {
                return Err(QaProfileError::InvalidResult(
                    "result attempt exceeds profile budget".into(),
                ));
            }
            if result.duration_ms > profile.max_timeout_seconds.saturating_mul(1_000) {
                return Err(QaProfileError::InvalidResult(
                    "result duration exceeds timeout budget".into(),
                ));
            }
            if result.output_bytes > profile.max_output_bytes {
                return Err(QaProfileError::InvalidResult(
                    "result output exceeds profile budget".into(),
                ));
            }
            match result.status {
                QaTestResultStatus::Passed | QaTestResultStatus::Failed => {
                    if result.artifact_digest.is_none() {
                        return Err(QaProfileError::ArtifactMissing);
                    }
                }
                QaTestResultStatus::Skipped
                | QaTestResultStatus::NoRun
                | QaTestResultStatus::Cancelled => return Err(QaProfileError::MissingEvidence),
                QaTestResultStatus::TimedOut => return Err(QaProfileError::MissingEvidence),
                QaTestResultStatus::Malformed => return Err(QaProfileError::MalformedEvidence),
                QaTestResultStatus::Stale => return Err(QaProfileError::StaleEvidence),
            }
        }
        if seen.len() != expected.len() {
            return Err(QaProfileError::IncompleteEvidence);
        }
        let derived = report_status(&self.commands, &self.results);
        if derived != self.status {
            return Err(QaProfileError::MalformedEvidence);
        }
        if matches!(
            self.status,
            QaReportStatus::Unknown | QaReportStatus::Stale | QaReportStatus::Malformed
        ) {
            return Err(match self.status {
                QaReportStatus::Unknown => QaProfileError::MissingEvidence,
                QaReportStatus::Stale => QaProfileError::StaleEvidence,
                QaReportStatus::Malformed => QaProfileError::MalformedEvidence,
                QaReportStatus::Complete | QaReportStatus::Failed => {
                    QaProfileError::MalformedEvidence
                }
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QaFailureHandoffStatus {
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaFailureHandoff {
    project_id: ProjectId,
    task_id: TaskId,
    head_sha: String,
    tree_sha: String,
    status: QaFailureHandoffStatus,
    failed_commands: Vec<QaCommand>,
}

impl QaFailureHandoff {
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    pub fn status(&self) -> QaFailureHandoffStatus {
        self.status
    }

    pub fn failed_commands(&self) -> &[QaCommand] {
        &self.failed_commands
    }

    pub fn can_disable_checks(&self) -> bool {
        false
    }

    pub fn can_authorize_release(&self) -> bool {
        false
    }
}

fn report_status(commands: &[QaCommand], results: &[QaTestResult]) -> QaReportStatus {
    if results
        .iter()
        .any(|result| result.status == QaTestResultStatus::Malformed)
    {
        return QaReportStatus::Malformed;
    }
    if results
        .iter()
        .any(|result| result.status == QaTestResultStatus::Stale)
    {
        return QaReportStatus::Stale;
    }
    if results.len() != commands.len()
        || results.iter().any(|result| {
            matches!(
                result.status,
                QaTestResultStatus::Skipped
                    | QaTestResultStatus::NoRun
                    | QaTestResultStatus::Cancelled
                    | QaTestResultStatus::TimedOut
            )
        })
    {
        return QaReportStatus::Unknown;
    }
    if results
        .iter()
        .any(|result| result.status == QaTestResultStatus::Failed)
    {
        return QaReportStatus::Failed;
    }
    if results.iter().all(|result| {
        result.status == QaTestResultStatus::Passed && result.artifact_digest.is_some()
    }) {
        QaReportStatus::Complete
    } else {
        QaReportStatus::Unknown
    }
}

fn validate_text(value: &str, max_len: usize, label: &str) -> Result<(), QaProfileError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(QaProfileError::InvalidPlan(format!(
            "{label} is empty, oversized, or contains control characters"
        )));
    }
    Ok(())
}

fn validate_sha(value: &str, label: &str) -> Result<(), QaProfileError> {
    if (value.len() != 40 && value.len() != 64)
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(QaProfileError::InvalidPlan(format!(
            "{label} must be 40 or 64 hexadecimal characters"
        )));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), QaProfileError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(QaProfileError::InvalidResult(format!(
            "{label} must be 64 hexadecimal characters"
        )));
    }
    Ok(())
}
