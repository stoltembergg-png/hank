//! Project application bridge for the desktop shell.
//!
//! The bridge owns only DTO translation and state access. Domain validation,
//! persistence, concurrency and lifecycle rules remain in agent-runtime.

use agent_core::error::{DomainError, DomainErrorCode, Retryability};
use agent_core::ids::ProjectId;
use agent_core::project::{Project, ProjectSettings, ProjectStatus};
use agent_runtime::{
    ArchiveProjectInput, ArchiveProjectService, CreateProjectInput, CreateProjectService,
    ListProjectsInput, ListProjectsService, SqliteProjectRepository, SqliteStorage,
    UpdateProjectInput, UpdateProjectService,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

#[derive(Clone)]
pub struct ProjectBridgeState {
    repository: Arc<SqliteProjectRepository>,
}

impl ProjectBridgeState {
    pub fn new(repository: Arc<SqliteProjectRepository>) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &Arc<SqliteProjectRepository> {
        &self.repository
    }
}

pub fn bridge_state(storage: &SqliteStorage) -> ProjectBridgeState {
    ProjectBridgeState::new(Arc::new(SqliteProjectRepository::new(
        storage.pool().clone(),
    )))
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectBridgeError {
    pub code: DomainErrorCode,
    pub retryability: Retryability,
    pub message: String,
    pub correlation_id: Option<String>,
}

impl std::fmt::Display for ProjectBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ProjectBridgeError {}

fn bridge_error(error: DomainError, correlation_id: Option<String>) -> ProjectBridgeError {
    let envelope = error.envelope(correlation_id);
    ProjectBridgeError {
        code: envelope.code,
        retryability: envelope.retryability,
        message: envelope.message,
        correlation_id: envelope.correlation_id,
    }
}

fn parse_project_id(
    value: String,
    correlation_id: Option<String>,
) -> Result<ProjectId, ProjectBridgeError> {
    value.parse::<ProjectId>().map_err(|_| {
        bridge_error(
            DomainError::Validation("invalid project id".into()),
            correlation_id,
        )
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSettingsSummary {
    pub retention_days: u32,
    pub auto_archive_idle_days: Option<u32>,
    pub telemetry_enabled: bool,
    pub max_active_agents: u32,
}

impl From<&ProjectSettings> for ProjectSettingsSummary {
    fn from(settings: &ProjectSettings) -> Self {
        Self {
            retention_days: settings.retention_days,
            auto_archive_idle_days: settings.auto_archive_idle_days,
            telemetry_enabled: settings.telemetry_enabled,
            max_active_agents: settings.max_active_agents,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: ProjectSettingsSummary,
}

impl From<&Project> for ProjectSummary {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id,
            name: project.name.clone(),
            description: project.description.clone(),
            status: project.status,
            owner: project.owner.clone(),
            created_at: project.created_at,
            updated_at: project.updated_at,
            settings: ProjectSettingsSummary::from(&project.settings),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProjectCommandInput {
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectCommandOutput {
    pub project: ProjectSummary,
    pub event_id: Option<agent_protocol::ids::EventId>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ListProjectsCommandInput {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<ProjectStatus>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListProjectsCommandOutput {
    pub projects: Vec<ProjectSummary>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProjectCommandInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProjectStatus>,
    pub expected_updated_at: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProjectCommandOutput {
    pub project: ProjectSummary,
    pub event_id: Option<agent_protocol::ids::EventId>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveProjectCommandInput {
    pub id: String,
    pub reason: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveProjectCommandOutput {
    pub project: ProjectSummary,
    pub event_id: Option<agent_protocol::ids::EventId>,
    pub already_archived: bool,
    pub correlation_id: Option<String>,
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, ProjectBridgeState>,
    input: CreateProjectCommandInput,
) -> Result<CreateProjectCommandOutput, ProjectBridgeError> {
    let correlation_id = input.correlation_id.clone();
    let service = CreateProjectService::new(state.repository().clone(), None);
    let output = service
        .execute(CreateProjectInput {
            name: input.name,
            owner: input.owner,
            description: input.description,
            correlation_id,
        })
        .await
        .map_err(|error| bridge_error(error, None))?;
    Ok(CreateProjectCommandOutput {
        project: ProjectSummary::from(&output.project),
        event_id: output.event_id,
        correlation_id: output.correlation_id,
    })
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, ProjectBridgeState>,
    input: Option<ListProjectsCommandInput>,
) -> Result<ListProjectsCommandOutput, ProjectBridgeError> {
    let input = input.unwrap_or_default();
    let correlation_id = input.correlation_id.clone();
    let service = ListProjectsService::new(state.repository().clone());
    let output = service
        .list(ListProjectsInput {
            limit: input.limit,
            offset: input.offset,
            status_filter: input.status,
            correlation_id,
        })
        .await
        .map_err(|error| bridge_error(error, None))?;
    let output_correlation_id = output.correlation_id.clone();
    let mut projects = Vec::with_capacity(output.items.len());
    for item in output.items {
        if let Some(project) = service
            .get_by_id(&item.id)
            .await
            .map_err(|error| bridge_error(error, output_correlation_id.clone()))?
        {
            projects.push(ProjectSummary::from(&project));
        }
    }
    let total = state
        .repository()
        .count()
        .await
        .map_err(|error| bridge_error(error, output_correlation_id.clone()))?;
    Ok(ListProjectsCommandOutput {
        projects,
        total,
        limit: output.limit,
        offset: output.offset,
        correlation_id: output.correlation_id,
    })
}

#[tauri::command]
pub async fn get_project(
    state: State<'_, ProjectBridgeState>,
    id: String,
) -> Result<Option<ProjectSummary>, ProjectBridgeError> {
    let project_id = parse_project_id(id, None)?;
    let service = ListProjectsService::new(state.repository().clone());
    service
        .get_by_id(&project_id)
        .await
        .map(|project| project.as_ref().map(ProjectSummary::from))
        .map_err(|error| bridge_error(error, None))
}

#[tauri::command]
pub async fn update_project(
    state: State<'_, ProjectBridgeState>,
    input: UpdateProjectCommandInput,
) -> Result<UpdateProjectCommandOutput, ProjectBridgeError> {
    let correlation_id = input.correlation_id.clone();
    let id = parse_project_id(input.id, correlation_id.clone())?;
    let expected_updated_at = input
        .expected_updated_at
        .map(|value| DateTime::parse_from_rfc3339(&value).map(|date| date.with_timezone(&Utc)))
        .transpose()
        .map_err(|_| {
            bridge_error(
                DomainError::Validation("invalid expected_updated_at".into()),
                correlation_id.clone(),
            )
        })?;
    let service = UpdateProjectService::new(state.repository().clone(), None);
    let output = service
        .execute(UpdateProjectInput {
            id,
            name: input.name,
            description: input.description,
            status: input.status,
            expected_updated_at,
            correlation_id,
        })
        .await
        .map_err(|error| bridge_error(error, None))?;
    Ok(UpdateProjectCommandOutput {
        project: ProjectSummary::from(&output.project),
        event_id: output.event_id,
        correlation_id: output.correlation_id,
    })
}

#[tauri::command]
pub async fn archive_project(
    state: State<'_, ProjectBridgeState>,
    input: ArchiveProjectCommandInput,
) -> Result<ArchiveProjectCommandOutput, ProjectBridgeError> {
    let correlation_id = input.correlation_id.clone();
    let id = parse_project_id(input.id, correlation_id.clone())?;
    let service = ArchiveProjectService::new(state.repository().clone(), None);
    let output = service
        .execute(ArchiveProjectInput {
            id,
            reason: input.reason,
            correlation_id,
        })
        .await
        .map_err(|error| bridge_error(error, None))?;
    Ok(ArchiveProjectCommandOutput {
        project: ProjectSummary::from(&output.project),
        event_id: output.event_id,
        already_archived: output.already_archived,
        correlation_id: output.correlation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // @spec:AC-112
    fn project_command_inputs_preserve_frontend_contract() {
        let input: CreateProjectCommandInput = serde_json::from_value(serde_json::json!({
            "name": "Hank",
            "owner": "gabriel",
            "description": "desktop",
            "correlation_id": "corr-1"
        }))
        .unwrap();
        assert_eq!(input.name, "Hank");
        assert_eq!(input.correlation_id.as_deref(), Some("corr-1"));
    }

    #[test]
    // @spec:AC-111 @spec:AC-112
    fn invalid_project_ids_fail_closed_without_raw_input() {
        let error = parse_project_id("not-an-id".into(), Some("corr-2".into())).unwrap_err();
        assert_eq!(error.code, DomainErrorCode::Validation);
        assert!(!error.message.contains("not-an-id"));
        assert_eq!(error.correlation_id.as_deref(), Some("corr-2"));
    }
}
