//! Serviço de aplicação para atualização de Projetos.
//!
//! Conforme PR-031 e regras de concorrência otimista, integridade e publicação transacional.

use crate::event_bus::EventBus;
use agent_core::error::DomainError;
use agent_core::ids::ProjectId;
use agent_core::project::{
    Project, ProjectRepository, ProjectStatus, MAX_PROJECT_DESCRIPTION_LEN, MAX_PROJECT_NAME_LEN,
};
use agent_protocol::events::{ApplicationEvent, EventKind};
use agent_protocol::ids::EventId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// DTO de entrada para atualização de projeto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectInput {
    pub id: ProjectId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProjectStatus>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub correlation_id: Option<String>,
}

/// DTO de saída resultante da atualização do projeto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProjectOutput {
    pub project: Project,
    pub event_id: Option<EventId>,
    pub correlation_id: Option<String>,
}

/// Serviço de aplicação para atualização controlada de projetos.
pub struct UpdateProjectService<R: ProjectRepository> {
    repository: Arc<R>,
    event_bus: Option<EventBus<ApplicationEvent>>,
}

impl<R: ProjectRepository> UpdateProjectService<R> {
    pub fn new(repository: Arc<R>, event_bus: Option<EventBus<ApplicationEvent>>) -> Self {
        Self {
            repository,
            event_bus,
        }
    }

