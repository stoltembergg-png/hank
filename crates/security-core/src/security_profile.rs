//! Contrato puro e advisory para perfis de análise de segurança.
//!
//! Este módulo não explora sistemas, executa comandos, acessa secrets, interpreta
//! shell/prompt, altera gates ou concede aprovação. Adapters externos podem
//! consumir um `SecurityPermit` somente para preparar uma análise autorizada.

use std::collections::BTreeSet;
use thiserror::Error;

pub const SECURITY_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const MAX_SECURITY_REVISION_LEN: usize = 64;
pub const MAX_SECURITY_IDENTIFIER_LEN: usize = 128;
pub const MAX_SECURITY_DESCRIPTION_LEN: usize = 512;
pub const MAX_SECURITY_CASES: usize = 32;
pub const MAX_SECURITY_FINDINGS: usize = 64;
pub const MAX_SECURITY_EVIDENCE: usize = 64;
pub const MAX_SECURITY_ARTIFACT_BYTES: u64 = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SecurityProfileError {
    #[error("security profile is invalid")]
    InvalidProfile,
    #[error("security manifest is invalid")]
    InvalidManifest,
    #[error("security metadata is invalid")]
    InvalidMetadata,
    #[error("security scope does not match the profile")]
    ScopeMismatch,
    #[error("security policy revision is stale")]
    StalePolicy,
    #[error("security control is not allowlisted")]
    ControlDenied,
    #[error("security evidence is missing")]
    MissingEvidence,
    #[error("security evidence is stale")]
    StaleEvidence,
    #[error("security evidence is malformed")]
    MalformedEvidence,
    #[error("security evidence identity does not match the manifest")]
    EvidenceIdentityMismatch,
    #[error("security artifact digest is missing")]
    ArtifactMissing,
    #[error("security evidence is incomplete")]
    IncompleteEvidence,
    #[error("security finding is invalid")]
    InvalidFinding,
    #[error("security finding evidence does not match")]
    FindingEvidenceMismatch,
    #[error("security hypothesis is not evidence")]
    HypothesisUnproven,
    #[error("failed security evidence has no finding")]
    UnmappedFailure,
    #[error("security report status is blocked")]
    Blocked,
    #[error("security report status is unknown")]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityFindingClassification {
    Evidence,
    Hypothesis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityFindingSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityFindingStatus {
    Open,
    Resolved,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityEvidenceStatus {
    Passed,
    Failed,
    Missing,
    Skipped,
    NoRun,
    Malformed,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityReportStatus {
    Pass,
    Fail,
    Blocked,
    Unknown,
    Stale,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityHandoffStatus {
    Failure,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAgentProfile {
    pub schema_version: u32,
    pub evaluator_revision: String,
    pub policy_revision: String,
    pub project_id: String,
    pub task_id: String,
    pub repository_id: String,
    pub allowed_controls: Vec<String>,
    pub max_cases: usize,
    pub max_findings: usize,
    pub max_evidence: usize,
    pub max_artifact_bytes: u64,
}

impl SecurityAgentProfile {
    pub fn new(
        project_id: impl Into<String>,
        task_id: impl Into<String>,
        repository_id: impl Into<String>,
        allowed_controls: Vec<String>,
    ) -> Result<Self, SecurityProfileError> {
        let profile = Self {
            schema_version: SECURITY_PROFILE_SCHEMA_VERSION,
            evaluator_revision: "security-evaluator-v1".into(),
            policy_revision: "security-v1".into(),
            project_id: project_id.into(),
            task_id: task_id.into(),
            repository_id: repository_id.into(),
            allowed_controls,
            max_cases: MAX_SECURITY_CASES,
            max_findings: MAX_SECURITY_FINDINGS,
            max_evidence: MAX_SECURITY_EVIDENCE,
            max_artifact_bytes: MAX_SECURITY_ARTIFACT_BYTES,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), SecurityProfileError> {
        if self.schema_version != SECURITY_PROFILE_SCHEMA_VERSION
            || self.max_cases == 0
            || self.max_cases > MAX_SECURITY_CASES
            || self.max_findings == 0
            || self.max_findings > MAX_SECURITY_FINDINGS
            || self.max_evidence == 0
            || self.max_evidence > MAX_SECURITY_EVIDENCE
            || self.max_artifact_bytes == 0
            || self.max_artifact_bytes > MAX_SECURITY_ARTIFACT_BYTES
        {
            return Err(SecurityProfileError::InvalidProfile);
        }
        validate_text(
            &self.evaluator_revision,
            MAX_SECURITY_REVISION_LEN,
            "evaluator revision",
        )?;
        validate_text(
            &self.policy_revision,
            MAX_SECURITY_REVISION_LEN,
            "policy revision",
        )?;
        validate_text(&self.project_id, MAX_SECURITY_IDENTIFIER_LEN, "project")?;
        validate_text(&self.task_id, MAX_SECURITY_IDENTIFIER_LEN, "task")?;
        validate_text(
            &self.repository_id,
            MAX_SECURITY_IDENTIFIER_LEN,
            "repository",
        )?;
        if self.allowed_controls.is_empty() || self.allowed_controls.len() > MAX_SECURITY_CASES {
            return Err(SecurityProfileError::InvalidProfile);
        }
        let mut unique = BTreeSet::new();
        for control in &self.allowed_controls {
            validate_prefixed_id(control, "TM-")?;
            if !unique.insert(control) {
                return Err(SecurityProfileError::InvalidProfile);
            }
        }
        Ok(())
    }

    pub fn authorize(
        &self,
        manifest: &SecurityThreatManifest,
    ) -> Result<SecurityPermit, SecurityProfileError> {
        self.validate()?;
        manifest.validate()?;
        if manifest.project_id != self.project_id
            || manifest.task_id != self.task_id
            || manifest.repository_id != self.repository_id
        {
            return Err(SecurityProfileError::ScopeMismatch);
        }
        if manifest.policy_revision != self.policy_revision {
            return Err(SecurityProfileError::StalePolicy);
        }
        if manifest.cases.len() > self.max_cases {
            return Err(SecurityProfileError::InvalidManifest);
        }
        for threat_case in &manifest.cases {
            if !self.allowed_controls.contains(&threat_case.control_id) {
                return Err(SecurityProfileError::ControlDenied);
            }
        }
        Ok(SecurityPermit {
            project_id: manifest.project_id.clone(),
            task_id: manifest.task_id.clone(),
            repository_id: manifest.repository_id.clone(),
            worktree_id: manifest.worktree_id.clone(),
            branch: manifest.branch.clone(),
            head_sha: manifest.head_sha.clone(),
            tree_sha: manifest.tree_sha.clone(),
            policy_revision: manifest.policy_revision.clone(),
            threat_ids: manifest
                .cases
                .iter()
                .map(|threat_case| threat_case.threat_id.clone())
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityPermit {
    project_id: String,
    task_id: String,
    repository_id: String,
    worktree_id: String,
    branch: String,
    head_sha: String,
    tree_sha: String,
    policy_revision: String,
    threat_ids: Vec<String>,
}

impl SecurityPermit {
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn repository_id(&self) -> &str {
        &self.repository_id
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

    pub fn threat_ids(&self) -> &[String] {
        &self.threat_ids
    }

    pub fn can_exploit(&self) -> bool {
        false
    }

    pub fn can_access_secrets(&self) -> bool {
        false
    }

    pub fn can_mutate_gate(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityThreatCase {
    pub threat_id: String,
    pub control_id: String,
    pub test_id: String,
    pub description: String,
}

impl SecurityThreatCase {
    pub fn new(
        threat_id: impl Into<String>,
        control_id: impl Into<String>,
        test_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, SecurityProfileError> {
        let threat_case = Self {
            threat_id: threat_id.into(),
            control_id: control_id.into(),
            test_id: test_id.into(),
            description: description.into(),
        };
        threat_case.validate()?;
        Ok(threat_case)
    }

    pub fn threat_id(&self) -> &str {
        &self.threat_id
    }

    pub fn control_id(&self) -> &str {
        &self.control_id
    }

    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    fn validate(&self) -> Result<(), SecurityProfileError> {
        validate_prefixed_id(&self.threat_id, "THREAT-")?;
        validate_prefixed_id(&self.control_id, "TM-")?;
        validate_prefixed_id(&self.test_id, "TEST-")?;
        validate_text(
            &self.description,
            MAX_SECURITY_DESCRIPTION_LEN,
            "threat description",
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityThreatManifest {
    pub project_id: String,
    pub task_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_revision: String,
    pub cases: Vec<SecurityThreatCase>,
}

impl SecurityThreatManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: impl Into<String>,
        task_id: impl Into<String>,
        repository_id: impl Into<String>,
        worktree_id: impl Into<String>,
        branch: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        policy_revision: impl Into<String>,
        cases: Vec<SecurityThreatCase>,
    ) -> Result<Self, SecurityProfileError> {
        let manifest = Self {
            project_id: project_id.into(),
            task_id: task_id.into(),
            repository_id: repository_id.into(),
            worktree_id: worktree_id.into(),
            branch: branch.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            policy_revision: policy_revision.into(),
            cases,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), SecurityProfileError> {
        validate_text(&self.project_id, MAX_SECURITY_IDENTIFIER_LEN, "project")?;
        validate_text(&self.task_id, MAX_SECURITY_IDENTIFIER_LEN, "task")?;
        validate_text(
            &self.repository_id,
            MAX_SECURITY_IDENTIFIER_LEN,
            "repository",
        )?;
        validate_text(&self.worktree_id, MAX_SECURITY_IDENTIFIER_LEN, "worktree")?;
        validate_branch(&self.branch)?;
        validate_sha(&self.head_sha)?;
        validate_sha(&self.tree_sha)?;
        validate_text(
            &self.policy_revision,
            MAX_SECURITY_REVISION_LEN,
            "policy revision",
        )?;
        if self.cases.is_empty() || self.cases.len() > MAX_SECURITY_CASES {
            return Err(SecurityProfileError::InvalidManifest);
        }
        let mut unique = BTreeSet::new();
        for threat_case in &self.cases {
            threat_case.validate()?;
            if !unique.insert(&threat_case.threat_id) {
                return Err(SecurityProfileError::InvalidManifest);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityEvidence {
    pub threat_id: String,
    pub control_id: String,
    pub test_id: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_revision: String,
    pub status: SecurityEvidenceStatus,
    pub artifact_digest: Option<String>,
    pub evidence_digest: String,
    pub artifact_bytes: u64,
}

impl SecurityEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        threat_id: impl Into<String>,
        control_id: impl Into<String>,
        test_id: impl Into<String>,
        head_sha: impl Into<String>,
        tree_sha: impl Into<String>,
        policy_revision: impl Into<String>,
        status: SecurityEvidenceStatus,
        artifact_digest: impl Into<String>,
        evidence_digest: impl Into<String>,
        artifact_bytes: u64,
    ) -> Result<Self, SecurityProfileError> {
        let artifact_digest = artifact_digest.into();
        let evidence_digest = evidence_digest.into();
        let evidence = Self {
            threat_id: threat_id.into(),
            control_id: control_id.into(),
            test_id: test_id.into(),
            head_sha: head_sha.into(),
            tree_sha: tree_sha.into(),
            policy_revision: policy_revision.into(),
            status,
            artifact_digest: (!artifact_digest.is_empty()).then_some(artifact_digest),
            evidence_digest,
            artifact_bytes,
        };
        evidence.validate_shape()?;
        Ok(evidence)
    }

    fn validate_shape(&self) -> Result<(), SecurityProfileError> {
        validate_prefixed_id(&self.threat_id, "THREAT-")?;
        validate_prefixed_id(&self.control_id, "TM-")?;
        validate_prefixed_id(&self.test_id, "TEST-")?;
        validate_sha(&self.head_sha)?;
        validate_sha(&self.tree_sha)?;
        validate_text(
            &self.policy_revision,
            MAX_SECURITY_REVISION_LEN,
            "policy revision",
        )?;
        if self.artifact_bytes > MAX_SECURITY_ARTIFACT_BYTES
            || (!self.evidence_digest.is_empty() && !is_digest(&self.evidence_digest))
        {
            return Err(SecurityProfileError::InvalidMetadata);
        }
        if let Some(digest) = &self.artifact_digest {
            if !is_digest(digest) {
                return Err(SecurityProfileError::InvalidMetadata);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    finding_id: String,
    threat_id: String,
    control_id: String,
    test_id: String,
    classification: SecurityFindingClassification,
    severity: SecurityFindingSeverity,
    status: SecurityFindingStatus,
    evidence_digest: Option<String>,
}

impl SecurityFinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        finding_id: impl Into<String>,
        threat_id: impl Into<String>,
        control_id: impl Into<String>,
        test_id: impl Into<String>,
        classification: SecurityFindingClassification,
        severity: SecurityFindingSeverity,
        status: SecurityFindingStatus,
        evidence_digest: Option<String>,
    ) -> Result<Self, SecurityProfileError> {
        let finding = Self {
            finding_id: finding_id.into(),
            threat_id: threat_id.into(),
            control_id: control_id.into(),
            test_id: test_id.into(),
            classification,
            severity,
            status,
            evidence_digest,
        };
        finding.validate_shape()?;
        Ok(finding)
    }

    pub fn finding_id(&self) -> &str {
        &self.finding_id
    }

    pub fn threat_id(&self) -> &str {
        &self.threat_id
    }

    pub fn control_id(&self) -> &str {
        &self.control_id
    }

    pub fn test_id(&self) -> &str {
        &self.test_id
    }

    pub fn classification(&self) -> SecurityFindingClassification {
        self.classification
    }

    pub fn status(&self) -> SecurityFindingStatus {
        self.status
    }

    pub fn evidence_digest(&self) -> Option<&str> {
        self.evidence_digest.as_deref()
    }

    fn validate_shape(&self) -> Result<(), SecurityProfileError> {
        validate_prefixed_id(&self.finding_id, "F-")?;
        validate_prefixed_id(&self.threat_id, "THREAT-")?;
        validate_prefixed_id(&self.control_id, "TM-")?;
        validate_prefixed_id(&self.test_id, "TEST-")?;
        match (&self.classification, &self.evidence_digest) {
            (SecurityFindingClassification::Evidence, Some(digest)) if is_digest(digest) => {}
            (SecurityFindingClassification::Hypothesis, None) => {}
            _ => return Err(SecurityProfileError::InvalidFinding),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityReport {
    pub project_id: String,
    pub task_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub branch: String,
    pub head_sha: String,
    pub tree_sha: String,
    pub policy_revision: String,
    pub findings: Vec<SecurityFinding>,
    pub evidence: Vec<SecurityEvidence>,
    pub status: SecurityReportStatus,
    manifest_cases: Vec<(String, String, String)>,
}

impl SecurityReport {
    pub fn new(
        manifest: &SecurityThreatManifest,
        findings: Vec<SecurityFinding>,
        evidence: Vec<SecurityEvidence>,
    ) -> Result<Self, SecurityProfileError> {
        manifest.validate()?;
        if findings.len() > MAX_SECURITY_FINDINGS || evidence.len() > MAX_SECURITY_EVIDENCE {
            return Err(SecurityProfileError::InvalidMetadata);
        }
        for finding in &findings {
            finding.validate_shape()?;
        }
        for item in &evidence {
            item.validate_shape()?;
        }
        let status = report_status(&findings, &evidence);
        Ok(Self {
            project_id: manifest.project_id.clone(),
            task_id: manifest.task_id.clone(),
            repository_id: manifest.repository_id.clone(),
            worktree_id: manifest.worktree_id.clone(),
            branch: manifest.branch.clone(),
            head_sha: manifest.head_sha.clone(),
            tree_sha: manifest.tree_sha.clone(),
            policy_revision: manifest.policy_revision.clone(),
            findings,
            evidence,
            status,
            manifest_cases: manifest
                .cases
                .iter()
                .map(|threat_case| {
                    (
                        threat_case.threat_id.clone(),
                        threat_case.control_id.clone(),
                        threat_case.test_id.clone(),
                    )
                })
                .collect(),
        })
    }

    pub fn status(&self) -> SecurityReportStatus {
        self.status
    }

    pub fn is_success(&self) -> bool {
        self.status == SecurityReportStatus::Pass
    }

    pub fn can_mutate_gate(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_access_secrets(&self) -> bool {
        false
    }

    pub fn failure_handoff(&self) -> Option<SecurityHandoff> {
        let status = match self.status {
            SecurityReportStatus::Fail => SecurityHandoffStatus::Failure,
            SecurityReportStatus::Blocked
            | SecurityReportStatus::Stale
            | SecurityReportStatus::Malformed => SecurityHandoffStatus::Blocked,
            SecurityReportStatus::Unknown => SecurityHandoffStatus::Unknown,
            SecurityReportStatus::Pass => return None,
        };
        Some(SecurityHandoff {
            project_id: self.project_id.clone(),
            task_id: self.task_id.clone(),
            head_sha: self.head_sha.clone(),
            tree_sha: self.tree_sha.clone(),
            status,
            finding_ids: self
                .findings
                .iter()
                .filter(|finding| finding.status == SecurityFindingStatus::Open)
                .map(|finding| finding.finding_id.clone())
                .collect(),
        })
    }

    pub fn validate(&self, profile: &SecurityAgentProfile) -> Result<(), SecurityProfileError> {
        profile.validate()?;
        validate_text(&self.project_id, MAX_SECURITY_IDENTIFIER_LEN, "project")?;
        validate_text(&self.task_id, MAX_SECURITY_IDENTIFIER_LEN, "task")?;
        validate_text(
            &self.repository_id,
            MAX_SECURITY_IDENTIFIER_LEN,
            "repository",
        )?;
        validate_text(&self.worktree_id, MAX_SECURITY_IDENTIFIER_LEN, "worktree")?;
        validate_branch(&self.branch)?;
        validate_sha(&self.head_sha)?;
        validate_sha(&self.tree_sha)?;
        if self.project_id != profile.project_id
            || self.task_id != profile.task_id
            || self.repository_id != profile.repository_id
        {
            return Err(SecurityProfileError::ScopeMismatch);
        }
        if self.policy_revision != profile.policy_revision {
            return Err(SecurityProfileError::StalePolicy);
        }
        if self.findings.len() > profile.max_findings || self.evidence.len() > profile.max_evidence
        {
            return Err(SecurityProfileError::InvalidMetadata);
        }
        if self.evidence.is_empty() {
            return Err(SecurityProfileError::IncompleteEvidence);
        }

        let mut manifest_cases = BTreeSet::new();
        for evidence in &self.evidence {
            evidence.validate_shape()?;
            let identity = (
                evidence.threat_id.as_str(),
                evidence.control_id.as_str(),
                evidence.test_id.as_str(),
            );
            if !manifest_cases.insert(identity) {
                return Err(SecurityProfileError::InvalidMetadata);
            }
            if evidence.head_sha != self.head_sha
                || evidence.tree_sha != self.tree_sha
                || evidence.policy_revision != self.policy_revision
            {
                return Err(SecurityProfileError::EvidenceIdentityMismatch);
            }
            if !profile.allowed_controls.contains(&evidence.control_id) {
                return Err(SecurityProfileError::ControlDenied);
            }
            match evidence.status {
                SecurityEvidenceStatus::Passed | SecurityEvidenceStatus::Failed => {
                    if evidence.artifact_digest.is_none() {
                        return Err(SecurityProfileError::ArtifactMissing);
                    }
                    if evidence.artifact_bytes > profile.max_artifact_bytes {
                        return Err(SecurityProfileError::InvalidMetadata);
                    }
                }
                SecurityEvidenceStatus::Missing
                | SecurityEvidenceStatus::Skipped
                | SecurityEvidenceStatus::NoRun => {
                    return Err(SecurityProfileError::MissingEvidence)
                }
                SecurityEvidenceStatus::Malformed => {
                    return Err(SecurityProfileError::MalformedEvidence)
                }
                SecurityEvidenceStatus::Stale => return Err(SecurityProfileError::StaleEvidence),
            }
        }

        let expected_cases: BTreeSet<_> = self.manifest_case_keys().into_iter().collect();
        if manifest_cases != expected_cases {
            return Err(SecurityProfileError::IncompleteEvidence);
        }

        let mut finding_ids = BTreeSet::new();
        for finding in &self.findings {
            finding.validate_shape()?;
            if !finding_ids.insert(&finding.finding_id) {
                return Err(SecurityProfileError::InvalidFinding);
            }
            let matching = self.evidence.iter().find(|evidence| {
                evidence.threat_id == finding.threat_id
                    && evidence.control_id == finding.control_id
                    && evidence.test_id == finding.test_id
            });
            let Some(evidence) = matching else {
                return Err(SecurityProfileError::FindingEvidenceMismatch);
            };
            match finding.classification {
                SecurityFindingClassification::Evidence => {
                    if finding.evidence_digest.as_deref() != Some(evidence.evidence_digest.as_str())
                        || evidence.status != SecurityEvidenceStatus::Failed
                    {
                        return Err(SecurityProfileError::FindingEvidenceMismatch);
                    }
                }
                SecurityFindingClassification::Hypothesis => {
                    return Err(SecurityProfileError::HypothesisUnproven)
                }
            }
        }

        for evidence in &self.evidence {
            if evidence.status == SecurityEvidenceStatus::Failed
                && !self.findings.iter().any(|finding| {
                    finding.classification == SecurityFindingClassification::Evidence
                        && finding.threat_id == evidence.threat_id
                        && finding.control_id == evidence.control_id
                        && finding.test_id == evidence.test_id
                        && finding.status == SecurityFindingStatus::Open
                })
            {
                return Err(SecurityProfileError::UnmappedFailure);
            }
        }

        let derived = report_status(&self.findings, &self.evidence);
        if derived != self.status {
            return Err(SecurityProfileError::MalformedEvidence);
        }
        match self.status {
            SecurityReportStatus::Pass | SecurityReportStatus::Fail => Ok(()),
            SecurityReportStatus::Blocked => Err(SecurityProfileError::Blocked),
            SecurityReportStatus::Unknown => Err(SecurityProfileError::Unknown),
            SecurityReportStatus::Stale => Err(SecurityProfileError::StaleEvidence),
            SecurityReportStatus::Malformed => Err(SecurityProfileError::MalformedEvidence),
        }
    }

    fn manifest_case_keys(&self) -> Vec<(&str, &str, &str)> {
        self.manifest_cases
            .iter()
            .map(|(threat_id, control_id, test_id)| {
                (threat_id.as_str(), control_id.as_str(), test_id.as_str())
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityHandoff {
    project_id: String,
    task_id: String,
    head_sha: String,
    tree_sha: String,
    status: SecurityHandoffStatus,
    finding_ids: Vec<String>,
}

impl SecurityHandoff {
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    pub fn tree_sha(&self) -> &str {
        &self.tree_sha
    }

    pub fn status(&self) -> SecurityHandoffStatus {
        self.status
    }

    pub fn finding_ids(&self) -> &[String] {
        &self.finding_ids
    }

    pub fn can_mutate_gate(&self) -> bool {
        false
    }

    pub fn can_approve(&self) -> bool {
        false
    }

    pub fn can_access_secrets(&self) -> bool {
        false
    }
}

fn report_status(
    findings: &[SecurityFinding],
    evidence: &[SecurityEvidence],
) -> SecurityReportStatus {
    if evidence
        .iter()
        .any(|item| item.status == SecurityEvidenceStatus::Malformed)
    {
        return SecurityReportStatus::Malformed;
    }
    if evidence
        .iter()
        .any(|item| item.status == SecurityEvidenceStatus::Stale)
    {
        return SecurityReportStatus::Stale;
    }
    if evidence.iter().any(|item| {
        matches!(
            item.status,
            SecurityEvidenceStatus::Missing
                | SecurityEvidenceStatus::Skipped
                | SecurityEvidenceStatus::NoRun
        )
    }) {
        return SecurityReportStatus::Blocked;
    }
    if findings
        .iter()
        .any(|finding| finding.classification == SecurityFindingClassification::Hypothesis)
    {
        return SecurityReportStatus::Unknown;
    }
    if evidence
        .iter()
        .any(|item| item.status == SecurityEvidenceStatus::Failed)
        || findings.iter().any(|finding| {
            finding.classification == SecurityFindingClassification::Evidence
                && finding.status == SecurityFindingStatus::Open
        })
    {
        return SecurityReportStatus::Fail;
    }
    if evidence
        .iter()
        .all(|item| item.status == SecurityEvidenceStatus::Passed)
    {
        SecurityReportStatus::Pass
    } else {
        SecurityReportStatus::Unknown
    }
}

fn validate_text(value: &str, max_len: usize, _label: &str) -> Result<(), SecurityProfileError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(SecurityProfileError::InvalidMetadata);
    }
    Ok(())
}

fn validate_prefixed_id(value: &str, prefix: &str) -> Result<(), SecurityProfileError> {
    validate_text(value, MAX_SECURITY_IDENTIFIER_LEN, "identifier")?;
    if !value.starts_with(prefix) {
        return Err(SecurityProfileError::InvalidMetadata);
    }
    Ok(())
}

fn validate_branch(value: &str) -> Result<(), SecurityProfileError> {
    validate_text(value, MAX_SECURITY_IDENTIFIER_LEN, "branch")?;
    if value.chars().any(char::is_whitespace) || value.contains("..") {
        return Err(SecurityProfileError::InvalidMetadata);
    }
    Ok(())
}

fn validate_sha(value: &str) -> Result<(), SecurityProfileError> {
    if (value.len() != 40 && value.len() != 64)
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(SecurityProfileError::InvalidMetadata);
    }
    Ok(())
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
