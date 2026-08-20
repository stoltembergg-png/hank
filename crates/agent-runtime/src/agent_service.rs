//! Serviço de aplicação para o use case de criação de Agents.
//!
//! Conforme PR-048 e regras de integridade e publicação transacional de eventos.

use crate::event_bus::EventBus;
use agent_core::agent::{Agent, AgentStatus};
use agent_core::error::DomainError;
use agent_core::ids::{AgentId, ProjectId};
use agent_core::project::ProjectRepository;
use agent_core::AgentRepository;
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_protocol::ids::EventId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// DTO de entrada para a criação de um novo Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentInput {
    pub project_id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub policy: agent_protocol::policy::AgentPolicyConfig,
    pub correlation_id: Option<String>,
}

/// DTO de entrada para atualização de um Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentInput {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<AgentStatus>,
    pub personality: Option<agent_core::agent::Personality>,
    pub policy: Option<agent_protocol::policy::AgentPolicyConfig>,
    pub expected_version: String, // optimistic version check
    pub correlation_id: Option<String>,
}

/// DTO de entrada para arquivamento de um Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveAgentInput {
    pub project_id: ProjectId,
    pub agent_id: AgentId,
    pub expected_version: String, // optimistic version check
    pub confirmation: String,     // explicit confirmation required
    pub correlation_id: Option<String>,
}

/// DTO de saída resultante das operações de Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub agent: Agent,
    pub event_id: Option<EventId>,
    pub correlation_id: Option<String>,
}

/// Serviço de aplicação para operações CRUD de Agents.
pub struct AgentService<R: AgentRepository, P: ProjectRepository> {
    agent_repository: Arc<R>,
    project_repository: Arc<P>,
    event_bus: Option<EventBus<ApplicationEvent>>,
}

impl<R: AgentRepository, P: ProjectRepository> AgentService<R, P> {
    pub fn new(
        agent_repository: Arc<R>,
        project_repository: Arc<P>,
        event_bus: Option<EventBus<ApplicationEvent>>,
    ) -> Self {
        Self {
            agent_repository,
            project_repository,
            event_bus,
        }
    }

    /// Cria um novo Agent.
    pub async fn create(&self, input: CreateAgentInput) -> Result<AgentOutput, DomainError> {
        // Validate project exists
        self.project_repository
            .get(&input.project_id)
            .await?
            .ok_or(DomainError::NotFound("project not found".into()))?;

        let mut agent = Agent::new(input.project_id, input.name, input.policy);
        if let Some(description) = input.description {
            agent.description = Some(description);
        }
        agent.validate()?;

        self.agent_repository.save(&agent).await?;

        let mut emitted_event_id = None;

        if let Some(ref bus) = self.event_bus {
            let payload = serde_json::json!({
                "project_id": agent.project_id.to_string(),
                "name": agent.name,
                "status": format!("{:?}", agent.status),
            })
            .to_string();

            let event_id = EventId::new();
            let event = ApplicationEvent {
                schema_version: 1,
                event_id,
                event_type: EventKind::AgentCreated,
                project_id: agent.project_id,
                aggregate_id: agent.id.to_string(),
                agent_id: Some(agent.id),
                session_id: None,
                occurred_at: Utc::now(),
                sequence: 1,
                payload,
            };

            let _ = bus.publish(event);
            emitted_event_id = Some(event_id);
        }

        Ok(AgentOutput {
            agent,
            event_id: emitted_event_id,
            correlation_id: input.correlation_id,
        })
    }

    /// Recupera um Agent por ID.
    pub async fn get(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
    ) -> Result<Option<Agent>, DomainError> {
        self.agent_repository.get(project_id, agent_id).await
    }

