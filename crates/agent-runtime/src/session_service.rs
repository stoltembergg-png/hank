//! Application service owning Session lifecycle and Agent turn orchestration.

use crate::execution::{Execution, ExecutionEvent, ExecutionState};
use crate::message_repo::{MessageStorageError, SqliteMessageRepository};
use crate::provider_service::{
    InvocationError, InvocationRequest, InvocationResult, ProviderApplicationService,
};
use crate::session_repo::{SessionStorageError, SqliteSessionRepository};
use agent_core::ids::{AgentId, ProjectId};
use agent_core::session::{Message, MessageProvenance, MessageRole, Session, SessionStatus};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use thiserror::Error;

pub trait TurnInvoker: Send + Sync {
    fn complete<'a>(
        &'a self,
        request: InvocationRequest,
    ) -> BoxFuture<'a, Result<InvocationResult, InvocationError>>;
}

pub struct ProviderApplicationInvoker {
    service: Arc<ProviderApplicationService>,
}

impl ProviderApplicationInvoker {
    pub fn new(service: Arc<ProviderApplicationService>) -> Self {
        Self { service }
    }
}

impl TurnInvoker for ProviderApplicationInvoker {
    fn complete<'a>(
        &'a self,
        request: InvocationRequest,
    ) -> BoxFuture<'a, Result<InvocationResult, InvocationError>> {
        Box::pin(self.service.complete(request))
    }
}

