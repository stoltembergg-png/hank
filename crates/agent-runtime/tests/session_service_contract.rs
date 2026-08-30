use agent_core::ids::{AgentId, ProjectId};
use agent_core::session::SessionStatus;
use agent_runtime::migrations::run_migrations;
use agent_runtime::provider_service::{InvocationError, InvocationRequest, InvocationResult};
use agent_runtime::session_service::{SessionApplicationService, SessionServiceError, TurnInvoker};
use agent_runtime::sqlite::SqliteStorage;
use provider_core::capabilities::{CapabilityRequirement, ModelModality};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, ProjectScopeId,
};
use provider_core::request::{
    CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
};
use provider_core::{CancellationToken, FinishReason, ModelId, ProviderId, Usage};
use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct MockInvoker {
    calls: AtomicUsize,
    outcome: MockOutcome,
}

#[derive(Clone, Copy)]
enum MockOutcome {
    Success,
    Failure,
}

impl MockInvoker {
    fn success() -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            outcome: MockOutcome::Success,
        })
    }

    fn failing(_error: InvocationError) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            outcome: MockOutcome::Failure,
        })
    }
}

impl TurnInvoker for MockInvoker {
    fn complete<'a>(
        &'a self,
        _request: InvocationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<InvocationResult, InvocationError>> + Send + 'a>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let result = match self.outcome {
            MockOutcome::Success => Ok(InvocationResult {
                attempt_id: "request-1:attempt_1".into(),
                attempt_number: 1,
                provider_id: ProviderId::parse("mock-provider").unwrap(),
                model_id: ModelId::parse("mock-model").unwrap(),
                text: "assistant response".into(),
                finish_reason: FinishReason::Stop,
                usage: Usage {
                    input_tokens: 2,
                    output_tokens: 3,
                },
            }),
            MockOutcome::Failure => Err(InvocationError::Provider(
                provider_core::ModelProviderError::Unavailable,
            )),
        };
        Box::pin(async move { result })
    }
}

