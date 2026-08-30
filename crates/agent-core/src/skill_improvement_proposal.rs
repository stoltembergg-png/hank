//! Proposal-only skill improvement artifact. Active skills are never mutated here.

use thiserror::Error;

const MAX_TEXT: usize = 256;
const MAX_DIFF: usize = 64 * 1024;
const MAX_FILES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    Added,
    Modified,
    Removed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub kind: FileChangeKind,
    pub diff: String,
}
impl ChangedFile {
    pub fn new(path: &str, kind: FileChangeKind, diff: &str) -> Self {
        Self {
            path: path.into(),
            kind,
            diff: diff.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalRequest {
    pub skill_id: String,
    pub active_version: String,
    pub candidate_id: String,
    pub source_observation: String,
    pub policy_id: String,
    pub files: Vec<ChangedFile>,
    pub capabilities: Vec<String>,
    pub tests: Vec<String>,
}
impl ProposalRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        skill_id: &str,
        active_version: &str,
        candidate_id: &str,
        source: &str,
        policy: &str,
        files: Vec<ChangedFile>,
        capabilities: Vec<&str>,
        tests: Vec<&str>,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            active_version: active_version.into(),
            candidate_id: candidate_id.into(),
            source_observation: source.into(),
            policy_id: policy.into(),
            files,
            capabilities: capabilities.into_iter().map(str::to_owned).collect(),
            tests: tests.into_iter().map(str::to_owned).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    Draft,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProposalError {
    #[error("proposal provenance is incomplete")]
    InvalidProvenance,
    #[error("proposal diff exceeds bounds")]
    DiffTooLarge,
    #[error("proposal path is unsafe or hidden")]
    UnsafePath,
    #[error("proposal contains secret-like content")]
    SecretLikeContent,
    #[error("proposal has no declared tests")]
    MissingTests,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillImprovementProposal {
    active_version: String,
    fingerprint: String,
    status: ProposalStatus,
}
impl SkillImprovementProposal {
    pub fn create(request: ProposalRequest) -> Result<Self, ProposalError> {
        if [
            request.skill_id.as_str(),
            request.active_version.as_str(),
            request.candidate_id.as_str(),
            request.source_observation.as_str(),
            request.policy_id.as_str(),
        ]
        .iter()
        .any(|v| v.is_empty() || v.len() > MAX_TEXT)
            || request.files.is_empty()
            || request.tests.is_empty()
        {
            return Err(if request.tests.is_empty() {
                ProposalError::MissingTests
            } else {
                ProposalError::InvalidProvenance
            });
        }
        if request.files.len() > MAX_FILES {
            return Err(ProposalError::DiffTooLarge);
        }
        for file in &request.files {
            if file.diff.len() > MAX_DIFF
                || file.path.is_empty()
                || file.path.len() > MAX_TEXT
                || file.path.starts_with('.')
                || file.path.contains("..")
                || file.path.starts_with('/')
                || file.path.chars().any(char::is_control)
            {
                return Err(ProposalError::UnsafePath);
            }
            if contains_secret_like(&file.diff) {
                return Err(ProposalError::SecretLikeContent);
            }
        }
        let mut material = format!(
            "{}|{}|{}|{}|{}",
            request.skill_id,
            request.active_version,
            request.candidate_id,
            request.source_observation,
            request.policy_id
        );
        for file in &request.files {
            material.push_str(&format!("|{}|{:?}|{}", file.path, file.kind, file.diff));
        }
        for value in request.capabilities.iter().chain(request.tests.iter()) {
            material.push('|');
            material.push_str(value);
        }
        Ok(Self {
            active_version: request.active_version,
            fingerprint: digest(&material),
            status: ProposalStatus::Draft,
        })
    }
    pub fn active_version(&self) -> &str {
        &self.active_version
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn status(&self) -> ProposalStatus {
        self.status
    }
    pub fn can_activate(&self) -> bool {
        false
    }
}
fn contains_secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["token=", "api_key=", "apikey=", "password=", "secret="]
        .iter()
        .any(|m| lower.contains(m))
}
fn digest(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
