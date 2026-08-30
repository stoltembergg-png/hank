//! Project-scoped Agent read bridge for the desktop shell.
//!
//! The bridge only translates bounded DTOs. Agent validation, project
//! ownership and persistence remain in the runtime application service and
//! repositories initialized from the boot storage pool.

use agent_core::agent::{Agent, AgentStatus, Personality};
use agent_core::error::{DomainError, DomainErrorCode, Retryability};
use agent_core::ids::ProjectId;
use agent_runtime::{
    agent_repo::SqliteAgentRepository, AgentService, CreateAgentInput, SqliteProjectRepository,
    SqliteStorage,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;
const MAX_OFFSET: usize = 10_000;

#[derive(Clone)]
pub struct AgentBridgeState {
    agents: Arc<SqliteAgentRepository>,
    projects: Arc<SqliteProjectRepository>,
}

impl AgentBridgeState {
    pub fn new(agents: Arc<SqliteAgentRepository>, projects: Arc<SqliteProjectRepository>) -> Self {
        Self { agents, projects }
    }
}

pub fn bridge_state(storage: &SqliteStorage) -> AgentBridgeState {
    AgentBridgeState::new(
        Arc::new(SqliteAgentRepository::new(storage.pool().clone())),
        Arc::new(SqliteProjectRepository::new(storage.pool().clone())),
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentBridgeError {
    pub code: DomainErrorCode,
    pub retryability: Retryability,
    pub message: String,
    pub correlation_id: Option<String>,
}

impl std::fmt::Display for AgentBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for AgentBridgeError {}

fn bridge_error(error: DomainError, correlation_id: Option<String>) -> AgentBridgeError {
    let envelope = error.envelope(correlation_id);
    AgentBridgeError {
        code: envelope.code,
        retryability: envelope.retryability,
        message: envelope.message,
        correlation_id: envelope.correlation_id,
    }
}

fn parse_project_id(
    value: String,
    correlation_id: Option<String>,
) -> Result<ProjectId, AgentBridgeError> {
    value.parse::<ProjectId>().map_err(|_| {
        bridge_error(
            DomainError::Validation("invalid project id".into()),
            correlation_id,
        )
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: AgentStatus,
    pub personality: Personality,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Agent> for AgentSummary {
    fn from(agent: &Agent) -> Self {
        Self {
            id: agent.id.to_string(),
            project_id: agent.project_id.to_string(),
            name: agent.name.clone(),
            description: agent.description.clone(),
            status: agent.status,
            personality: agent.personality.clone(),
            created_at: agent.created_at,
            updated_at: agent.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListAgentsCommandInput {
    pub project_id: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListAgentsCommandOutput {
    pub agents: Vec<AgentSummary>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateAgentCommandInput {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateAgentCommandOutput {
    pub agent: AgentSummary,
    pub event_id: Option<agent_protocol::ids::EventId>,
    pub correlation_id: Option<String>,
}

#[tauri::command]
pub async fn list_agents(
    state: State<'_, AgentBridgeState>,
    input: ListAgentsCommandInput,
) -> Result<ListAgentsCommandOutput, AgentBridgeError> {
    list_agents_for_state(&state, input).await
}

async fn list_agents_for_state(
    state: &AgentBridgeState,
    input: ListAgentsCommandInput,
) -> Result<ListAgentsCommandOutput, AgentBridgeError> {
    let correlation_id = input.correlation_id.clone();
    let project_id = parse_project_id(input.project_id, correlation_id.clone())?;
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = input.offset.unwrap_or(0).min(MAX_OFFSET);
    let service = AgentService::new(state.agents.clone(), state.projects.clone(), None);
    let agents = service
        .list(&project_id, limit, offset)
        .await
        .map_err(|error| bridge_error(error, correlation_id.clone()))?;
    let total = state
        .agents
        .count(&project_id)
        .await
        .map_err(|error| bridge_error(error, correlation_id.clone()))?;

    Ok(ListAgentsCommandOutput {
        agents: agents.iter().map(AgentSummary::from).collect(),
        total,
        limit,
        offset,
        correlation_id,
    })
}

#[tauri::command]
pub async fn create_agent(
    state: State<'_, AgentBridgeState>,
    input: CreateAgentCommandInput,
) -> Result<CreateAgentCommandOutput, AgentBridgeError> {
    create_agent_for_state(&state, input).await
}

async fn create_agent_for_state(
    state: &AgentBridgeState,
    input: CreateAgentCommandInput,
) -> Result<CreateAgentCommandOutput, AgentBridgeError> {
    let correlation_id = input.correlation_id.clone();
    let project_id = parse_project_id(input.project_id, correlation_id.clone())?;
    let service = AgentService::new(state.agents.clone(), state.projects.clone(), None);
    let output = service
        .create(CreateAgentInput {
            project_id,
            name: input.name,
            description: input.description,
            policy: Default::default(),
            correlation_id,
        })
        .await
        .map_err(|error| bridge_error(error, input.correlation_id.clone()))?;

    Ok(CreateAgentCommandOutput {
        agent: AgentSummary::from(&output.agent),
        event_id: output.event_id,
        correlation_id: output.correlation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::agent::Agent;
    use agent_core::project::{Project, ProjectRepository};
    use agent_protocol::policy::AgentPolicyConfig;
    use agent_runtime::{migrations::run_migrations, sqlite::SqliteStorage};

    #[tokio::test]
    async fn list_is_project_scoped_and_preserves_pagination_metadata() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let projects = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agents = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));

        let project = Project::create("agents-project", "owner", None).unwrap();
        let other_project = Project::create("other-project", "owner", None).unwrap();
        projects.save(&project).await.unwrap();
        projects.save(&other_project).await.unwrap();

        for name in ["first-agent", "second-agent"] {
            agents
                .save(&Agent::new(
                    project.id,
                    name.into(),
                    AgentPolicyConfig::default(),
                ))
                .await
                .unwrap();
        }
        agents
            .save(&Agent::new(
                other_project.id,
                "other-agent".into(),
                AgentPolicyConfig::default(),
            ))
            .await
            .unwrap();

        let state = AgentBridgeState::new(agents, projects);
        let output = list_agents_for_state(
            &state,
            ListAgentsCommandInput {
                project_id: project.id.to_string(),
                limit: Some(1),
                offset: Some(1),
                correlation_id: Some("corr-agents".into()),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.agents.len(), 1);
        assert_eq!(output.total, 2);
        assert_eq!(output.limit, 1);
        assert_eq!(output.offset, 1);
        assert_eq!(output.correlation_id.as_deref(), Some("corr-agents"));
        assert_eq!(output.agents[0].project_id, project.id.to_string());
        assert_ne!(output.agents[0].name, "other-agent");
    }

    #[tokio::test]
    async fn invalid_project_id_fails_closed_with_safe_error_envelope() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let state = AgentBridgeState::new(
            Arc::new(SqliteAgentRepository::new(storage.pool().clone())),
            Arc::new(SqliteProjectRepository::new(storage.pool().clone())),
        );

        let error = list_agents_for_state(
            &state,
            ListAgentsCommandInput {
                project_id: "not-a-project".into(),
                correlation_id: Some("corr-invalid".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code, DomainErrorCode::Validation);
        assert_eq!(error.correlation_id.as_deref(), Some("corr-invalid"));
        assert!(!error.message.contains("not-a-project"));
    }

    #[tokio::test]
    async fn create_uses_project_scope_and_persists_the_domain_default_policy() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let projects = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agents = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let project = Project::create("create-project", "owner", None).unwrap();
        projects.save(&project).await.unwrap();

        let state = AgentBridgeState::new(agents.clone(), projects);
        let output = create_agent_for_state(
            &state,
            CreateAgentCommandInput {
                project_id: project.id.to_string(),
                name: "release-agent".into(),
                description: Some("Prepara releases com revisão humana.".into()),
                correlation_id: Some("corr-create".into()),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.agent.project_id, project.id.to_string());
        assert_eq!(output.agent.name, "release-agent");
        assert_eq!(
            output.agent.description.as_deref(),
            Some("Prepara releases com revisão humana.")
        );
        assert_eq!(output.correlation_id.as_deref(), Some("corr-create"));
        let persisted = agents
            .get(&project.id, &output.agent.id.parse().unwrap())
            .await
            .unwrap()
            .expect("agent must be persisted in the requested project");
        assert_eq!(persisted.name, "release-agent");
    }
}