async fn setup(
    invoker: Arc<MockInvoker>,
) -> (SqliteStorage, SessionApplicationService, ProjectId, AgentId) {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project_id = ProjectId::new();
    let agent_id = AgentId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Project', 'active', 'owner', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '{}')")
        .bind(project_id.to_string()).execute(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO agents (id, project_id, name, status, personality, policy, created_at, updated_at) VALUES (?, ?, 'Agent', 'active', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
        .bind(agent_id.to_string()).bind(project_id.to_string()).execute(storage.pool()).await.unwrap();
    let service = SessionApplicationService::new(storage.pool().clone(), invoker, 1).unwrap();
    (storage, service, project_id, agent_id)
}

fn request(
    _project_id: &ProjectId,
    agent_id: &AgentId,
    session_id: &agent_core::ids::SessionId,
) -> InvocationRequest {
    let cancellation = CancellationToken::new();
    let normalized = NormalizedRequest {
        schema_version: 1,
        request_id: "request-1".into(),
        correlation_id: "correlation-1".into(),
        project_id: "project_1".into(),
        agent_id: agent_id.to_string(),
        session_id: Some(session_id.to_string()),
        provider_id: ProviderId::parse("mock-provider").unwrap(),
        model_id: ModelId::parse("mock-model").unwrap(),
        messages: vec![NormalizedMessage {
            role: MessageRole::User,
            content: "hello".into(),
        }],
        modalities: BTreeSet::from([ModelModality::Text]),
        capabilities: CapabilityRequirement {
            modalities: BTreeSet::from([ModelModality::Text]),
            features: BTreeSet::new(),
            min_context_tokens: Some(128),
            min_output_tokens: Some(16),
        },
        tools: vec![],
        budget: RequestBudget {
            max_tokens: Some(100),
            max_cost_micros: Some(1_000),
        },
        cancellation: CancellationMetadata {
            cancellation_id: "cancel-1".into(),
            deadline_unix_ms: None,
        },
        temperature: None,
    };
    InvocationRequest::new(
        normalized,
        CredentialAccount::new(
            ProjectScopeId::parse("project_1").unwrap(),
            ProviderId::parse("mock-provider").unwrap(),
            AccountId::parse("account_1").unwrap(),
        )
        .unwrap(),
        CredentialAccessContext::new(
            ProjectScopeId::parse("project_1").unwrap(),
            agent_id.to_string(),
            cancellation,
        )
        .unwrap(),
        vec![],
    )
    .unwrap()
}

#[tokio::test]
async fn lifecycle_create_open_close_is_project_agent_scoped() {
    let (storage, service, project_id, agent_id) = setup(MockInvoker::success()).await;
    let session = service
        .create(project_id, agent_id, "correlation-1", Some("title".into()))
        .await
        .unwrap();
    assert_eq!(session.status, SessionStatus::Active);
    let opened = service
        .open(&project_id, &agent_id, &session.id)
        .await
        .unwrap();
    assert_eq!(opened.id, session.id);
    assert!(matches!(
        service
            .open(&project_id, &AgentId::new(), &session.id)
            .await,
        Err(SessionServiceError::Unauthorized)
    ));
    let closed = service
        .close(&project_id, &agent_id, &session.id)
        .await
        .unwrap();
    assert_eq!(closed.status, SessionStatus::Closed);
    storage.close().await;
}

#[tokio::test]
async fn lifecycle_constructor_creates_sessions_without_provider_invoker() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project_id = ProjectId::new();
    let agent_id = AgentId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Project', 'active', 'owner', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '{}')")
        .bind(project_id.to_string()).execute(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO agents (id, project_id, name, status, personality, policy, created_at, updated_at) VALUES (?, ?, 'Agent', 'active', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
        .bind(agent_id.to_string()).bind(project_id.to_string()).execute(storage.pool()).await.unwrap();

    let service = SessionApplicationService::new_lifecycle(storage.pool().clone(), 1).unwrap();
    let session = service
        .create(project_id, agent_id, "correlation-lifecycle", None)
        .await
        .unwrap();

    assert_eq!(session.status, SessionStatus::Active);
    storage.close().await;
}

#[tokio::test]
async fn lifecycle_service_rejects_turns_before_persisting_messages_without_provider() {
    let storage = SqliteStorage::connect_in_memory().await.unwrap();
    run_migrations(storage.pool()).await.unwrap();
    let project_id = ProjectId::new();
    let agent_id = AgentId::new();
    sqlx::query("INSERT INTO projects (id, name, status, owner, created_at, updated_at, settings) VALUES (?, 'Project', 'active', 'owner', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '{}')")
        .bind(project_id.to_string()).execute(storage.pool()).await.unwrap();
    sqlx::query("INSERT INTO agents (id, project_id, name, status, personality, policy, created_at, updated_at) VALUES (?, ?, 'Agent', 'active', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')")
        .bind(agent_id.to_string()).bind(project_id.to_string()).execute(storage.pool()).await.unwrap();

    let service = SessionApplicationService::new_lifecycle(storage.pool().clone(), 1).unwrap();
    let session = service
        .create(project_id, agent_id, "correlation-no-provider", None)
        .await
        .unwrap();
    let error = service
        .send_turn(
            &project_id,
            &agent_id,
            &session.id,
            request(&project_id, &agent_id, &session.id),
            1,
            "hello",
        )
        .await
        .unwrap_err();
    assert!(matches!(error, SessionServiceError::ProviderFailure));

    let messages =
        agent_runtime::message_repo::SqliteMessageRepository::new(storage.pool().clone())
            .list(&project_id, &session.id, 0, 10)
            .await
            .unwrap();
    assert!(messages.is_empty());
    storage.close().await;
}

#[tokio::test]
async fn send_turn_persists_user_and_assistant_and_returns_terminal_result() {
    let (storage, service, project_id, agent_id) = setup(MockInvoker::success()).await;
    let session = service
        .create(project_id, agent_id, "correlation-1", None)
        .await
        .unwrap();
    let result = service
        .send_turn(
            &project_id,
            &agent_id,
            &session.id,
            request(&project_id, &agent_id, &session.id),
            1,
            "hello",
        )
        .await
        .unwrap();
    assert_eq!(
        result.state,
        agent_runtime::execution::ExecutionState::Completed
    );
    assert!(result.assistant_message_id.is_some());
    let messages =
        agent_runtime::message_repo::SqliteMessageRepository::new(storage.pool().clone())
            .list(&project_id, &session.id, 0, 10)
            .await
            .unwrap();
    assert_eq!(messages.len(), 2);
    storage.close().await;
}

#[tokio::test]
async fn provider_failure_persists_user_only_and_cancel_skips_invoker() {
    let failing = MockInvoker::failing(InvocationError::Provider(
        provider_core::ModelProviderError::Unavailable,
    ));
    let (storage, service, project_id, agent_id) = setup(failing.clone()).await;
    let session = service
        .create(project_id, agent_id, "correlation-1", None)
        .await
        .unwrap();
    assert!(matches!(
        service
            .send_turn(
                &project_id,
                &agent_id,
                &session.id,
                request(&project_id, &agent_id, &session.id),
                1,
                "hello"
            )
            .await,
        Err(SessionServiceError::ProviderFailure)
    ));
    assert_eq!(failing.calls.load(Ordering::SeqCst), 1);
    let messages =
        agent_runtime::message_repo::SqliteMessageRepository::new(storage.pool().clone())
            .list(&project_id, &session.id, 0, 10)
            .await
            .unwrap();
    assert_eq!(messages.len(), 1);
    let cancelled = MockInvoker::success();
    let (storage2, service2, project2, agent2) = setup(cancelled.clone()).await;
    let session2 = service2
        .create(project2, agent2, "correlation-2", None)
        .await
        .unwrap();
    let token = CancellationToken::new();
    token.cancel();
    let mut req = request(&project2, &agent2, &session2.id);
    req.access.cancellation = token;
    assert!(matches!(
        service2
            .send_turn(&project2, &agent2, &session2.id, req, 1, "hello")
            .await,
        Err(SessionServiceError::Cancelled)
    ));
    assert_eq!(cancelled.calls.load(Ordering::SeqCst), 0);
    storage.close().await;
    storage2.close().await;
}

#[tokio::test]
async fn closed_session_rejects_new_turn_without_invocation() {
    let invoker = MockInvoker::success();
    let (storage, service, project_id, agent_id) = setup(invoker.clone()).await;
    let session = service
        .create(project_id, agent_id, "correlation-1", None)
        .await
        .unwrap();
    service
        .close(&project_id, &agent_id, &session.id)
        .await
        .unwrap();
    assert!(matches!(
        service
            .send_turn(
                &project_id,
                &agent_id,
                &session.id,
                request(&project_id, &agent_id, &session.id),
                1,
                "hello"
            )
            .await,
        Err(SessionServiceError::SessionClosed)
    ));
    assert_eq!(invoker.calls.load(Ordering::SeqCst), 0);
    storage.close().await;
}
