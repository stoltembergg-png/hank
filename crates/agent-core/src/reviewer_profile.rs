//! Perfil read-only e bounded para agentes reviewer.
//!
//! Este módulo valida identidade, escopo e evidência, mas nunca executa
//! ferramentas nem transforma um relatório advisory em aprovação ou merge.

use crate::task_mapping::{MappingState, TaskWorkspaceMapping};
use crate::{ProjectId, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const REVIEWER_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const MAX_REVIEWER_POLICY_REVISION_LEN: usize = 64;
pub const MAX_REVIEWER_SOURCE_LEN: usize = 256;
pub const MAX_REVIEWER_CODE_LEN: usize = 64;
pub const MAX_REVIEWER_SUMMARY_LEN: usize = 512;
pub const MAX_REVIEWER_FINDINGS: usize = 128;
pub const MAX_REVIEWER_EVIDENCE: usize = 128;
pub const MAX_REVIEWER_DIFF_BYTES: u64 = 1_048_576;
pub const MAX_REVIEWER_LOG_BYTES: u64 = 262_144;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReviewerProfileError {
    #[error("invalid reviewer profile: {0}")]
    InvalidProfile(String),
    #[error("reviewer mapping is not active")]
    MappingInactive,
    #[error("reviewer request scope does not match the mapping")]
    ScopeMismatch,
    #[error("reviewer commit SHA does not match the authorized request")]
    ShaMismatch,
    #[error("reviewer tree SHA does not match the authorized request")]
    TreeMismatch,
    #[error("reviewer tool is not allowlisted")]
    ToolDenied,
    #[error("reviewer write or mutation attempt is denied")]
    WriteDenied,
    #[error("reviewer path is outside the bounded read scope")]
    PathDenied,
    #[error("invalid reviewer request: {0}")]
    InvalidRequest(String),
    #[error("invalid reviewer evidence: {0}")]
    InvalidEvidence(String),
    #[error("reviewer evidence is missing, skipped, or not run")]
    MissingEvidence,
    #[error("reviewer evidence is stale")]
    StaleEvidence,
    #[error("reviewer finding is unknown because evidence is incomplete")]
    UnknownEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerTool {
    ReadFile,
    GitDiff,
    GitStatus,
    ReadChecks,
    ReadArtifact,
    WriteFile,
}

impl ReviewerTool {
    fn is_mutating(self) -> bool {
        matches!(self, Self::WriteFile)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewerAgentProfile {
    pub schema_version: u32,
    pub policy_revision: String,
    pub allowed_tools: Vec<ReviewerTool>,
    pub max_diff_bytes: u64,
    pub max_log_bytes: u64,
    pub max_findings: usize,
    pub max_evidence: usize,
}

impl Default for ReviewerAgentProfile {
    fn default() -> Self {
        Self {
            schema_version: REVIEWER_PROFILE_SCHEMA_VERSION,
            policy_revision: "reviewer-v1".into(),
            allowed_tools: vec![
                ReviewerTool::ReadFile,
                ReviewerTool::GitDiff,
                ReviewerTool::GitStatus,
                ReviewerTool::ReadChecks,
                ReviewerTool::ReadArtifact,
            ],
            max_diff_bytes: MAX_REVIEWER_DIFF_BYTES,
            max_log_bytes: MAX_REVIEWER_LOG_BYTES,
            max_findings: MAX_REVIEWER_FINDINGS,
            max_evidence: MAX_REVIEWER_EVIDENCE,
        }
    }
}

impl ReviewerAgentProfile {
    pub fn validate(&self) -> Result<(), ReviewerProfileError> {
        if self.schema_version != REVIEWER_PROFILE_SCHEMA_VERSION {
            return Err(ReviewerProfileError::InvalidProfile(
                "unsupported schema version".into(),
            ));
        }
        validate_text(
            &self.policy_revision,
            MAX_REVIEWER_POLICY_REVISION_LEN,
            "policy revision",
        )?;
        if self.allowed_tools.is_empty() || self.allowed_tools.len() > 16 {
            return Err(ReviewerProfileError::InvalidProfile(
                "tool allowlist is outside bounds".into(),
            ));
        }
        let mut unique = BTreeSet::new();
        for tool in &self.allowed_tools {
            if tool.is_mutating() || !unique.insert(*tool) {
                return Err(ReviewerProfileError::InvalidProfile(
                    "allowlist contains mutation or duplicate tool".into(),
                ));
            }
        }
        if self.max_diff_bytes == 0 || self.max_diff_bytes > MAX_REVIEWER_DIFF_BYTES {
            return Err(ReviewerProfileError::InvalidProfile(
                "diff budget is outside bounds".into(),
            ));
        }
        if self.max_log_bytes == 0 || self.max_log_bytes > MAX_REVIEWER_LOG_BYTES {
            return Err(ReviewerProfileError::InvalidProfile(
                "log budget is outside bounds".into(),
            ));
        }
        if self.max_findings == 0 || self.max_findings > MAX_REVIEWER_FINDINGS {
            return Err(ReviewerProfileError::InvalidProfile(
                "finding budget is outside bounds".into(),
            ));
        }
        if self.max_evidence == 0 || self.max_evidence > MAX_REVIEWER_EVIDENCE {
            return Err(ReviewerProfileError::InvalidProfile(
                "evidence budget is outside bounds".into(),
            ));
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        mapping: &TaskWorkspaceMapping,
        request: &ReviewerRequest,
    ) -> Result<ReviewerPermit, ReviewerProfileError> {
        self.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(ReviewerProfileError::MappingInactive);
        }
        request.validate()?;
        if request.project_id != mapping.project_id()
            || request.task_id != mapping.task_id()
            || request.repository_id != mapping.repository_id()
            || request.worktree_id != mapping.worktree_id()
            || request.branch != mapping.branch()
        {
            return Err(ReviewerProfileError::ScopeMismatch);
        }
        if request.write_requested || request.tool.is_mutating() {
            return Err(ReviewerProfileError::WriteDenied);
        }
        if !self.allowed_tools.contains(&request.tool) {
            return Err(ReviewerProfileError::ToolDenied);
        }
        Ok(ReviewerPermit {
            project_id: request.project_id,
            task_id: request.task_id,
            worktree_id: request.worktree_id.clone(),
            branch: request.branch.clone(),
            head_sha: request.head_sha.clone(),
            tree_sha: request.tree_sha.clone(),
            tool: request.tool,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewerRequest {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub tool: ReviewerTool,
    pub path: Option<String>,
    pub write_requested: bool,
}

impl ReviewerRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        task_id: TaskId,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        tool: ReviewerTool,
        path: Option<String>,
    ) -> Result<Self, ReviewerProfileError> {
        let request = Self {
            project_id,
            task_id,
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            tool,
            path,
            write_requested: false,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn with_tool(mut self, tool: ReviewerTool) -> Self {
        self.tool = tool;
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_head_sha(mut self, head_sha: impl Into<String>) -> Self {
        self.head_sha = head_sha.into();
        self
    }

    pub fn with_tree_sha(mut self, tree_sha: impl Into<String>) -> Self {
        self.tree_sha = tree_sha.into();
        self
    }

    pub fn with_project_id(mut self, project_id: ProjectId) -> Self {
        self.project_id = project_id;
        self
    }

    pub fn write_attempt(mut self) -> Self {
        self.write_requested = true;
        self
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    fn validate(&self) -> Result<(), ReviewerProfileError> {
        validate_text(&self.repository_id, MAX_REVIEWER_SOURCE_LEN, "repository")?;
        validate_text(&self.worktree_id, MAX_REVIEWER_SOURCE_LEN, "worktree")?;
        validate_text(&self.branch, MAX_REVIEWER_SOURCE_LEN, "branch")?;
        validate_sha(&self.head_sha, "commit SHA")?;
        validate_sha(&self.tree_sha, "tree SHA")?;
        if let Some(path) = &self.path {
            validate_path(path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewerPermit {
    project_id: ProjectId,
    task_id: TaskId,
    worktree_id: String,
    branch: String,
    head_sha: String,
    tree_sha: String,
    tool: ReviewerTool,
}

impl ReviewerPermit {
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

    pub fn tool(&self) -> ReviewerTool {
        self.tool
    }

    pub fn can_write(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerFindingStatus {
    Observed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewerFinding {
    pub code: String,
    pub severity: ReviewerSeverity,
    pub status: ReviewerFindingStatus,
    pub summary: String,
    pub evidence_ref: Option<String>,
}

impl ReviewerFinding {
    pub fn observed(
        code: impl Into<String>,
        severity: ReviewerSeverity,
        summary: impl Into<String>,
        evidence_ref: Option<String>,
    ) -> Result<Self, ReviewerProfileError> {
        let finding = Self {
            code: code.into(),
            severity,
            status: ReviewerFindingStatus::Observed,
            summary: summary.into(),
            evidence_ref,
        };
        finding.validate()?;
        Ok(finding)
    }

    pub fn unknown(
        code: impl Into<String>,
        summary: impl Into<String>,
    ) -> Result<Self, ReviewerProfileError> {
        let finding = Self {
            code: code.into(),
            severity: ReviewerSeverity::Info,
            status: ReviewerFindingStatus::Unknown,
            summary: summary.into(),
            evidence_ref: None,
        };
        finding.validate()?;
        Ok(finding)
    }

    pub fn status(&self) -> ReviewerFindingStatus {
        self.status
    }

    fn validate(&self) -> Result<(), ReviewerProfileError> {
        validate_text(&self.code, MAX_REVIEWER_CODE_LEN, "finding code")?;
        validate_text(&self.summary, MAX_REVIEWER_SUMMARY_LEN, "finding summary")?;
        if let Some(reference) = &self.evidence_ref {
            validate_text(reference, MAX_REVIEWER_SOURCE_LEN, "evidence reference")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerEvidenceKind {
    Test,
    Artifact,
    Diff,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerEvidenceStatus {
    Passed,
    Failed,
    Missing,
    Skipped,
    NoRun,
    Malformed,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewerEvidence {
    pub kind: ReviewerEvidenceKind,
    pub source: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub digest: String,
    pub status: ReviewerEvidenceStatus,
}

impl ReviewerEvidence {
    pub fn new(
        kind: ReviewerEvidenceKind,
        source: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        digest: impl Into<String>,
        status: ReviewerEvidenceStatus,
    ) -> Result<Self, ReviewerProfileError> {
        let evidence = Self {
            kind,
            source: source.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            digest: digest.into(),
            status,
        };
        evidence.validate_shape()?;
        Ok(evidence)
    }

    fn validate_shape(&self) -> Result<(), ReviewerProfileError> {
        validate_text(&self.source, MAX_REVIEWER_SOURCE_LEN, "evidence source")?;
        validate_sha(&self.head_sha, "evidence commit SHA")?;
        validate_sha(&self.tree_sha, "evidence tree SHA")?;
        if self.digest.len() > 64 {
            return Err(ReviewerProfileError::InvalidEvidence(
                "digest exceeds 64 bytes".into(),
            ));
        }
        if matches!(
            self.status,
            ReviewerEvidenceStatus::Passed | ReviewerEvidenceStatus::Failed
        ) {
            validate_digest(&self.digest)?;
        }
        if matches!(
            self.status,
            ReviewerEvidenceStatus::Missing
                | ReviewerEvidenceStatus::Skipped
                | ReviewerEvidenceStatus::NoRun
        ) && !self.digest.is_empty()
        {
            return Err(ReviewerProfileError::InvalidEvidence(
                "incomplete evidence cannot carry a digest".into(),
            ));
        }
        Ok(())
    }

    fn validate_against(&self, head_sha: &str, tree_sha: &str) -> Result<(), ReviewerProfileError> {
        self.validate_shape()?;
        if self.head_sha != head_sha {
            return Err(ReviewerProfileError::ShaMismatch);
        }
        if self.tree_sha != tree_sha {
            return Err(ReviewerProfileError::TreeMismatch);
        }
        match self.status {
            ReviewerEvidenceStatus::Passed | ReviewerEvidenceStatus::Failed => Ok(()),
            ReviewerEvidenceStatus::Missing
            | ReviewerEvidenceStatus::Skipped
            | ReviewerEvidenceStatus::NoRun => Err(ReviewerProfileError::MissingEvidence),
            ReviewerEvidenceStatus::Malformed => Err(ReviewerProfileError::InvalidEvidence(
                "artifact is malformed".into(),
            )),
            ReviewerEvidenceStatus::Stale => Err(ReviewerProfileError::StaleEvidence),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerReportStatus {
    Complete,
    Unknown,
    Stale,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerAuthority {
    Advisory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewerReport {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_revision: String,
    pub status: ReviewerReportStatus,
    pub authority: ReviewerAuthority,
    pub findings: Vec<ReviewerFinding>,
    pub evidence: Vec<ReviewerEvidence>,
}

impl ReviewerReport {
    pub fn new(
        request: &ReviewerRequest,
        findings: Vec<ReviewerFinding>,
        evidence: Vec<ReviewerEvidence>,
    ) -> Result<Self, ReviewerProfileError> {
        request.validate()?;
        if findings.len() > MAX_REVIEWER_FINDINGS || evidence.len() > MAX_REVIEWER_EVIDENCE {
            return Err(ReviewerProfileError::InvalidEvidence(
                "report exceeds finding or evidence bounds".into(),
            ));
        }
        for finding in &findings {
            finding.validate()?;
        }
        for item in &evidence {
            item.validate_shape()?;
        }
        let status = if evidence
            .iter()
            .any(|item| item.status == ReviewerEvidenceStatus::Malformed)
        {
            ReviewerReportStatus::Malformed
        } else if evidence
            .iter()
            .any(|item| item.status == ReviewerEvidenceStatus::Stale)
        {
            ReviewerReportStatus::Stale
        } else if evidence.is_empty()
            || findings
                .iter()
                .any(|finding| finding.status == ReviewerFindingStatus::Unknown)
            || evidence.iter().any(|item| {
                matches!(
                    item.status,
                    ReviewerEvidenceStatus::Missing
                        | ReviewerEvidenceStatus::Skipped
                        | ReviewerEvidenceStatus::NoRun
                )
            })
        {
            ReviewerReportStatus::Unknown
        } else {
            ReviewerReportStatus::Complete
        };
        Ok(Self {
            project_id: request.project_id,
            task_id: request.task_id,
            repository_id: request.repository_id.clone(),
            worktree_id: request.worktree_id.clone(),
            branch: request.branch.clone(),
            head_sha: request.head_sha.clone(),
            tree_sha: request.tree_sha.clone(),
            policy_revision: "reviewer-v1".into(),
            status,
            authority: ReviewerAuthority::Advisory,
            findings,
            evidence,
        })
    }

    pub fn status(&self) -> ReviewerReportStatus {
        self.status
    }

    pub fn is_advisory(&self) -> bool {
        self.authority == ReviewerAuthority::Advisory
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_merge(&self) -> bool {
        false
    }

    pub fn validate(
        &self,
        profile: &ReviewerAgentProfile,
        mapping: &TaskWorkspaceMapping,
    ) -> Result<(), ReviewerProfileError> {
        profile.validate()?;
        if mapping.state() != MappingState::Active {
            return Err(ReviewerProfileError::MappingInactive);
        }
        validate_text(&self.repository_id, MAX_REVIEWER_SOURCE_LEN, "repository")?;
        validate_text(&self.worktree_id, MAX_REVIEWER_SOURCE_LEN, "worktree")?;
        validate_text(&self.branch, MAX_REVIEWER_SOURCE_LEN, "branch")?;
        validate_text(
            &self.policy_revision,
            MAX_REVIEWER_POLICY_REVISION_LEN,
            "policy revision",
        )?;
        if self.policy_revision != profile.policy_revision {
            return Err(ReviewerProfileError::InvalidEvidence(
                "report policy revision is stale".into(),
            ));
        }
        validate_sha(&self.head_sha, "report commit SHA")?;
        validate_sha(&self.tree_sha, "report tree SHA")?;
        if self.project_id != mapping.project_id()
            || self.task_id != mapping.task_id()
            || self.repository_id != mapping.repository_id()
            || self.worktree_id != mapping.worktree_id()
            || self.branch != mapping.branch()
        {
            return Err(ReviewerProfileError::ScopeMismatch);
        }
        if self.findings.len() > profile.max_findings || self.evidence.len() > profile.max_evidence
        {
            return Err(ReviewerProfileError::InvalidEvidence(
                "report exceeds profile bounds".into(),
            ));
        }
        if self.evidence.is_empty() {
            return Err(ReviewerProfileError::MissingEvidence);
        }
        for finding in &self.findings {
            finding.validate()?;
            if finding.status == ReviewerFindingStatus::Unknown {
                return Err(ReviewerProfileError::UnknownEvidence);
            }
        }
        for item in &self.evidence {
            item.validate_against(&self.head_sha, &self.tree_sha)?;
        }
        Ok(())
    }
}

fn validate_text(value: &str, max_len: usize, label: &str) -> Result<(), ReviewerProfileError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(ReviewerProfileError::InvalidRequest(format!(
            "{label} is empty, oversized, or contains control characters"
        )));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), ReviewerProfileError> {
    if path.is_empty()
        || path.len() > MAX_REVIEWER_SOURCE_LEN
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains("..")
        || path.contains(':')
        || path.chars().any(char::is_control)
    {
        return Err(ReviewerProfileError::PathDenied);
    }
    Ok(())
}

fn validate_sha(value: &str, label: &str) -> Result<(), ReviewerProfileError> {
    let valid_length = value.len() == 40 || value.len() == 64;
    if !valid_length || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ReviewerProfileError::InvalidRequest(format!(
            "{label} must be 40 or 64 hexadecimal characters"
        )));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ReviewerProfileError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ReviewerProfileError::InvalidEvidence(
            "digest must be 64 hexadecimal characters".into(),
        ));
    }
    Ok(())
}
