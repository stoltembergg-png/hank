//! Declarative Skill manifest and lifecycle domain contracts.
//!
//! A manifest is metadata only. It describes an untrusted skill package and
//! never authorizes capabilities, changes instruction hierarchy, loads files,
//! or executes scripts.

use crate::budget::BudgetLimits;
use crate::ids::{ProjectId, SkillId, TraceId};
use agent_protocol::{Capability, Resource};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

pub const SKILL_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const SKILL_TRACE_SCHEMA_VERSION: u32 = 1;
pub const MAX_SKILL_NAME_BYTES: usize = 64;
pub const MAX_SKILL_DESCRIPTION_BYTES: usize = 2 * 1024;
pub const MAX_SKILL_AUTHOR_BYTES: usize = 128;
pub const MAX_SKILL_LICENSE_BYTES: usize = 64;
pub const MAX_SKILL_SOURCE_BYTES: usize = 512;
pub const MAX_SKILL_CAPABILITIES: usize = 32;
pub const MAX_SKILL_DEPENDENCIES: usize = 32;
pub const MAX_SKILL_FILES: usize = 128;
pub const MAX_SKILL_PATH_BYTES: usize = 256;
pub const MAX_SKILL_TESTS: usize = 32;
pub const MAX_SKILL_DIGEST_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SkillManifestError {
    #[error("unsupported skill manifest schema version")]
    UnsupportedSchemaVersion,
    #[error("skill manifest field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("skill manifest contains duplicate {0}")]
    Duplicate(&'static str),
    #[error("skill manifest requires SKILL.md as an instruction file")]
    MissingInstructionFile,
    #[error("skill manifest test file is not declared")]
    MissingTestFile,
    #[error("skill manifest capability is not allowed to access secrets")]
    SecretCapability,
    #[error("skill manifest budget is invalid")]
    InvalidBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillScope {
    Project,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSourceKind {
    BuiltIn,
    Local,
    Git,
    Registry,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillSource {
    pub kind: SkillSourceKind,
    pub reference: String,
}

impl SkillSource {
    pub fn local(reference: impl Into<String>) -> Self {
        Self {
            kind: SkillSourceKind::Local,
            reference: reference.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPolicy {
    /// Activation is always an explicit host/evaluator decision.
    pub requires_approval: bool,
    /// A manifest cannot authorize silent runtime mutation.
    pub allow_runtime_mutation: bool,
    /// Skill instructions cannot override system or security layers.
    pub allow_instruction_override: bool,
}

impl Default for SkillPolicy {
    fn default() -> Self {
        Self {
            requires_approval: true,
            allow_runtime_mutation: false,
            allow_instruction_override: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillTraceMetadata {
    pub schema_version: u32,
    pub trace_id: TraceId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillManifest {
    pub schema_version: u32,
    pub id: SkillId,
    pub version: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub repository: Option<String>,
    pub source: SkillSource,
    pub scope: SkillScope,
    pub capabilities: Vec<Capability>,
    pub dependencies: Vec<SkillDependency>,
    pub files: Vec<SkillFile>,
    pub tests: Vec<String>,
    pub policy: SkillPolicy,
    pub budget: BudgetLimits,
    pub trace: SkillTraceMetadata,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub digest: String,
}

impl SkillManifest {
    /// Creates a bounded manifest fixture with no executable or privileged
    /// behavior. Importers must still validate it before persistence.
    pub fn new(name: impl Into<String>, version: impl Into<String>, scope: SkillScope) -> Self {
        Self {
            schema_version: SKILL_MANIFEST_SCHEMA_VERSION,
            id: SkillId::new(),
            version: version.into(),
            name: name.into(),
            description: "Declarative skill metadata".into(),
            author: "unknown".into(),
            license: "UNLICENSED".into(),
            repository: None,
            source: SkillSource::local("workspace://skill"),
            scope,
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            files: vec![SkillFile {
                path: "SKILL.md".into(),
                role: SkillFileRole::Instruction,
                digest: "0".repeat(MAX_SKILL_DIGEST_BYTES),
            }],
            tests: Vec::new(),
            policy: SkillPolicy::default(),
            budget: BudgetLimits::default(),
            trace: SkillTraceMetadata {
                schema_version: SKILL_TRACE_SCHEMA_VERSION,
                trace_id: TraceId::new(),
            },
            created_at: chrono::Utc::now(),
            digest: "0".repeat(MAX_SKILL_DIGEST_BYTES),
        }
    }

    pub fn validate(&self) -> Result<(), SkillManifestError> {
        if self.schema_version != SKILL_MANIFEST_SCHEMA_VERSION {
            return Err(SkillManifestError::UnsupportedSchemaVersion);
        }
        validate_name(&self.name)?;
        validate_text(
            &self.description,
            MAX_SKILL_DESCRIPTION_BYTES,
            true,
            "description",
        )?;
        validate_text(&self.author, MAX_SKILL_AUTHOR_BYTES, false, "author")?;
        validate_text(&self.license, MAX_SKILL_LICENSE_BYTES, false, "license")?;
        if Version::parse(&self.version).is_err() {
            return Err(SkillManifestError::InvalidField("version"));
        }
        validate_optional_reference(self.repository.as_deref(), "repository")?;
        self.source.validate()?;

        if self.capabilities.len() > MAX_SKILL_CAPABILITIES {
            return Err(SkillManifestError::InvalidField("capabilities"));
        }
        let mut capabilities = HashSet::new();
        for capability in &self.capabilities {
            if capability.resource == Resource::Secret {
                return Err(SkillManifestError::SecretCapability);
            }
            if capability
                .scope
                .as_deref()
                .is_some_and(|scope| validate_scope_text(scope).is_err())
            {
                return Err(SkillManifestError::InvalidField("capability.scope"));
            }
            if !capabilities.insert(capability) {
                return Err(SkillManifestError::Duplicate("capability"));
            }
            if !self.policy.requires_approval && is_side_effect_capability(capability) {
                return Err(SkillManifestError::InvalidField("policy.requires_approval"));
            }
        }

        if self.dependencies.len() > MAX_SKILL_DEPENDENCIES {
            return Err(SkillManifestError::InvalidField("dependencies"));
        }
        let mut dependency_ids = HashSet::new();
        for dependency in &self.dependencies {
            if !dependency_ids.insert(dependency.skill_id) {
                return Err(SkillManifestError::Duplicate("dependency"));
            }
            if dependency.version_req.trim().is_empty()
                || dependency.version_req.len() > MAX_SKILL_NAME_BYTES
                || VersionReq::parse(&dependency.version_req).is_err()
            {
                return Err(SkillManifestError::InvalidField("dependency.version_req"));
            }
        }

        if self.files.len() > MAX_SKILL_FILES {
            return Err(SkillManifestError::InvalidField("files"));
        }
        let mut paths = HashSet::new();
        for file in &self.files {
            file.validate()?;
            if !paths.insert(&file.path) {
                return Err(SkillManifestError::Duplicate("file path"));
            }
        }
        if !self
            .files
            .iter()
            .any(|file| file.path == "SKILL.md" && file.role == SkillFileRole::Instruction)
        {
            return Err(SkillManifestError::MissingInstructionFile);
        }

        if self.tests.len() > MAX_SKILL_TESTS {
            return Err(SkillManifestError::InvalidField("tests"));
        }
        let mut tests = HashSet::new();
        for test in &self.tests {
            validate_relative_path(test, "test path")?;
            if !tests.insert(test) {
                return Err(SkillManifestError::Duplicate("test path"));
            }
            if !self
                .files
                .iter()
                .any(|file| file.path == *test && file.role == SkillFileRole::Test)
            {
                return Err(SkillManifestError::MissingTestFile);
            }
        }

        if self.policy.allow_runtime_mutation || self.policy.allow_instruction_override {
            return Err(SkillManifestError::InvalidField("policy"));
        }
        if self.budget.validate().is_err() {
            return Err(SkillManifestError::InvalidBudget);
        }
        if self.trace.schema_version != SKILL_TRACE_SCHEMA_VERSION
            || self.trace.trace_id.as_uuid().is_nil()
        {
            return Err(SkillManifestError::InvalidField("trace"));
        }
        validate_digest(&self.digest, "digest")?;
        Ok(())
    }

    /// This is a declaration lookup, not a permission grant.
    pub fn capability_is_declared(&self, capability: &Capability) -> bool {
        self.capabilities
            .iter()
            .any(|declared| declared == capability)
    }
}

impl SkillSource {
    fn validate(&self) -> Result<(), SkillManifestError> {
        validate_text(
            &self.reference,
            MAX_SKILL_SOURCE_BYTES,
            false,
            "source.reference",
        )?;
        if self.reference.contains("..") {
            return Err(SkillManifestError::InvalidField("source.reference"));
        }
        match self.kind {
            SkillSourceKind::Git
                if !(self.reference.starts_with("https://")
                    || self.reference.starts_with("ssh://")) =>
            {
                Err(SkillManifestError::InvalidField("source.reference"))
            }
            SkillSourceKind::BuiltIn if !self.reference.starts_with("builtin:") => {
                Err(SkillManifestError::InvalidField("source.reference"))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDependency {
    pub skill_id: SkillId,
    pub version_req: String,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillFile {
    pub path: String,
    pub role: SkillFileRole,
    pub digest: String,
}

impl SkillFile {
    fn validate(&self) -> Result<(), SkillManifestError> {
        validate_relative_path(&self.path, "file path")?;
        validate_digest(&self.digest, "file digest")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFileRole {
    Instruction,
    Script,
    Template,
    Reference,
    Test,
    Manifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Draft,
    Testing,
    Active,
    Deprecated,
    Archived,
    Blocked,
}

impl SkillStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Testing | Self::Blocked | Self::Archived)
                | (
                    Self::Testing,
                    Self::Draft | Self::Active | Self::Blocked | Self::Archived
                )
                | (Self::Active, Self::Deprecated | Self::Blocked)
                | (Self::Deprecated, Self::Active | Self::Archived)
                | (Self::Blocked, Self::Draft | Self::Archived)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SkillError {
    #[error("invalid skill state transition: {from:?} -> {to:?}")]
    InvalidTransition { from: SkillStatus, to: SkillStatus },
    #[error("skill version is invalid or does not match the manifest")]
    InvalidVersion,
    #[error("skill scope binding is invalid")]
    InvalidScope,
    #[error(transparent)]
    Manifest(#[from] SkillManifestError),
}

/// Skill de domínio; loading and execution are intentionally outside this card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub manifest: SkillManifest,
    pub status: SkillStatus,
    pub project_id: Option<ProjectId>,
    pub pinned_version: Option<String>,
    pub activated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rollback_version: Option<String>,
}

impl Skill {
    pub fn new(manifest: SkillManifest, project_id: Option<ProjectId>) -> Self {
        Self {
            manifest,
            status: SkillStatus::Draft,
            project_id,
            pinned_version: None,
            activated_at: None,
            rollback_version: None,
        }
    }

    pub fn validate(&self) -> Result<(), SkillError> {
        self.manifest.validate()?;
        match (self.manifest.scope, self.project_id) {
            (SkillScope::Project, Some(_)) | (SkillScope::Global, None) => {}
            _ => return Err(SkillError::InvalidScope),
        }
        if let Some(version) = &self.pinned_version {
            if Version::parse(version).is_err() {
                return Err(SkillError::InvalidVersion);
            }
        }
        if self.status == SkillStatus::Active && self.pinned_version.is_none() {
            return Err(SkillError::InvalidVersion);
        }
        Ok(())
    }

    pub fn transition(&mut self, next: SkillStatus) -> Result<(), SkillError> {
        if !self.status.can_transition_to(next) {
            return Err(SkillError::InvalidTransition {
                from: self.status,
                to: next,
            });
        }
        self.status = next;
        Ok(())
    }

    pub fn activate(&mut self, version: String) -> Result<(), SkillError> {
        let parsed = Version::parse(&version).map_err(|_| SkillError::InvalidVersion)?;
        if parsed
            != Version::parse(&self.manifest.version).map_err(|_| SkillError::InvalidVersion)?
        {
            return Err(SkillError::InvalidVersion);
        }
        self.transition(SkillStatus::Active)?;
        self.pinned_version = Some(version);
        self.activated_at = Some(chrono::Utc::now());
        Ok(())
    }

    pub fn rollback(&mut self, version: String) -> Result<(), SkillError> {
        Version::parse(&version).map_err(|_| SkillError::InvalidVersion)?;
        if !matches!(self.status, SkillStatus::Active | SkillStatus::Deprecated) {
            return Err(SkillError::InvalidTransition {
                from: self.status,
                to: SkillStatus::Active,
            });
        }
        self.rollback_version = self.pinned_version.clone();
        self.pinned_version = Some(version);
        self.status = SkillStatus::Active;
        self.activated_at = Some(chrono::Utc::now());
        Ok(())
    }

    pub fn deprecate(&mut self) -> Result<(), SkillError> {
        self.transition(SkillStatus::Deprecated)
    }
}

fn validate_name(name: &str) -> Result<(), SkillManifestError> {
    if name.is_empty()
        || name.len() > MAX_SKILL_NAME_BYTES
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'.' | b'_' | b'-'))
        })
    {
        return Err(SkillManifestError::InvalidField("name"));
    }
    Ok(())
}

fn validate_text(
    value: &str,
    max_bytes: usize,
    allow_empty: bool,
    field: &'static str,
) -> Result<(), SkillManifestError> {
    if (!allow_empty && value.trim().is_empty())
        || value.len() > max_bytes
        || value.chars().any(char::is_control)
        || contains_sensitive_marker(value)
    {
        return Err(SkillManifestError::InvalidField(field));
    }
    Ok(())
}

fn validate_optional_reference(
    reference: Option<&str>,
    field: &'static str,
) -> Result<(), SkillManifestError> {
    if let Some(reference) = reference {
        validate_text(reference, MAX_SKILL_SOURCE_BYTES, false, field)?;
        if reference.contains("..") {
            return Err(SkillManifestError::InvalidField(field));
        }
    }
    Ok(())
}

fn validate_scope_text(value: &str) -> Result<(), SkillManifestError> {
    validate_text(value, 160, false, "scope")?;
    if value.contains("..") || value.contains('/') || value.contains('\\') {
        return Err(SkillManifestError::InvalidField("scope"));
    }
    Ok(())
}

fn validate_relative_path(path: &str, field: &'static str) -> Result<(), SkillManifestError> {
    if path.is_empty()
        || path.len() > MAX_SKILL_PATH_BYTES
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.chars().any(char::is_control)
        || path
            .split(['/', '\\'])
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || contains_sensitive_marker(path)
    {
        return Err(SkillManifestError::InvalidField(field));
    }
    Ok(())
}

fn validate_digest(digest: &str, field: &'static str) -> Result<(), SkillManifestError> {
    if digest.len() != MAX_SKILL_DIGEST_BYTES
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(SkillManifestError::InvalidField(field));
    }
    Ok(())
}

fn is_side_effect_capability(capability: &Capability) -> bool {
    !matches!(
        capability.action,
        agent_protocol::Action::Read
            | agent_protocol::Action::List
            | agent_protocol::Action::Discover
    )
}

fn contains_sensitive_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "authorization:",
        "bearer ",
        "api_key=",
        "api-key=",
        "password=",
        "private key",
        "secret=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}
