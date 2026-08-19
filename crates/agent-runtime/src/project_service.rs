//! Serviço de aplicação para o use case de criação de projetos.
//!
//! Conforme PR-029 e regras de integridade e publicação transacional de eventos.

use crate::event_bus::EventBus;
use agent_core::error::DomainError;
use agent_core::project::{Project, ProjectRepository};
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_protocol::ids::EventId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// DTO de entrada para a criação de um novo projeto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub correlation_id: Option<String>,
}

/// DTO de saída resultante da criação do projeto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectOutput {
    pub project: Project,
    pub event_id: Option<EventId>,
    pub correlation_id: Option<String>,
}

/// Serviço de aplicação para criação e orquestração de Projetos.
pub struct CreateProjectService<R: ProjectRepository> {
    repository: Arc<R>,
    event_bus: Option<EventBus<ApplicationEvent>>,
}

impl<R: ProjectRepository> CreateProjectService<R> {
    pub fn new(repository: Arc<R>, event_bus: Option<EventBus<ApplicationEvent>>) -> Self {
        Self {
            repository,
            event_bus,
        }
    }

    /// Executa o use case de criação de projeto:
    /// 1. Cria a entidade Project com validações estritas de domínio.
    /// 2. Persiste o projeto no repositório transacional.
    /// 3. Publica o evento ProjectCreated somente após sucesso da persistência.
    pub async fn execute(
        &self,
        input: CreateProjectInput,
    ) -> Result<CreateProjectOutput, DomainError> {
        let project = Project::create(input.name, input.owner, input.description)?;

        self.repository.save(&project).await?;

        let mut emitted_event_id = None;

        if let Some(ref bus) = self.event_bus {
            let payload = serde_json::json!({
                "name": project.name,
                "owner": project.owner,
                "status": project.status,
            })
            .to_string();

            let event_id = EventId::new();
            let event = ApplicationEvent {
                schema_version: 1,
                event_id,
                event_type: EventKind::ProjectCreated,
                project_id: project.id,
                aggregate_id: project.id.to_string(),
                agent_id: None,
                session_id: None,
                occurred_at: Utc::now(),
                sequence: 1,
                payload,
            };

            // Publica no barramento; se não houver subscribers ativos, não falha a criação
            let _ = bus.publish(event);
            emitted_event_id = Some(event_id);
        }

        Ok(CreateProjectOutput {
            project,
            event_id: emitted_event_id,
            correlation_id: input.correlation_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use crate::project_repo::SqliteProjectRepository;
    use crate::sqlite::SqliteStorage;

    async fn setup_service(
        with_bus: bool,
    ) -> (
        CreateProjectService<SqliteProjectRepository>,
        Option<EventBus<ApplicationEvent>>,
    ) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));

        if with_bus {
            let bus = EventBus::bounded(16);
            let service = CreateProjectService::new(repo, Some(bus.clone()));
            (service, Some(bus))
        } else {
            let service = CreateProjectService::new(repo, None);
            (service, None)
        }
    }

    #[tokio::test]
    async fn create_project_persists_and_emits_event() {
        let (service, bus) = setup_service(true).await;
        let bus = bus.unwrap();
        let mut receiver = bus.subscribe();

        let input = CreateProjectInput {
            name: "Hank App".into(),
            owner: "gabriel".into(),
            description: Some("Description".into()),
            correlation_id: Some("req-123".into()),
        };

        let output = service.execute(input).await.unwrap();
        assert_eq!(output.project.name, "Hank App");
        assert_eq!(output.correlation_id.as_deref(), Some("req-123"));
        assert!(output.event_id.is_some());

        // Verifica evento recebido no barramento
        let received_event = receiver.recv().await.unwrap();
        assert_eq!(received_event.event_type, EventKind::ProjectCreated);
        assert_eq!(received_event.project_id, output.project.id);
        assert_eq!(received_event.event_id, output.event_id.unwrap());
    }

    #[tokio::test]
    async fn create_project_with_invalid_name_fails_and_does_not_emit_event() {
        let (service, bus) = setup_service(true).await;
        let bus = bus.unwrap();
        let mut receiver = bus.subscribe();

        let input = CreateProjectInput {
            name: "".into(), // Inválido
            owner: "gabriel".into(),
            description: None,
            correlation_id: None,
        };

        let err = service.execute(input).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));

        // Nenhum evento falso deve ser emitido
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn create_project_without_event_bus_succeeds() {
        let (service, _) = setup_service(false).await;

        let input = CreateProjectInput {
            name: "Hank NoBus".into(),
            owner: "gabriel".into(),
            description: None,
            correlation_id: None,
        };

        let output = service.execute(input).await.unwrap();
        assert_eq!(output.project.name, "Hank NoBus");
        assert!(output.event_id.is_none());
    }
}
