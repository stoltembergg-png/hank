use crate::workflow_repo::SqliteWorkflowRepository;
use agent_protocol::ids::{AgentId, ProjectId, WorkflowId};
use thiserror::Error;
use workflow_core::WorkflowStatus;

const MAX_ID: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunRequest {
    pub project_id: String,
    pub owner_id: String,
    pub job_id: String,
    pub run_id: String,
    pub workflow_id: String,
    pub workflow_version: u32,
    pub idempotency_key: String,
    pub policy_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum WorkflowSchedulerError {
    #[error("workflow scheduler identity is invalid")]
    InvalidIdentity,
    #[error("scheduled workflow version was not found")]
    NotFound,
    #[error("scheduled workflow version is not active")]
    NotActive,
    #[error("scheduled workflow owner does not match")]
    OwnerMismatch,
    #[error("scheduled workflow project does not match")]
    ProjectMismatch,
    #[error("workflow repository query failed")]
    Repository,
}

impl WorkflowRunRequest {
    pub async fn prepare(
        repository: &SqliteWorkflowRepository,
        project_id: &ProjectId,
        owner_id: &AgentId,
        job_id: &str,
        run_id: &str,
        workflow_id: &WorkflowId,
        version: u32,
    ) -> Result<Self, WorkflowSchedulerError> {
        let project_text = project_id.to_string();
        let owner_text = owner_id.to_string();
        let workflow_text = workflow_id.to_string();
        for value in [
            project_text.as_str(),
            owner_text.as_str(),
            job_id,
            run_id,
            workflow_text.as_str(),
        ] {
            if value.is_empty() || value.len() > MAX_ID || value.chars().any(char::is_control) {
                return Err(WorkflowSchedulerError::InvalidIdentity);
            }
        }
        let Some((definition, _graph)) = repository
            .load_definition(project_id, workflow_id, version)
            .await
            .map_err(|_| WorkflowSchedulerError::Repository)?
        else {
            return Err(WorkflowSchedulerError::NotFound);
        };
        if definition.project_id != *project_id {
            return Err(WorkflowSchedulerError::ProjectMismatch);
        }
        if definition.owner_id != *owner_id {
            return Err(WorkflowSchedulerError::OwnerMismatch);
        }
        if definition.status != WorkflowStatus::Active {
            return Err(WorkflowSchedulerError::NotActive);
        }
        Ok(Self {
            project_id: project_text,
            owner_id: owner_text,
            job_id: job_id.into(),
            run_id: run_id.into(),
            workflow_id: workflow_text,
            workflow_version: version,
            idempotency_key: format!("scheduler:workflow:{}:{}:{}", project_id, run_id, version),
            policy_ref: definition.policy_ref,
        })
    }
}
