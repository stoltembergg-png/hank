use agent_core::ids::{AgentId, SessionId};
use agent_runtime::agent_node::{
    AgentNodeAdapter, AgentNodeError, AgentNodeRequest, AgentNodeResult,
};
use agent_runtime::provider_service::{InvocationError, InvocationRequest, InvocationResult};
use agent_runtime::session_service::TurnInvoker;
use provider_core::capabilities::{CapabilityRequirement, ModelModality};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, ProjectScopeId,
};
use provider_core::fallback::FallbackCandidate;
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
}
impl MockInvoker {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
        })
    }
}
impl TurnInvoker for MockInvoker {
    fn complete<'a>(
        &'a self,
        _request: InvocationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<InvocationResult, InvocationError>> + Send + 'a>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(InvocationResult {
                attempt_id: "request-1:attempt_1".into(),
                attempt_number: 1,
                provider_id: ProviderId::parse("mock-provider").unwrap(),
                model_id: ModelId::parse("mock-model").unwrap(),
                text: "agent response".into(),
                finish_reason: FinishReason::Stop,
                usage: Usage {
                    input_tokens: 2,
                    output_tokens: 3,
                },
            })
        })
    }
}

fn request(
    project: &str,
    agent: &AgentId,
    session: &SessionId,
    cancellation: CancellationToken,
    max_tokens: u32,
) -> InvocationRequest {
    let project = project.to_string();
    let account = CredentialAccount::new(
        ProjectScopeId::parse(project.clone()).unwrap(),
        ProviderId::parse("mock-provider").unwrap(),
        AccountId::parse("account_mock").unwrap(),
    )
    .unwrap();
    let access = CredentialAccessContext::new(
        ProjectScopeId::parse(project.clone()).unwrap(),
        agent.to_string(),
        cancellation.clone(),
    )
    .unwrap();
    let normalized = NormalizedRequest {
        schema_version: 1,
        request_id: "request-1".into(),
        correlation_id: "correlation-1".into(),
        project_id: project,
        agent_id: agent.to_string(),
        session_id: Some(session.to_string()),
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
            max_tokens: Some(max_tokens),
            max_cost_micros: Some(1_000),
        },
        cancellation: CancellationMetadata {
            cancellation_id: "cancel-1".into(),
            deadline_unix_ms: Some(2_000_000_000_000),
        },
        temperature: Some(0.2),
    };
    InvocationRequest::new(normalized, account, access, Vec::<FallbackCandidate>::new()).unwrap()
}

fn node_request(
    project: String,
    agent: AgentId,
    session: SessionId,
    invocation: InvocationRequest,
    generation: u64,
    max_tokens: u64,
) -> AgentNodeRequest {
    AgentNodeRequest {
        run_id: "run-1".into(),
        node_id: "agent-a".into(),
        project_id: project,
        agent_id: agent,
        session_id: session,
        generation,
        max_tokens,
        invocation,
    }
}

// @spec:AC-970
#[tokio::test]
async fn identity_and_cancellation_fail_before_invoker() {
    let project = "project_1".to_string();
    let agent = AgentId::new();
    let session = SessionId::new();
    let invoker = MockInvoker::new();
    let adapter = AgentNodeAdapter::new(invoker.clone());
    let foreign = "project_2".to_string();
    let foreign_request = node_request(
        foreign,
        agent,
        session,
        request(&project, &agent, &session, CancellationToken::new(), 10),
        1,
        10,
    );
    assert!(matches!(
        adapter.execute(foreign_request).await,
        Err(AgentNodeError::Unauthorized)
    ));
    assert_eq!(invoker.calls.load(Ordering::SeqCst), 0);

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let cancelled = node_request(
        project,
        agent,
        session,
        request("project_1", &agent, &session, cancellation, 10),
        1,
        10,
    );
    assert!(matches!(
        adapter.execute(cancelled).await,
        Err(AgentNodeError::Cancelled)
    ));
    assert_eq!(invoker.calls.load(Ordering::SeqCst), 0);
}

// @spec:AC-971
#[tokio::test]
async fn result_preserves_correlation_and_budget() {
    let project = "project_1".to_string();
    let agent = AgentId::new();
    let session = SessionId::new();
    let invoker = MockInvoker::new();
    let adapter = AgentNodeAdapter::new(invoker.clone());
    let result = adapter
        .execute(node_request(
            project.clone(),
            agent,
            session,
            request(&project, &agent, &session, CancellationToken::new(), 10),
            7,
            10,
        ))
        .await
        .unwrap();
    assert_eq!(
        (
            result.run_id.as_str(),
            result.node_id.as_str(),
            result.generation
        ),
        ("run-1", "agent-a", 7)
    );
    assert_eq!(result.usage_tokens, 5);
    assert_eq!(invoker.calls.load(Ordering::SeqCst), 1);

    let exceeded = node_request(
        project.clone(),
        agent,
        session,
        request("project_1", &agent, &session, CancellationToken::new(), 10),
        8,
        4,
    );
    assert!(matches!(
        adapter.execute(exceeded).await,
        Err(AgentNodeError::BudgetExceeded)
    ));
    assert_eq!(invoker.calls.load(Ordering::SeqCst), 1);
}

// @spec:AC-972
#[tokio::test]
async fn stale_result_is_rejected_without_mutating_it() {
    let result = AgentNodeResult {
        run_id: "run-1".into(),
        node_id: "agent-a".into(),
        session_id: SessionId::new(),
        generation: 3,
        text: "response".into(),
        usage_tokens: 1,
    };
    let adapter = AgentNodeAdapter::new(MockInvoker::new());
    assert_eq!(
        adapter.accept_result(&result, 4),
        Err(AgentNodeError::StaleGeneration)
    );
    assert_eq!(result.generation, 3);
}
