use agent_protocol::{AgentId, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const WORKFLOW_SCHEMA_VERSION: u32 = 1;
const MAX_NAME_BYTES: usize = 128;
const MAX_POLICY_BYTES: usize = 128;
const MAX_METADATA_ENTRIES: usize = 32;
const MAX_METADATA_VALUE_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Draft,
    Active,
    Paused,
    Archived,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkflowError {
    #[error("workflow identity is invalid")]
    InvalidIdentity,
    #[error("workflow name is invalid or oversized")]
    InvalidName,
    #[error("workflow policy reference is invalid or oversized")]
    InvalidPolicy,
    #[error("workflow version is invalid")]
    InvalidVersion,
    #[error("workflow metadata is invalid or oversized")]
    MetadataTooLarge,
    #[error("workflow lifecycle transition is invalid")]
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub workflow_id: uuid::Uuid,
    pub project_id: ProjectId,
    pub owner_id: AgentId,
    pub name: String,
    pub version: u32,
    pub status: WorkflowStatus,
    pub policy_ref: String,
    pub metadata: BTreeMap<String, String>,
}

fn deserialize_schema_version<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let version = u32::deserialize(deserializer)?;
    if version == WORKFLOW_SCHEMA_VERSION {
        Ok(version)
    } else {
        Err(serde::de::Error::custom("unknown workflow schema version"))
    }
}

impl Workflow {
    pub fn new(
        project_id: ProjectId,
        owner_id: AgentId,
        name: String,
        policy_ref: String,
    ) -> Result<Self, WorkflowError> {
        if name.trim().is_empty() || name.len() > MAX_NAME_BYTES {
            return Err(WorkflowError::InvalidName);
        }
        if policy_ref.trim().is_empty() || policy_ref.len() > MAX_POLICY_BYTES {
            return Err(WorkflowError::InvalidPolicy);
        }
        Ok(Self {
            schema_version: WORKFLOW_SCHEMA_VERSION,
            workflow_id: uuid::Uuid::new_v4(),
            project_id,
            owner_id,
            name,
            version: 1,
            status: WorkflowStatus::Draft,
            policy_ref,
            metadata: BTreeMap::new(),
        })
    }
    pub fn set_version(&mut self, version: u32) -> Result<(), WorkflowError> {
        if version == 0 || version < self.version {
            return Err(WorkflowError::InvalidVersion);
        }
        self.version = version;
        Ok(())
    }
    pub fn set_metadata(&mut self, key: String, value: String) -> Result<(), WorkflowError> {
        if key.trim().is_empty()
            || value.len() > MAX_METADATA_VALUE_BYTES
            || (!self.metadata.contains_key(&key) && self.metadata.len() >= MAX_METADATA_ENTRIES)
        {
            return Err(WorkflowError::MetadataTooLarge);
        }
        self.metadata.insert(key, value);
        Ok(())
    }
    pub fn activate(&mut self) -> Result<(), WorkflowError> {
        if !matches!(self.status, WorkflowStatus::Draft | WorkflowStatus::Paused) {
            return Err(WorkflowError::InvalidTransition);
        }
        self.status = WorkflowStatus::Active;
        Ok(())
    }
    pub fn pause(&mut self) -> Result<(), WorkflowError> {
        if self.status != WorkflowStatus::Active {
            return Err(WorkflowError::InvalidTransition);
        }
        self.status = WorkflowStatus::Paused;
        Ok(())
    }
    pub fn archive(&mut self) -> Result<(), WorkflowError> {
        if matches!(
            self.status,
            WorkflowStatus::Archived | WorkflowStatus::Blocked
        ) {
            return Err(WorkflowError::InvalidTransition);
        }
        if self.status != WorkflowStatus::Active && self.status != WorkflowStatus::Paused {
            return Err(WorkflowError::InvalidTransition);
        }
        self.status = WorkflowStatus::Archived;
        Ok(())
    }
    pub fn block(&mut self) {
        self.status = WorkflowStatus::Blocked;
    }
}
