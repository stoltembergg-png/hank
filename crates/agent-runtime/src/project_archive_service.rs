//! Serviço de aplicação para arquivamento de Projetos.
//!
//! Conforme PR-032 e regras de integridade, idempotência e publicação transacional.

use crate::event_bus::EventBus;
use agent_core::error::DomainError;
use agent_core::ids::ProjectId;
use agent_core::project::{Project, ProjectRepository, ProjectStatus};
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_protocol::ids::EventId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// DTO de entrada para arquivamento de projeto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveProjectInput {
    pub id: ProjectId,
    pub reason: Option<String>,
    pub correlation_id: Option<String>,
}

/// DTO de saída resultante do arquivamento do projeto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveProjectOutput {
    pub project: Project,
    pub event_id: Option<EventId>,
    pub already_archived: bool,
    pub correlation_id: Option<String>,
}

/// Serviço de aplicação para arquivamento seguro de projetos (soft/state-only).
pub struct ArchiveProjectService<R: ProjectRepository> {
    repository: Arc<R>,
    event_bus: Option<EventBus<ApplicationEvent>>,
}

impl<R: ProjectRepository> ArchiveProjectService<R> {
    pub fn new(repository: Arc<R>, event_bus: Option<EventBus<ApplicationEvent>>) -> Self {
        Self {
            repository,
            event_bus,
        }
    }

    /// Executa o arquivamento do projeto:
    /// 1. Busca o projeto pelo ID
    /// 2. Se já arquivado, retorna sucesso idempotente sem republicar evento
    /// 3. Transiciona para ProjectStatus::Archived
    /// 4. Persiste no repositório
    /// 5. Publica o evento ProjectArchived no barramento
    pub async fn execute(
        &self,
        input: ArchiveProjectInput,
    ) -> Result<ArchiveProjectOutput, DomainError> {
        let mut project = self.repository.get_by_id(&input.id).await?.ok_or_else(|| {
            DomainError::NotFound(format!("projeto não encontrado: {}", input.id))
        })?;

        if project.status == ProjectStatus::Archived {
            return Ok(ArchiveProjectOutput {
                project,
                event_id: None,
                already_archived: true,
                correlation_id: input.correlation_id,
            });
        }

        project.archive()?;
        self.repository.update(&project).await?;

        let mut emitted_event_id = None;

        if let Some(ref bus) = self.event_bus {
            let payload = serde_json::json!({
                "project_id": project.id.to_string(),
                "reason": input.reason,
            })
            .to_string();

            let event_id = EventId::new();
            let event = ApplicationEvent {
                schema_version: 1,
                event_id,
                event_type: EventKind::ProjectArchived,
                project_id: project.id,
                aggregate_id: project.id.to_string(),
                agent_id: None,
                session_id: None,
                occurred_at: Utc::now(),
                sequence: 1,
                payload,
            };

            let _ = bus.publish(event);
            emitted_event_id = Some(event_id);
        }

        Ok(ArchiveProjectOutput {
            project,
            event_id: emitted_event_id,
            already_archived: false,
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

    async fn setup_service() -> (
        ArchiveProjectService<SqliteProjectRepository>,
        EventBus<ApplicationEvent>,
        Arc<SqliteProjectRepository>,
    ) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);
        let service = ArchiveProjectService::new(repo.clone(), Some(bus.clone()));
        (service, bus, repo)
    }

    #[tokio::test]
    async fn archive_active_project_persists_and_emits_event() {
        let (service, bus, repo) = setup_service().await;
        let mut receiver = bus.subscribe();

        let initial = Project::create("Hank To Archive", "gabriel", None).unwrap();
        repo.save(&initial).await.unwrap();

        let input = ArchiveProjectInput {
            id: initial.id,
            reason: Some("End of life".into()),
            correlation_id: Some("req-arch-1".into()),
        };

        let output = service.execute(input).await.unwrap();
        assert_eq!(output.project.status, ProjectStatus::Archived);
        assert!(!output.already_archived);
        assert!(output.event_id.is_some());

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.event_type, EventKind::ProjectArchived);
        assert_eq!(event.project_id, initial.id);
    }

    #[tokio::test]
    async fn archive_already_archived_project_is_idempotent() {
        let (service, bus, repo) = setup_service().await;
        let mut receiver = bus.subscribe();

        let mut initial = Project::create("Hank Archived", "gabriel", None).unwrap();
        initial.archive().unwrap();
        repo.save(&initial).await.unwrap();

        let input = ArchiveProjectInput {
            id: initial.id,
            reason: None,
            correlation_id: None,
        };

        let output = service.execute(input).await.unwrap();
        assert_eq!(output.project.status, ProjectStatus::Archived);
        assert!(output.already_archived);
        assert!(output.event_id.is_none());

        // Nenhum evento duplicado deve ser emitido
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn archive_nonexistent_project_fails() {
        let (service, _, _) = setup_service().await;

        let input = ArchiveProjectInput {
            id: ProjectId::new(),
            reason: None,
            correlation_id: None,
        };

        let err = service.execute(input).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