    /// Lista Agents de um Project com paginação.
    pub async fn list(
        &self,
        project_id: &ProjectId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Agent>, DomainError> {
        // Validate project exists
        self.project_repository
            .get(project_id)
            .await?
            .ok_or(DomainError::NotFound("project not found".into()))?;

        let limit = limit.min(100);
        self.agent_repository.list(project_id, limit, offset).await
    }

    /// Atualiza um Agent com versionamento otimista.
    pub async fn update(&self, input: UpdateAgentInput) -> Result<AgentOutput, DomainError> {
        let mut agent = self
            .agent_repository
            .get(&input.project_id, &input.agent_id)
            .await?
            .ok_or(DomainError::NotFound("agent not found in project".into()))?;

        // Optimistic version check
        if agent.updated_at.to_rfc3339() != input.expected_version {
            return Err(DomainError::ConcurrencyConflict {
                expected: input.expected_version,
                actual: agent.updated_at.to_rfc3339(),
            });
        }

        if let Some(name) = input.name {
            let name = name.trim();
            if name.is_empty() || name.len() > 120 {
                return Err(DomainError::Validation(
                    "agent name is empty or oversized".into(),
                ));
            }
            agent.name = name.to_string();
        }

        if let Some(description) = input.description {
            if description.len() > 4000 {
                return Err(DomainError::Validation(
                    "agent description is oversized".into(),
                ));
            }
            agent.description = Some(description);
        }

        if let Some(status) = input.status {
            agent.status = status;
        }

        if let Some(personality) = input.personality {
            personality.validate()?;
            agent.personality = personality;
        }

        if let Some(policy) = input.policy {
            agent.policy = policy;
        }

        agent.updated_at = Utc::now();
        agent.validate()?;

        self.agent_repository.update(&agent).await?;

        let mut emitted_event_id = None;

        if let Some(ref bus) = self.event_bus {
            let payload = serde_json::json!({
                "project_id": agent.project_id.to_string(),
                "name": agent.name,
                "status": format!("{:?}", agent.status),
            })
            .to_string();

            let event_id = EventId::new();
            let event = ApplicationEvent {
                schema_version: 1,
                event_id,
                event_type: EventKind::AgentUpdated,
                project_id: agent.project_id,
                aggregate_id: agent.id.to_string(),
                agent_id: Some(agent.id),
                session_id: None,
                occurred_at: Utc::now(),
                sequence: 2,
                payload,
            };

            let _ = bus.publish(event);
            emitted_event_id = Some(event_id);
        }

        Ok(AgentOutput {
            agent,
            event_id: emitted_event_id,
            correlation_id: input.correlation_id,
        })
    }

    /// Arquiva um Agent (estado terminal) com confirmação explícita.
    pub async fn archive(&self, input: ArchiveAgentInput) -> Result<AgentOutput, DomainError> {
        let mut agent = self
            .agent_repository
            .get(&input.project_id, &input.agent_id)
            .await?
            .ok_or(DomainError::NotFound("agent not found in project".into()))?;

        // Optimistic version check
        if agent.updated_at.to_rfc3339() != input.expected_version {
            return Err(DomainError::ConcurrencyConflict {
                expected: input.expected_version,
                actual: agent.updated_at.to_rfc3339(),
            });
        }

        // Explicit confirmation required
        if input.confirmation != "confirm archive" {
            return Err(DomainError::Validation(
                "archive requires explicit confirmation".into(),
            ));
        }

        if agent.status == AgentStatus::Inactive {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", agent.status),
                to: "Inactive".into(),
            });
        }

        agent.status = AgentStatus::Inactive;
        agent.updated_at = Utc::now();

        self.agent_repository.update(&agent).await?;

        let mut emitted_event_id = None;

        if let Some(ref bus) = self.event_bus {
            let payload = serde_json::json!({
                "project_id": agent.project_id.to_string(),
                "name": agent.name,
            })
            .to_string();

            let event_id = EventId::new();
            let event = ApplicationEvent {
                schema_version: 1,
                event_id,
                event_type: EventKind::AgentArchived,
                project_id: agent.project_id,
                aggregate_id: agent.id.to_string(),
                agent_id: Some(agent.id),
                session_id: None,
                occurred_at: Utc::now(),
                sequence: 3,
                payload,
            };

            let _ = bus.publish(event);
            emitted_event_id = Some(event_id);
        }

