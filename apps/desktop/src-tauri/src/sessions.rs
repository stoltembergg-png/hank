//! Project/Agent-scoped Session lifecycle bridge for the desktop shell.
//!
//! Only bounded session metadata crosses the Tauri boundary. Provider
//! selection, turn execution and message streaming remain outside this
//! increment; the lifecycle service persists real sessions in SQLite.

use agent_core::agent::{Agent, AgentStatus};
use agent_core::error::{DomainError, DomainErrorCode, Retryability};
use agent_core::ids::{AgentId, ProjectId};
use agent_core::project::{Project, ProjectRepository, ProjectStatus};
use agent_core::session::{Session, SessionStatus};
use agent_runtime::agent_repo::SqliteAgentRepository;
use agent_runtime::project_repo::SqliteProjectRepository;
use agent_runtime::session_repo::{SessionStorageError, SqliteSessionRepository};
use agent_runtime::session_service::{SessionApplicationService, SessionServiceError};
use agent_runtime::SqliteStorage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;
const MAX_OFFSET: usize = 10_000;
const MAX_TITLE_BYTES: usize = 256;

#[derive(Clone)]
pub struct SessionBridgeState {
    sessions: Arc<SqliteSessionRepository>,
    projects: Arc<SqliteProjectRepository>,
    agents: Arc<SqliteAgentRepository>,
}

impl SessionBridgeState {
    pub fn new(
        sessions: Arc<SqliteSessionRepository>,
        projects: Arc<SqliteProjectRepository>,
        agents: Arc<SqliteAgentRepository>,
    ) -> Self {
        Self {
            sessions,
            projects,
            agents,
        }
    }
}