    /// Executa a atualização do projeto:
    /// 1. Busca o projeto existente
    /// 2. Valida concorrência otimista (expected_updated_at)
    /// 3. Bloqueia updates em projetos arquivados
    /// 4. Aplica patches com validação de invariants
    /// 5. Persiste as mudanças
    /// 6. Publica o evento ProjectUpdated
    pub async fn execute(
        &self,
        input: UpdateProjectInput,
    ) -> Result<UpdateProjectOutput, DomainError> {
        let mut project = self.repository.get_by_id(&input.id).await?.ok_or_else(|| {
            DomainError::NotFound(format!("projeto não encontrado: {}", input.id))
        })?;

        // Não permite alterações em projetos já arquivados
        if project.status == ProjectStatus::Archived {
            return Err(DomainError::InvalidStateTransition {
                from: "archived".into(),
                to: "modified".into(),
            });
        }

        // Concorrência otimista: valida se o projeto não foi modificado concorrentemente
        if let Some(expected_time) = input.expected_updated_at {
            if (project.updated_at.timestamp_millis() - expected_time.timestamp_millis()).abs()
                > 1000
            {
                return Err(DomainError::InvariantViolation(
                    "conflito de concorrência: projeto foi modificado por outra operação".into(),
                ));
            }
        }

        let mut changed_fields = Vec::new();

        // Validação e aplicação do nome
        if let Some(new_name) = input.name {
            let trimmed = new_name.trim();
            if trimmed.is_empty() {
                return Err(DomainError::Validation(
                    "nome do projeto não pode ser vazio".into(),
                ));
            }
            if trimmed.len() > MAX_PROJECT_NAME_LEN {
                return Err(DomainError::Validation(format!(
                    "nome excede {} caracteres",
                    MAX_PROJECT_NAME_LEN
                )));
            }
            if trimmed.chars().any(|c| c.is_control()) {
                return Err(DomainError::Validation(
                    "nome não pode conter caracteres de controle".into(),
                ));
            }
            project.name = trimmed.to_string();
            changed_fields.push("name");
        }

        // Validação e aplicação da descrição
        if let Some(new_desc) = input.description {
            let trimmed = new_desc.trim();
            if trimmed.len() > MAX_PROJECT_DESCRIPTION_LEN {
                return Err(DomainError::Validation(format!(
                    "descrição excede {} caracteres",
                    MAX_PROJECT_DESCRIPTION_LEN
                )));
            }
            if trimmed
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
            {
                return Err(DomainError::Validation(
                    "descrição não pode conter caracteres de controle proibidos".into(),
                ));
            }
            project.description = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            changed_fields.push("description");
        }

        // Aplicação de transição de status (Pause / Resume)
        if let Some(new_status) = input.status {
            match new_status {
                ProjectStatus::Active => {
                    project.resume()?;
                    changed_fields.push("status");
                }
                ProjectStatus::Paused => {
                    project.pause()?;
                    changed_fields.push("status");
                }
                ProjectStatus::Archived => {
                    return Err(DomainError::InvalidStateTransition {
                        from: format!("{:?}", project.status),
                        to: "archived (use ArchiveProjectService)".into(),
                    });
                }
            }
        }

        project.updated_at = Utc::now();
        self.repository.update(&project).await?;

        let mut emitted_event_id = None;

        if let Some(ref bus) = self.event_bus {
            let payload = serde_json::json!({
                "name": project.name,
                "status": project.status,
                "changed_fields": changed_fields,
            })
            .to_string();

            let event_id = EventId::new();
            let event = ApplicationEvent {
                schema_version: 1,
                event_id,
                event_type: EventKind::ProjectUpdated,
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

        Ok(UpdateProjectOutput {
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

    async fn setup_service() -> (
        UpdateProjectService<SqliteProjectRepository>,
        EventBus<ApplicationEvent>,
        Arc<SqliteProjectRepository>,
    ) {
        let storage = SqliteStorage::connect_in_memory().await.unwrap();
        run_migrations(storage.pool()).await.unwrap();
        let repo = Arc::new(SqliteProjectRepository::new(storage.pool().clone()));
        let bus = EventBus::bounded(16);
        let service = UpdateProjectService::new(repo.clone(), Some(bus.clone()));
        (service, bus, repo)
    }

    #[tokio::test]
    async fn update_project_name_and_status_emits_event() {
        let (service, bus, repo) = setup_service().await;
        let mut receiver = bus.subscribe();

        let initial = Project::create("Initial Name", "gabriel", None).unwrap();
        repo.save(&initial).await.unwrap();

        let input = UpdateProjectInput {
            id: initial.id,
            name: Some("Renamed App".into()),
            description: Some("New desc".into()),
            status: Some(ProjectStatus::Paused),
            expected_updated_at: None,
            correlation_id: Some("req-update-1".into()),
        };

        let output = service.execute(input).await.unwrap();
        assert_eq!(output.project.name, "Renamed App");
        assert_eq!(output.project.status, ProjectStatus::Paused);
        assert_eq!(output.project.description, Some("New desc".into()));
        assert!(output.event_id.is_some());

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.event_type, EventKind::ProjectUpdated);
        assert_eq!(event.project_id, initial.id);
    }

    #[tokio::test]
    async fn update_archived_project_fails() {
        let (service, _, repo) = setup_service().await;

        let mut initial = Project::create("Initial Name", "gabriel", None).unwrap();
        initial.archive().unwrap();
        repo.save(&initial).await.unwrap();

        let input = UpdateProjectInput {
            id: initial.id,
            name: Some("Attempt rename".into()),
            description: None,
            status: None,
            expected_updated_at: None,
            correlation_id: None,
        };

        let err = service.execute(input).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidStateTransition { .. }));
    }

    #[tokio::test]
    async fn update_with_stale_concurrency_timestamp_fails() {
        let (service, _, repo) = setup_service().await;

        let initial = Project::create("Initial Name", "gabriel", None).unwrap();
        repo.save(&initial).await.unwrap();

        let stale_timestamp = initial.updated_at - chrono::Duration::seconds(60);

        let input = UpdateProjectInput {
            id: initial.id,
            name: Some("Stale rename".into()),
            description: None,
            status: None,
            expected_updated_at: Some(stale_timestamp),
            correlation_id: None,
        };

        let err = service.execute(input).await.unwrap_err();
        assert!(matches!(err, DomainError::InvariantViolation(_)));
    }
}