#[derive(Debug, Error)]
pub enum SessionServiceError {
    #[error("session storage failed: {0}")]
    Storage(#[from] SessionStorageError),
    #[error("message storage failed: {0}")]
    MessageStorage(#[from] MessageStorageError),
    #[error("session access is unauthorized")]
    Unauthorized,
    #[error("session is not active")]
    SessionClosed,
    #[error("session request is invalid")]
    Invalid,
    #[error("session execution was cancelled")]
    Cancelled,
    #[error("session execution provider failure")]
    ProviderFailure,
    #[error("session execution state transition failed")]
    State,
    #[error("session execution budget exceeded")]
    Budget,
    #[error("session execution concurrency limit reached")]
    Concurrency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnResult {
    pub execution_id: String,
    pub user_message_id: agent_core::ids::MessageId,
    pub assistant_message_id: Option<agent_core::ids::MessageId>,
    pub state: ExecutionState,
    pub text: Option<String>,
    pub attempt_id: Option<String>,
}

pub struct SessionApplicationService {
    sessions: SqliteSessionRepository,
    messages: SqliteMessageRepository,
    invoker: Arc<dyn TurnInvoker>,
    concurrency: crate::execution::ExecutionConcurrency,
}

impl SessionApplicationService {
    pub fn new(
        pool: sqlx::Pool<sqlx::Sqlite>,
        invoker: Arc<dyn TurnInvoker>,
        max_concurrent: usize,
    ) -> Result<Self, SessionServiceError> {
        Ok(Self {
            sessions: SqliteSessionRepository::new(pool.clone()),
            messages: SqliteMessageRepository::new(pool),
            invoker,
            concurrency: crate::execution::ExecutionConcurrency::new(max_concurrent)
                .map_err(|_| SessionServiceError::Concurrency)?,
        })
    }

    pub async fn create(
        &self,
        project_id: ProjectId,
        agent_id: AgentId,
        correlation_id: &str,
        title: Option<String>,
    ) -> Result<Session, SessionServiceError> {
        let mut session = Session::new(project_id, agent_id, correlation_id)
            .map_err(|_| SessionServiceError::Invalid)?;
        session.title = title;
        session.activate().map_err(|_| SessionServiceError::State)?;
        self.sessions.create(&session).await?;
        Ok(session)
    }

    pub async fn open(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        session_id: &agent_core::ids::SessionId,
    ) -> Result<Session, SessionServiceError> {
        let session = self
            .sessions
            .get_by_id(project_id, session_id)
            .await?
            .ok_or(SessionServiceError::Storage(SessionStorageError::NotFound))?;
        if session.agent_id != *agent_id {
            return Err(SessionServiceError::Unauthorized);
        }
        Ok(session)
    }

    pub async fn close(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        session_id: &agent_core::ids::SessionId,
    ) -> Result<Session, SessionServiceError> {
        let mut session = self.open(project_id, agent_id, session_id).await?;
        if session.status == SessionStatus::Closed {
            return Ok(session);
        }
        let expected_updated_at = session.updated_at;
        session
            .begin_close()
            .map_err(|_| SessionServiceError::State)?;
        session.close().map_err(|_| SessionServiceError::State)?;
        self.sessions.update(&session, expected_updated_at).await?;
        Ok(session)
    }

    pub async fn send_turn(
        &self,
        project_id: &ProjectId,
        agent_id: &AgentId,
        session_id: &agent_core::ids::SessionId,
        request: InvocationRequest,
        generation: u64,
        user_text: &str,
    ) -> Result<TurnResult, SessionServiceError> {
        let _permit = self
            .concurrency
            .try_acquire()
            .map_err(|_| SessionServiceError::Concurrency)?;
        let session = self.open(project_id, agent_id, session_id).await?;
        if session.status != SessionStatus::Active {
            return Err(SessionServiceError::SessionClosed);
        }
        if request.access.cancellation.is_cancelled() {
            return Err(SessionServiceError::Cancelled);
        }
        if request.normalized.agent_id != agent_id.to_string()
            || request.normalized.session_id.as_deref() != Some(session_id.to_string().as_str())
        {
            return Err(SessionServiceError::Unauthorized);
        }
        let existing = self.messages.list(project_id, session_id, 0, 100).await?;
        let next_sequence = existing
            .last()
            .map(|message| message.sequence.saturating_add(1))
            .unwrap_or(0);
        let user_message = Message::new(
            *session_id,
            MessageRole::User,
            MessageProvenance::User,
            next_sequence,
            generation,
            user_text,
        )
        .map_err(|_| SessionServiceError::Invalid)?;
        self.messages
            .append(project_id, session_id, &user_message)
            .await?;

        let execution_id = format!("exec_{}", request.normalized.request_id);
        let mut execution = Execution::new(
            execution_id.clone(),
            *session_id,
            *agent_id,
            request.normalized.correlation_id.clone(),
            generation,
            u64::from(request.normalized.budget.max_tokens.unwrap_or(1_000_000)),
            request
                .normalized
                .budget
                .max_cost_micros
                .unwrap_or(1_000_000),
        )
        .map_err(|_| SessionServiceError::Invalid)?;
        execution
            .apply(ExecutionEvent::Start)
            .map_err(|_| SessionServiceError::State)?;
        execution
            .apply(ExecutionEvent::ProviderInvoked(
                request.normalized.request_id.clone(),
            ))
            .map_err(|_| SessionServiceError::State)?;
        let result = match self.invoker.complete(request).await {
            Ok(result) => result,
            Err(InvocationError::Cancelled) => {
                execution
                    .apply(ExecutionEvent::Cancelled)
                    .map_err(|_| SessionServiceError::State)?;
                return Err(SessionServiceError::Cancelled);
            }
            Err(_) => {
                execution
                    .apply(ExecutionEvent::Failed("provider_error".into()))
                    .map_err(|_| SessionServiceError::State)?;
                return Err(SessionServiceError::ProviderFailure);
            }
        };
        execution
            .record_usage(
                u64::from(result.usage.input_tokens) + u64::from(result.usage.output_tokens),
                0,
            )
            .map_err(|_| SessionServiceError::Budget)?;
        execution
            .apply(ExecutionEvent::Completed)
            .map_err(|_| SessionServiceError::State)?;
        let assistant = Message::new(
            *session_id,
            MessageRole::Assistant,
            MessageProvenance::Provider,
            next_sequence.saturating_add(1),
            generation,
            result.text.clone(),
        )
        .map_err(|_| SessionServiceError::Invalid)?;
        let mut assistant = assistant;
        assistant
            .start_stream()
            .map_err(|_| SessionServiceError::State)?;
        assistant
            .complete()
            .map_err(|_| SessionServiceError::State)?;
        let assistant_id = assistant.id;
        self.messages
            .append(project_id, session_id, &assistant)
            .await?;
        Ok(TurnResult {
            execution_id,
            user_message_id: user_message.id,
            assistant_message_id: Some(assistant_id),
            state: execution.state(),
            text: Some(result.text),
            attempt_id: Some(result.attempt_id),
        })
    }
}