pub fn bridge_state(storage: &SqliteStorage) -> SessionBridgeState {
    let pool = storage.pool().clone();
    SessionBridgeState::new(
        Arc::new(SqliteSessionRepository::new(pool.clone())),
        Arc::new(SqliteProjectRepository::new(pool.clone())),
        Arc::new(SqliteAgentRepository::new(pool)),
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionBridgeError {
    pub code: DomainErrorCode,
    pub retryability: Retryability,
    pub message: String,
    pub correlation_id: Option<String>,
}

impl std::fmt::Display for SessionBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for SessionBridgeError {}

fn bridge_error(error: DomainError, correlation_id: Option<String>) -> SessionBridgeError {
    let envelope = error.envelope(correlation_id);
    SessionBridgeError {
        code: envelope.code,
        retryability: envelope.retryability,
        message: envelope.message,
        correlation_id: envelope.correlation_id,
    }
}

fn session_error(
    error: SessionServiceError,
    correlation_id: Option<String>,
) -> SessionBridgeError {
    let domain_error = match error {
        SessionServiceError::Storage(error) => match error {
            SessionStorageError::NotFound => DomainError::NotFound("session".into()),
            SessionStorageError::ScopeMismatch => DomainError::PermissionDenied {
                capability: "session.read".into(),
                reason: "session is outside the requested scope".into(),
            },
            SessionStorageError::Conflict => DomainError::ConcurrencyConflict {
                expected: "current session version".into(),
                actual: "stale session version".into(),
            },
            SessionStorageError::Invalid => {
                DomainError::Validation("invalid session request".into())
            }
            SessionStorageError::Serialization(_) | SessionStorageError::Database(_) => {
                DomainError::InvariantViolation("session persistence failed".into())
            }
        },
        SessionServiceError::MessageStorage(_) => {
            DomainError::InvariantViolation("session message persistence failed".into())
        }
        SessionServiceError::Unauthorized => DomainError::PermissionDenied {
            capability: "session.access".into(),
            reason: "session is outside the requested scope".into(),
        },
        SessionServiceError::Invalid => DomainError::Validation("invalid session request".into()),
        SessionServiceError::State => DomainError::InvalidStateTransition {
            from: "current".into(),
            to: "requested".into(),
        },
        SessionServiceError::SessionClosed => DomainError::InvalidStateTransition {
            from: "closed".into(),
            to: "active".into(),
        },
        SessionServiceError::Cancelled => DomainError::InvariantViolation(
            "session lifecycle operation was cancelled".into(),
        ),
        SessionServiceError::ProviderFailure => {
            DomainError::CapabilityUnavailable("session turn execution".into())
        }
        SessionServiceError::Budget => DomainError::BudgetExceeded {
            budget_type: "session".into(),
            limit: "configured".into(),
            used: "exceeded".into(),
        },
        SessionServiceError::Concurrency => {
            DomainError::InvariantViolation("session concurrency limit reached".into())
        }
    };
    bridge_error(domain_error, correlation_id)
}

fn parse_project_id(
    value: String,
    correlation_id: Option<String>,
) -> Result<ProjectId, SessionBridgeError> {
    value.parse::<ProjectId>().map_err(|_| {
        bridge_error(
            DomainError::Validation("invalid project id".into()),
            correlation_id,
        )
    })
}

fn parse_agent_id(
    value: String,
    correlation_id: Option<String>,
) -> Result<AgentId, SessionBridgeError> {
    value.parse::<AgentId>().map_err(|_| {
        bridge_error(
            DomainError::Validation("invalid agent id".into()),
            correlation_id,
        )
    })
}

fn bounded_title(
    title: Option<String>,
    correlation_id: Option<String>,
) -> Result<Option<String>, SessionBridgeError> {
    let Some(title) = title else { return Ok(None) };
    let title = title.trim();
    if title.is_empty() {
        return Ok(None);
    }
    if title.len() > MAX_TITLE_BYTES || title.chars().any(char::is_control) {
        return Err(bridge_error(
            DomainError::Validation("session title is invalid or oversized".into()),
            correlation_id,
        ));
    }
    Ok(Some(title.to_string()))
}

async fn load_scope(
    state: &SessionBridgeState,
    project_id: &ProjectId,
    agent_id: &AgentId,
    correlation_id: Option<String>,
) -> Result<(Project, Agent), SessionBridgeError> {
    let project = state
        .projects
        .get_by_id(project_id)
        .await
        .map_err(|error| bridge_error(error, correlation_id.clone()))?
        .ok_or_else(|| bridge_error(DomainError::NotFound("project".into()), correlation_id.clone()))?;
    let agent = state
        .agents
        .get(project_id, agent_id)
        .await
        .map_err(|error| bridge_error(error, correlation_id.clone()))?
        .ok_or_else(|| bridge_error(DomainError::NotFound("agent".into()), correlation_id))?;
    Ok((project, agent))
}

fn require_creation_scope(
    project: &Project,
    agent: &Agent,
    correlation_id: Option<String>,
) -> Result<(), SessionBridgeError> {
    if project.status != ProjectStatus::Active {
        return Err(bridge_error(
            DomainError::InvalidStateTransition {
                from: "project_unavailable".into(),
                to: "session_active".into(),
            },
            correlation_id,
        ));
    }
    if agent.status != AgentStatus::Active {
        return Err(bridge_error(
            DomainError::PermissionDenied {
                capability: "session.create".into(),
                reason: "agent is not active".into(),
            },
            correlation_id,
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionSummary {
    pub id: String,
    pub project_id: String,
    pub agent_id: String,
    pub status: SessionStatus,
    pub title: Option<String>,
    pub message_count: usize,
    pub token_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.to_string(),
            project_id: session.project_id.to_string(),
            agent_id: session.agent_id.to_string(),
            status: session.status,
            title: session.title.clone(),
            message_count: session.message_count,
            token_count: session.token_count,
            created_at: session.created_at,
            updated_at: session.updated_at,
            closed_at: session.closed_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListSessionsCommandInput {
    pub project_id: String,
    pub agent_id: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListSessionsCommandOutput {
    pub sessions: Vec<SessionSummary>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSessionCommandInput {
    pub project_id: String,
    pub agent_id: String,
    pub title: Option<String>,
    pub correlation_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateSessionCommandOutput {
    pub session: SessionSummary,
    pub correlation_id: String,
}

#[tauri::command]
pub async fn list_sessions(
    state: State<'_, SessionBridgeState>,
    input: ListSessionsCommandInput,
) -> Result<ListSessionsCommandOutput, SessionBridgeError> {
    list_sessions_for_state(&state, input).await
}

async fn list_sessions_for_state(
    state: &SessionBridgeState,
    input: ListSessionsCommandInput,
) -> Result<ListSessionsCommandOutput, SessionBridgeError> {
    let correlation_id = input.correlation_id.clone();
    let project_id = parse_project_id(input.project_id, correlation_id.clone())?;
    let agent_id = parse_agent_id(input.agent_id, correlation_id.clone())?;
    let _ = load_scope(state, &project_id, &agent_id, correlation_id.clone()).await?;
    let limit = input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = input.offset.unwrap_or(0).min(MAX_OFFSET);
    let sessions = state
        .sessions
        .list_for_agent(&project_id, &agent_id, offset as u32, limit as u32)
        .await
        .map_err(|error| session_error(error.into(), correlation_id.clone()))?;
    let total = state
        .sessions
        .count_for_agent(&project_id, &agent_id)
        .await
        .map_err(|error| session_error(error.into(), correlation_id.clone()))?;

    Ok(ListSessionsCommandOutput {
        sessions: sessions.iter().map(SessionSummary::from).collect(),
        total,
        limit,
        offset,
        correlation_id,
    })
}

#[tauri::command]
pub async fn create_session(
    state: State<'_, SessionBridgeState>,
    input: CreateSessionCommandInput,
) -> Result<CreateSessionCommandOutput, SessionBridgeError> {
    create_session_for_state(&state, input).await
}

async fn create_session_for_state(
    state: &SessionBridgeState,
    input: CreateSessionCommandInput,
) -> Result<CreateSessionCommandOutput, SessionBridgeError> {
    let correlation_id = Some(input.correlation_id.clone());
    let project_id = parse_project_id(input.project_id, correlation_id.clone())?;
    let agent_id = parse_agent_id(input.agent_id, correlation_id.clone())?;
    let (project, agent) = load_scope(state, &project_id, &agent_id, correlation_id.clone()).await?;
    require_creation_scope(&project, &agent, correlation_id.clone())?;
    let title = bounded_title(input.title, correlation_id.clone())?;
    let service = SessionApplicationService::new_lifecycle_from_repository(
        (*state.sessions).clone(),
        1,
    )
        .map_err(|error| session_error(error, correlation_id.clone()))?;
    let session = service
        .create(project_id, agent_id, &input.correlation_id, title)
        .await
        .map_err(|error| session_error(error, correlation_id.clone()))?;

    Ok(CreateSessionCommandOutput {
        session: SessionSummary::from(&session),
        correlation_id: input.correlation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_core::agent::Agent;
    use agent_core::project::Project;
    use agent_protocol::policy::AgentPolicyConfig;
    use agent_runtime::migrations::run_migrations;

    async fn state() -> (SqliteStorage, SessionBridgeState, Project, Agent) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let projects = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agents = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let sessions = Arc::new(SqliteSessionRepository::new(storage.pool().clone()));
        let project = Project::create("session-project", "owner", None).unwrap();
        projects.save(&project).await.unwrap();
        let agent = Agent::new(
            project.id,
            "session-agent".into(),
            AgentPolicyConfig::default(),
        );
        agents.save(&agent).await.unwrap();
        (
            storage,
            SessionBridgeState::new(sessions, projects, agents),
            project,
            agent,
        )
    }

    #[tokio::test]
    async fn creates_and_lists_only_the_requested_project_agent_scope() {
        let (storage, bridge, project, agent) = state().await;
        let first = create_session_for_state(
            &bridge,
            CreateSessionCommandInput {
                project_id: project.id.to_string(),
                agent_id: agent.id.to_string(),
                title: Some("First conversation".into()),
                correlation_id: "corr-session-1".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(first.session.status, SessionStatus::Active);

        let second = create_session_for_state(
            &bridge,
            CreateSessionCommandInput {
                project_id: project.id.to_string(),
                agent_id: agent.id.to_string(),
                title: None,
                correlation_id: "corr-session-2".into(),
            },
        )
        .await
        .unwrap();
        assert_ne!(first.session.id, second.session.id);

        let output = list_sessions_for_state(
            &bridge,
            ListSessionsCommandInput {
                project_id: project.id.to_string(),
                agent_id: agent.id.to_string(),
                limit: Some(1),
                offset: Some(1),
                correlation_id: Some("corr-list".into()),
            },
        )
        .await
        .unwrap();
        assert_eq!(output.total, 2);
        assert_eq!(output.sessions.len(), 1);
        assert_eq!(output.offset, 1);
        assert_eq!(output.limit, 1);

        let other_agent = Agent::new(
            project.id,
            "other-session-agent".into(),
            AgentPolicyConfig::default(),
        );
        bridge.agents.save(&other_agent).await.unwrap();
        create_session_for_state(
            &bridge,
            CreateSessionCommandInput {
                project_id: project.id.to_string(),
                agent_id: other_agent.id.to_string(),
                title: Some("Other agent conversation".into()),
                correlation_id: "corr-session-other-agent".into(),
            },
        )
        .await
        .unwrap();
        let scoped = list_sessions_for_state(
            &bridge,
            ListSessionsCommandInput {
                project_id: project.id.to_string(),
                agent_id: agent.id.to_string(),
                limit: Some(100),
                offset: Some(0),
                correlation_id: Some("corr-list-scoped".into()),
            },
        )
        .await
        .unwrap();
        assert_eq!(scoped.total, 2);
        assert!(scoped
            .sessions
            .iter()
            .all(|session| session.agent_id == agent.id.to_string()));
        storage.close().await;
    }

    #[tokio::test]
    async fn rejects_unavailable_creation_scope_and_unsafe_titles() {
        let (storage, bridge, project, agent) = state().await;
        let mut inactive = agent.clone();
        inactive.status = AgentStatus::Inactive;
        bridge.agents.update(&inactive).await.unwrap();
        let error = create_session_for_state(
            &bridge,
            CreateSessionCommandInput {
                project_id: project.id.to_string(),
                agent_id: agent.id.to_string(),
                title: None,
                correlation_id: "corr-inactive".into(),
            },
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, DomainErrorCode::PermissionDenied);

        let active = Agent {
            status: AgentStatus::Active,
            ..inactive
        };
        bridge.agents.update(&active).await.unwrap();
        let error = create_session_for_state(
            &bridge,
            CreateSessionCommandInput {
                project_id: project.id.to_string(),
                agent_id: agent.id.to_string(),
                title: Some("unsafe\n title".into()),
                correlation_id: "corr-title".into(),
            },
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, DomainErrorCode::Validation);
        storage.close().await;
    }
}
