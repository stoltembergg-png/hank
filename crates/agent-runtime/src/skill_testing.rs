//! Deterministic, non-activating skill fixture harness.
//!
//! Fixtures are data only. This module deliberately has no process, network,
//! filesystem, provider, activation, or script execution boundary.

use agent_core::ids::{ProjectId, SkillId, TraceId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAX_VERSION_BYTES: usize = 64;
const MAX_STEPS: usize = 64;
const MAX_LABEL_BYTES: usize = 128;
const MAX_SCRIPT_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SkillTestError {
    #[error("skill test manifest is invalid: {0}")]
    InvalidManifest(&'static str),
    #[error("skill test step requires forbidden capability: {0}")]
    PrivilegedStep(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillTestStep {
    AssertLabel { label: String },
    ExecuteScript { source: String },
    RequestNetwork { target: String },
    MutateHost { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillFixture {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub version: String,
    pub trace_id: TraceId,
    pub steps: Vec<SkillTestStep>,
    pub max_steps: u16,
}

impl SkillFixture {
    pub fn new(
        project_id: ProjectId,
        skill_id: SkillId,
        version: impl Into<String>,
        trace_id: TraceId,
        steps: Vec<SkillTestStep>,
        max_steps: u16,
    ) -> Result<Self, SkillTestError> {
        let fixture = Self {
            project_id,
            skill_id,
            version: version.into(),
            trace_id,
            steps,
            max_steps,
        };
        fixture.validate()?;
        Ok(fixture)
    }

    fn validate(&self) -> Result<(), SkillTestError> {
        if self.version.trim().is_empty() || self.version.len() > MAX_VERSION_BYTES {
            return Err(SkillTestError::InvalidManifest("version"));
        }
        if self.max_steps == 0 || usize::from(self.max_steps) > MAX_STEPS {
            return Err(SkillTestError::InvalidManifest("max_steps"));
        }
        if self.steps.is_empty() || self.steps.len() > MAX_STEPS {
            return Err(SkillTestError::InvalidManifest("steps"));
        }
        if self.steps.len() > usize::from(self.max_steps) {
            return Err(SkillTestError::InvalidManifest("step budget"));
        }
        for step in &self.steps {
            match step {
                SkillTestStep::AssertLabel { label }
                    if { label.trim().is_empty() || label.len() > MAX_LABEL_BYTES } =>
                {
                    return Err(SkillTestError::InvalidManifest("label"))
                }
                SkillTestStep::ExecuteScript { source } if source.len() > MAX_SCRIPT_BYTES => {
                    return Err(SkillTestError::InvalidManifest("script"));
                }
                SkillTestStep::RequestNetwork { target } if target.len() > MAX_LABEL_BYTES => {
                    return Err(SkillTestError::InvalidManifest("network target"));
                }
                SkillTestStep::MutateHost { path } if path.len() > MAX_LABEL_BYTES => {
                    return Err(SkillTestError::InvalidManifest("host path"));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillTestReport {
    pub project_id: ProjectId,
    pub skill_id: SkillId,
    pub version: String,
    pub trace_id: TraceId,
    pub fixture_digest: String,
    pub steps_executed: u16,
    pub status: String,
    pub activation_requested: bool,
}

pub struct DeterministicSkillTestRunner;

impl DeterministicSkillTestRunner {
    pub fn run(fixture: &SkillFixture) -> Result<SkillTestReport, SkillTestError> {
        fixture.validate()?;
        for step in &fixture.steps {
            match step {
                SkillTestStep::ExecuteScript { .. } => {
                    return Err(SkillTestError::PrivilegedStep("script"));
                }
                SkillTestStep::RequestNetwork { .. } => {
                    return Err(SkillTestError::PrivilegedStep("network"));
                }
                SkillTestStep::MutateHost { .. } => {
                    return Err(SkillTestError::PrivilegedStep("host mutation"));
                }
                SkillTestStep::AssertLabel { .. } => {}
            }
        }

        let bytes = serde_json::to_vec(fixture)
            .map_err(|_| SkillTestError::InvalidManifest("serialization"))?;
        let digest = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(SkillTestReport {
            project_id: fixture.project_id,
            skill_id: fixture.skill_id,
            version: fixture.version.clone(),
            trace_id: fixture.trace_id,
            fixture_digest: digest,
            steps_executed: fixture.steps.len() as u16,
            status: "passed".into(),
            activation_requested: false,
        })
    }
}