        Ok(AgentOutput {
            agent,
            event_id: emitted_event_id,
            correlation_id: input.correlation_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_repo::SqliteAgentRepository;
    use crate::event_bus::EventBus;
    use crate::migrations::run_migrations;
    use crate::project_repo::SqliteProjectRepository;
    use crate::sqlite::SqliteStorage;
    use agent_core::ids::ProjectId;

    #[tokio::test]
    async fn create_agent_persists_and_emits_event() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        // Create project first
        let project = agent_core::project::Project::create("test-project", "owner", None).unwrap();
        project_repo.save(&project).await.unwrap();

        let service = AgentService::new(agent_repo, project_repo, Some(bus));
        let input = CreateAgentInput {
            project_id: project.id,
            name: "worker".into(),
            description: Some("A test agent".into()),
            policy: agent_protocol::policy::AgentPolicyConfig::default(),
            correlation_id: Some("corr-1".into()),
        };

        let output = service.create(input).await.unwrap();

        assert_eq!(output.agent.name, "worker");
        assert_eq!(output.agent.project_id, project.id);
        assert!(output.event_id.is_some());
        assert_eq!(output.correlation_id, Some("corr-1".into()));
    }

    #[tokio::test]
    async fn create_agent_without_project_fails() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        let service = AgentService::new(agent_repo, project_repo, Some(bus));
        let input = CreateAgentInput {
            project_id: ProjectId::new(),
            name: "worker".into(),
            description: None,
            policy: agent_protocol::policy::AgentPolicyConfig::default(),
            correlation_id: None,
        };

        let result = service.create(input).await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn get_agent_returns_exact_agent() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        let project = agent_core::project::Project::create("test-project", "owner", None).unwrap();
        project_repo.save(&project).await.unwrap();

        let service = AgentService::new(agent_repo.clone(), project_repo, Some(bus));
        let input = CreateAgentInput {
            project_id: project.id,
            name: "worker".into(),
            description: None,
            policy: agent_protocol::policy::AgentPolicyConfig::default(),
            correlation_id: None,
        };
        let output = service.create(input).await.unwrap();

        let fetched = service.get(&project.id, &output.agent.id).await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, output.agent.id);
        assert_eq!(fetched.name, "worker");
    }

    #[tokio::test]
    async fn list_agents_paginated() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        let project = agent_core::project::Project::create("test-project", "owner", None).unwrap();
        project_repo.save(&project).await.unwrap();

        let service = AgentService::new(agent_repo.clone(), project_repo, Some(bus));

        for i in 0..5 {
            let input = CreateAgentInput {
                project_id: project.id,
                name: format!("worker-{}", i),
                description: None,
                policy: agent_protocol::policy::AgentPolicyConfig::default(),
                correlation_id: None,
            };
            service.create(input).await.unwrap();
        }

        let page1 = service.list(&project.id, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = service.list(&project.id, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = service.list(&project.id, 2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);
    }

    #[tokio::test]
    async fn update_agent_optimistic_version() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        let project = agent_core::project::Project::create("test-project", "owner", None).unwrap();
        project_repo.save(&project).await.unwrap();

        let service = AgentService::new(agent_repo.clone(), project_repo, Some(bus));
        let input = CreateAgentInput {
            project_id: project.id,
            name: "worker".into(),
            description: None,
            policy: agent_protocol::policy::AgentPolicyConfig::default(),
            correlation_id: None,
        };
        let output = service.create(input).await.unwrap();

        // Successful update with correct version
        let update = UpdateAgentInput {
            project_id: project.id,
            agent_id: output.agent.id,
            name: Some("updated-worker".into()),
            description: None,
            status: None,
            personality: None,
            policy: None,
            expected_version: output.agent.updated_at.to_rfc3339(),
            correlation_id: None,
        };
        let updated = service.update(update).await.unwrap();
        assert_eq!(updated.agent.name, "updated-worker");

        // Stale version fails
        let stale_update = UpdateAgentInput {
            project_id: project.id,
            agent_id: output.agent.id,
            name: Some("stale-worker".into()),
            description: None,
            status: None,
            personality: None,
            policy: None,
            expected_version: output.agent.updated_at.to_rfc3339(), // old version
            correlation_id: None,
        };
        let result = service.update(stale_update).await;
        assert!(matches!(
            result,
            Err(DomainError::ConcurrencyConflict { .. })
        ));
    }

    #[tokio::test]
    async fn archive_agent_requires_confirmation() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        let project = agent_core::project::Project::create("test-project", "owner", None).unwrap();
        project_repo.save(&project).await.unwrap();

        let service = AgentService::new(agent_repo.clone(), project_repo, Some(bus));
        let input = CreateAgentInput {
            project_id: project.id,
            name: "worker".into(),
            description: None,
            policy: agent_protocol::policy::AgentPolicyConfig::default(),
            correlation_id: None,
        };
        let output = service.create(input).await.unwrap();

        // Archive without confirmation fails
        let archive = ArchiveAgentInput {
            project_id: project.id,
            agent_id: output.agent.id,
            expected_version: output.agent.updated_at.to_rfc3339(),
            confirmation: "wrong".into(),
            correlation_id: None,
        };
        let result = service.archive(archive).await;
        assert!(matches!(result, Err(DomainError::Validation(_))));

        // Archive with correct confirmation succeeds
        let archive = ArchiveAgentInput {
            project_id: project.id,
            agent_id: output.agent.id,
            expected_version: output.agent.updated_at.to_rfc3339(),
            confirmation: "confirm archive".into(),
            correlation_id: None,
        };
        let archived = service.archive(archive).await.unwrap();
        assert_eq!(archived.agent.status, AgentStatus::Inactive);
    }

    #[tokio::test]
    async fn archive_already_inactive_agent_is_idempotent() {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();

        let project_repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let agent_repo = Arc::new(SqliteAgentRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);

        let project = agent_core::project::Project::create("test-project", "owner", None).unwrap();
        project_repo.save(&project).await.unwrap();

        let service = AgentService::new(agent_repo.clone(), project_repo, Some(bus));
        let input = CreateAgentInput {
            project_id: project.id,
            name: "worker".into(),
            description: None,
            policy: agent_protocol::policy::AgentPolicyConfig::default(),
            correlation_id: None,
        };
        let output = service.create(input).await.unwrap();

        // First archive
        let archive = ArchiveAgentInput {
            project_id: project.id,
            agent_id: output.agent.id,
            expected_version: output.agent.updated_at.to_rfc3339(),
            confirmation: "confirm archive".into(),
            correlation_id: None,
        };
        let archived = service.archive(archive).await.unwrap();
        assert_eq!(archived.agent.status, AgentStatus::Inactive);

        // Second archive fails (already inactive)
        let archive2 = ArchiveAgentInput {
            project_id: project.id,
            agent_id: output.agent.id,
            expected_version: archived.agent.updated_at.to_rfc3339(),
            confirmation: "confirm archive".into(),
            correlation_id: None,
        };
        let result = service.archive(archive2).await;
        assert!(matches!(
            result,
            Err(DomainError::InvalidStateTransition { .. })
        ));
    }
}
