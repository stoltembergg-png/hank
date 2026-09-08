//! Single-agent chat command and stream bridge for the desktop shell.
//!
//! The desktop fixture uses `MockProvider` only when running a debug build or
//! when `HANK_E2E_MOCK_PROVIDER=1` is explicitly set. Release builds without a
//! configured provider fail closed with a typed capability error.

use agent_core::agent::AgentStatus;
use agent_core::ids::{ProjectId, SessionId};
use agent_core::project::{ProjectRepository, ProjectStatus};
use agent_core::session::{Message, MessageProvenance, MessageRole, MessageStatus, SessionStatus};
use agent_protocol::chat_command::{ChatCommand, ChatCommandError, ChatCommandRegistry, CallerIdentity};
use agent_protocol::chat_stream::{
    ChatCancelReason, ChatStreamEvent, ChatStreamPayload, ChatStreamSubscription,
    ChatTerminalReason,
};
use agent_runtime::agent_repo::SqliteAgentRepository;
use agent_runtime::execution::Execution;
use agent_runtime::message_repo::{MessageStorageError, SqliteMessageRepository};
use agent_runtime::provider_service::{InvocationError, InvocationRequest, ProviderApplicationService};
use agent_runtime::session_repo::{SessionStorageError, SqliteSessionRepository};
use agent_runtime::streaming::StreamEventConsumer;
use agent_runtime::usage::{
    UsageAggregator, UsageConfidence, UsageEvent, UsageOutcome, UsageReadModel, UsageSource,
    USAGE_SCHEMA_VERSION,
};
use agent_runtime::SqliteStorage;
use agent_protocol::ids::TraceId;
use provider_core::capabilities::{CapabilityFeature, CapabilityRequirement, ModelModality};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService,
    ProjectScopeId,
};
use provider_core::fallback::FallbackPolicy;
use provider_core::registry::ProviderRegistry;
use provider_core::request::{
    CancellationMetadata, NormalizedMessage, NormalizedRequest, RequestBudget,
    MessageRole as ProviderMessageRole,
};
use provider_core::{CancellationToken, MockProvider, ModelId, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

use crate::streaming::{StreamBridge, StreamEventSink, StreamSinkError};
use crate::provider_runtime::configured_openai_provider;
use provider_core::transport::EndpointPolicy;
use crate::platform_store::PlatformSecretBackend;
use crate::provider_credential_store::ProviderCredentialStore;

const DESKTOP_CALLER_ID: &str = "desktop-webview";
const DESKTOP_CALLER_CLASS: &str = "desktop";
const MOCK_PROVIDER_ENV: &str = "HANK_E2E_MOCK_PROVIDER";
const MOCK_PROVIDER_ID: &str = "mock";
const MOCK_MODEL_ID: &str = "mock-model";
const MOCK_ACCOUNT_ID: &str = "account_mock";
const MAX_STREAM_QUEUE: usize = 64;
const MAX_OUTPUT_TOKENS: u32 = 8_192;
const MAX_MESSAGE_PAGE: usize = 100;

#[derive(Clone)]
pub struct ChatBridgeState {
    sessions: Arc<SqliteSessionRepository>,
    messages: Arc<SqliteMessageRepository>,
    projects: Arc<agent_runtime::project_repo::SqliteProjectRepository>,
    agents: Arc<SqliteAgentRepository>,
    provider: Arc<ProviderApplicationService>,
    credentials: Arc<dyn CredentialService>,
    commands: Arc<ChatCommandRegistry>,
    usage: Arc<Mutex<UsageAggregator>>,
    active: Arc<Mutex<HashMap<String, ActiveChat>>>,
    enabled: bool,
    provider_id: ProviderId,
    model_id: ModelId,
    account_id: AccountId,
}

#[derive(Clone)]
struct ActiveChat {
    cancellation: CancellationToken,
    session_id: SessionId,
    caller: CallerIdentity,
}

impl ChatBridgeState {
    fn new(storage: &SqliteStorage, credentials: Arc<dyn CredentialService>) -> Self {
        Self::new_with_store(storage, credentials, None)
    }

    fn new_with_store(
        storage: &SqliteStorage,
        credentials: Arc<dyn CredentialService>,
        secure_store: Option<Arc<ProviderCredentialStore<PlatformSecretBackend>>>,
    ) -> Self {
        let pool = storage.pool().clone();
        let registry = Arc::new(ProviderRegistry::new());
        let mut provider_id = ProviderId::parse(MOCK_PROVIDER_ID).expect("static provider id");
        let mut model_id = ModelId::parse(MOCK_MODEL_ID).expect("static model id");
        let mut account_id = AccountId::parse(MOCK_ACCOUNT_ID).expect("static account id");
        let fixture_enabled = cfg!(debug_assertions)
            || std::env::var(MOCK_PROVIDER_ENV).ok().as_deref() == Some("1");
        let mut enabled = fixture_enabled;
        let mut registered = false;
        if let Some(store) = secure_store {
            if let Some(endpoint) = std::env::var_os("HANK_OPENAI_ENDPOINT") {
                let configured = EndpointPolicy::parse(endpoint.to_string_lossy().into_owned())
                    .ok()
                    .and_then(|endpoint| configured_openai_provider(endpoint, store).ok());
                if let Some(provider) = configured {
                    registry.register(Arc::new(provider)).expect("OpenAI provider registration");
                    provider_id = ProviderId::parse("openai").expect("static OpenAI provider id");
                    model_id = ModelId::parse("gpt-4o-mini").expect("static OpenAI model id");
                    account_id = AccountId::parse(
                        std::env::var("HANK_OPENAI_ACCOUNT_ID").unwrap_or_else(|_| "account_openai".into()),
                    )
                    .expect("configured OpenAI account id");
                    enabled = true;
                    registered = true;
                } else {
                    // A malformed or unavailable configured endpoint must not
                    // silently fall back to the fixture provider.
                    enabled = false;
                }
            }
        }
        if !registered {
            registry
                .register(Arc::new(MockProvider::new(provider_id.clone(), "fixture-1")))
                .expect("mock provider registration");
        }
        let provider = Arc::new(ProviderApplicationService::new(
            registry,
            credentials.clone(),
            FallbackPolicy::new(2, 1_000_000, 1_000_000_000).expect("valid fixture policy"),
        ));
        Self {
            sessions: Arc::new(SqliteSessionRepository::new(pool.clone())),
            messages: Arc::new(SqliteMessageRepository::new(pool.clone())),
            projects: Arc::new(agent_runtime::project_repo::SqliteProjectRepository::new(
                pool.clone(),
            )),
            agents: Arc::new(SqliteAgentRepository::new(pool)),
            provider,
            credentials,
            commands: Arc::new(ChatCommandRegistry::new(256).expect("valid command capacity")),
            usage: Arc::new(Mutex::new(UsageAggregator::new(4096).expect("valid usage capacity"))),
            active: Arc::new(Mutex::new(HashMap::new())),
            enabled,
            provider_id,
            model_id,
            account_id,
        }
    }

    fn register(&self, command: &ChatCommand, session_id: SessionId) -> Result<CancellationToken, ChatBridgeError> {
        match self
            .commands
            .accept(command)
            .map_err(|error| ChatBridgeError::new(map_command_error(error), &command.command_id))?
        {
            agent_protocol::chat_command::ChatCommandStatus::Accepted => {}
            agent_protocol::chat_command::ChatCommandStatus::Duplicate => {
                return Err(ChatBridgeError::new(ChatBridgeErrorCode::DuplicateCommand, &command.command_id))
            }
            agent_protocol::chat_command::ChatCommandStatus::Stale => {
                return Err(ChatBridgeError::new(ChatBridgeErrorCode::StaleCommand, &command.command_id))
            }
        }
        let cancellation = CancellationToken::new();
        let mut active = self
            .active
            .lock()
            .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &command.command_id))?;
        active.insert(
            command.command_id.clone(),
            ActiveChat {
                cancellation: cancellation.clone(),
                session_id,
                caller: command.caller.clone(),
            },
        );
        Ok(cancellation)
    }

    fn remove(&self, command_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(command_id);
        }
    }
}

pub fn bridge_state(
    storage: &SqliteStorage,
    credentials: Arc<dyn CredentialService>,
) -> ChatBridgeState {
    ChatBridgeState::new(storage, credentials)
}

pub fn bridge_state_with_store(
    storage: &SqliteStorage,
    credentials: Arc<dyn CredentialService>,
    secure_store: Arc<ProviderCredentialStore<PlatformSecretBackend>>,
) -> ChatBridgeState {
    ChatBridgeState::new_with_store(storage, credentials, Some(secure_store))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatBridgeErrorCode {
    Disabled,
    InvalidCommand,
    DuplicateCommand,
    StaleCommand,
    Unauthorized,
    NotFound,
    Storage,
    ProviderUnavailable,
    Cancelled,
    InvalidStream,
    Event,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChatBridgeError {
    pub code: ChatBridgeErrorCode,
    pub correlation_id: String,
}

impl ChatBridgeError {
    fn new(code: ChatBridgeErrorCode, correlation_id: &str) -> Self {
        Self {
            code,
            correlation_id: if correlation_id.trim().is_empty() {
                "chat".into()
            } else {
                correlation_id.chars().take(128).collect()
            },
        }
    }
}

impl std::fmt::Display for ChatBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "chat command failed: {:?}", self.code)
    }
}

impl std::error::Error for ChatBridgeError {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SendChatCommandOutput {
    pub command_id: String,
    pub stream_id: String,
    pub state: &'static str,
    pub provider_id: String,
    pub model_id: String,
    pub provider_state: &'static str,
    pub capability: &'static str,
    pub attempt_number: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GetChatUsageInput {
    pub project_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub caller: CallerIdentity,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GetChatUsageOutput {
    pub usage: Option<UsageReadModel>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CancelChatCommandInput {
    pub command_id: String,
    pub session_id: String,
    pub caller: CallerIdentity,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CancelChatCommandOutput {
    pub accepted: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListChatMessagesInput {
    pub project_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub caller: CallerIdentity,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChatMessageSummary {
    pub id: String,
    pub role: &'static str,
    pub text: String,
    pub status: MessageStatus,
    pub sequence: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ListChatMessagesOutput {
    pub messages: Vec<ChatMessageSummary>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Clone)]
struct AppSink(AppHandle);

impl StreamEventSink for AppSink {
    fn emit(&mut self, event_name: &str, event: &ChatStreamEvent) -> Result<(), StreamSinkError> {
        self.0
            .emit(event_name, event)
            .map_err(|_| StreamSinkError::Unavailable)
    }
}

#[tauri::command]
pub async fn send_chat_command(
    app: AppHandle,
    state: State<'_, ChatBridgeState>,
    command: ChatCommand,
) -> Result<SendChatCommandOutput, ChatBridgeError> {
    send_chat_command_for_state(&app, state.inner(), command).await
}

async fn send_chat_command_for_state(
    app: &AppHandle,
    state: &ChatBridgeState,
    command: ChatCommand,
) -> Result<SendChatCommandOutput, ChatBridgeError> {
    if !state.enabled {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Disabled, &command.command_id));
    }
    let command = ChatCommand::new(
        command.command_id.clone(),
        command.stream_id.clone(),
        command.caller.clone(),
        command.project_id,
        command.agent_id,
        command.session_id,
        command.text.clone(),
        command.generation,
        command.cancellation_id.clone(),
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &command.command_id))?;
    if command.schema_version != ChatCommand::SCHEMA_VERSION
        || command.caller.caller_id != DESKTOP_CALLER_ID
        || command.caller.class != DESKTOP_CALLER_CLASS
    {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, &command.command_id));
    }
    let project_id = command
        .project_id
        .to_string()
        .parse::<ProjectId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &command.command_id))?;
    let agent_id = command.agent_id;
    let session_id = command.session_id;
    let session = state
        .sessions
        .get_by_id(&project_id, &session_id)
        .await
        .map_err(|error| map_session_storage(error, &command.command_id))?
        .ok_or_else(|| ChatBridgeError::new(ChatBridgeErrorCode::NotFound, &command.command_id))?;
    if session.agent_id != agent_id || session.status != SessionStatus::Active {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, &command.command_id));
    }
    let project = state
        .projects
        .get_by_id(&project_id)
        .await
        .map_err(|error| map_domain_error(error, &command.command_id))?
        .ok_or_else(|| ChatBridgeError::new(ChatBridgeErrorCode::NotFound, &command.command_id))?;
    let agent = state
        .agents
        .get(&project_id, &agent_id)
        .await
        .map_err(|error| map_domain_error(error, &command.command_id))?
        .ok_or_else(|| ChatBridgeError::new(ChatBridgeErrorCode::NotFound, &command.command_id))?;
    if project.status != ProjectStatus::Active || agent.status != AgentStatus::Active {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, &command.command_id));
    }
    let cancellation = state.register(&command, session_id)?;
    let result = execute_chat_turn(app, state, &command, session, cancellation).await;
    state.remove(&command.command_id);
    result
}

async fn execute_chat_turn(
    app: &AppHandle,
    state: &ChatBridgeState,
    command: &ChatCommand,
    mut session: agent_core::session::Session,
    cancellation: CancellationToken,
) -> Result<SendChatCommandOutput, ChatBridgeError> {
    let project_id = session.project_id;
    let agent_id = session.agent_id;
    let session_id = session.id;
    if session.trace_id.is_none() {
        session
            .set_trace_id(TraceId::new())
            .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &command.command_id))?;
    }
    let project_scope = ProjectScopeId::parse(format!("project_{project_id}"))
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &command.command_id))?;
    let provider_id = state.provider_id.clone();
    let account = CredentialAccount::new(
        project_scope.clone(),
        provider_id.clone(),
        state.account_id.clone(),
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &command.command_id))?;
    let access = CredentialAccessContext::new(
        project_scope.clone(),
        command.caller.caller_id.clone(),
        cancellation.clone(),
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, &command.command_id))?;
    state
        .credentials
        .resolve_ref(access.clone(), account.clone())
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::ProviderUnavailable, &command.command_id))?;
    let model_id = state.model_id.clone();
    let normalized = NormalizedRequest {
        schema_version: 1,
        request_id: command.command_id.clone(),
        correlation_id: command.command_id.clone(),
        project_id: project_scope.as_str().to_string(),
        agent_id: agent_id.to_string(),
        session_id: Some(session_id.to_string()),
        provider_id: provider_id.clone(),
        model_id: model_id.clone(),
        messages: vec![NormalizedMessage {
            role: ProviderMessageRole::User,
            content: command.text.clone(),
        }],
        modalities: std::collections::BTreeSet::from([ModelModality::Text]),
        capabilities: CapabilityRequirement {
            modalities: std::collections::BTreeSet::from([ModelModality::Text]),
            features: std::collections::BTreeSet::from([CapabilityFeature::Streaming]),
            min_context_tokens: None,
            min_output_tokens: Some(1),
        },
        tools: vec![],
        budget: RequestBudget {
            max_tokens: Some(MAX_OUTPUT_TOKENS),
            max_cost_micros: Some(1_000_000),
        },
        cancellation: CancellationMetadata {
            cancellation_id: command.cancellation_id.clone(),
            deadline_unix_ms: None,
        },
        temperature: None,
    };
    let invocation = InvocationRequest::new(normalized, account, access, vec![])
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &command.command_id))?;

    let existing = state
        .messages
        .list(&project_id, &session_id, 0, 100)
        .await
        .map_err(|error| map_message_storage(error, &command.command_id))?;
    let next_sequence = existing
        .last()
        .map(|message| message.sequence.saturating_add(1))
        .unwrap_or(0);
    let user_message = Message::new(
        session_id,
        MessageRole::User,
        MessageProvenance::User,
        next_sequence,
        command.generation,
        command.text.clone(),
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &command.command_id))?;
    state
        .messages
        .append(&project_id, &session_id, &user_message)
        .await
        .map_err(|error| map_message_storage(error, &command.command_id))?;
    let assistant_sequence = next_sequence.saturating_add(1);
    let mut assistant = Message::new(
        session_id,
        MessageRole::Assistant,
        MessageProvenance::Provider,
        assistant_sequence,
        command.generation,
        "",
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &command.command_id))?;
    state
        .messages
        .append(&project_id, &session_id, &assistant)
        .await
        .map_err(|error| map_message_storage(error, &command.command_id))?;

    let stream_id = command.stream_id.clone();
    let subscription = ChatStreamSubscription::new(
        stream_id.clone(),
        command.command_id.clone(),
        command.caller.clone(),
        project_id,
        agent_id,
        session_id,
        command.generation,
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &command.command_id))?;
    let mut bridge = StreamBridge::new(subscription.clone(), MAX_STREAM_QUEUE, AppSink(app.clone()))
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &command.command_id))?;
    publish(&mut bridge, &subscription, 0, ChatStreamPayload::Start, &command.command_id)?;

    let execution_id = format!("exec_{}", command.command_id);
    let mut execution = Execution::new(
        execution_id.clone(),
        session_id,
        agent_id,
        command.command_id.clone(),
        command.generation,
        u64::from(MAX_OUTPUT_TOKENS),
        1_000_000,
    )
    .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &command.command_id))?;
    let provider_events = match state.provider.stream(invocation).await {
        Ok(events) => events,
        Err(InvocationError::Cancelled) => {
            assistant.start_stream().ok();
            assistant.cancel().ok();
            record_missing_usage(
                state,
                &command.command_id,
                MissingUsage {
                    execution_id: execution_id.clone(),
                    attempt_id: format!("{}:attempt_1", command.command_id),
                    project_id,
                    agent_id,
                    session_id,
                    outcome: UsageOutcome::Cancelled,
                },
            )?;
            persist_turn(state, &mut session, &user_message, &assistant, &command.command_id).await?;
            publish(&mut bridge, &subscription, 1, ChatStreamPayload::Cancel { reason: ChatCancelReason::User }, &command.command_id)?;
            return Ok(chat_output_for_state(state, &command.command_id, stream_id, "cancelled"));
        }
        Err(_) => {
            assistant.start_stream().ok();
            assistant.fail("provider_error").ok();
            record_missing_usage(
                state,
                &command.command_id,
                MissingUsage {
                    execution_id: execution_id.clone(),
                    attempt_id: format!("{}:attempt_1", command.command_id),
                    project_id,
                    agent_id,
                    session_id,
                    outcome: UsageOutcome::Failed,
                },
            )?;
            persist_turn(state, &mut session, &user_message, &assistant, &command.command_id).await?;
            publish(&mut bridge, &subscription, 1, ChatStreamPayload::Error { code: agent_protocol::chat_stream::ChatErrorCode::ProviderFailure }, &command.command_id)?;
            return Err(ChatBridgeError::new(ChatBridgeErrorCode::ProviderUnavailable, &command.command_id));
        }
    };

    let stream_outcome = StreamEventConsumer::apply_with_cancellation(
        &mut execution,
        &mut assistant,
        provider_events.clone(),
        command.generation,
        cancellation,
    );
    let mut next_event = 1;
    let output_state = match stream_outcome {
        Ok(_) => {
            record_missing_usage(
                state,
                &command.command_id,
                MissingUsage {
                    execution_id: execution_id.clone(),
                    attempt_id: provider_events
                        .first()
                        .map(|event| event.attempt_id.clone())
                        .unwrap_or_else(|| "missing-attempt".into()),
                    project_id,
                    agent_id,
                    session_id,
                    outcome: UsageOutcome::Completed,
                },
            )?;
            persist_turn(state, &mut session, &user_message, &assistant, &command.command_id).await?;
            for event in provider_events {
                if !event.text.is_empty() {
                    publish(&mut bridge, &subscription, next_event, ChatStreamPayload::Delta { text: event.text }, &command.command_id)?;
                    next_event = next_event.saturating_add(1);
                }
                if event.terminal {
                    publish(&mut bridge, &subscription, next_event, ChatStreamPayload::Finish { reason: ChatTerminalReason::Completed }, &command.command_id)?;
                }
            }
            "completed"
        }
        Err(agent_runtime::streaming::StreamError::Cancelled) => {
            record_missing_usage(
                state,
                &command.command_id,
                MissingUsage {
                    execution_id: execution_id.clone(),
                    attempt_id: provider_events
                        .first()
                        .map(|event| event.attempt_id.clone())
                        .unwrap_or_else(|| "missing-attempt".into()),
                    project_id,
                    agent_id,
                    session_id,
                    outcome: UsageOutcome::Cancelled,
                },
            )?;
            persist_turn(state, &mut session, &user_message, &assistant, &command.command_id).await?;
            publish(&mut bridge, &subscription, next_event, ChatStreamPayload::Cancel { reason: ChatCancelReason::User }, &command.command_id)?;
            "cancelled"
        }
        Err(_) => {
            if !assistant.status.is_terminal() {
                assistant.fail("stream_error").ok();
            }
            record_missing_usage(
                state,
                &command.command_id,
                MissingUsage {
                    execution_id: execution_id.clone(),
                    attempt_id: provider_events
                        .first()
                        .map(|event| event.attempt_id.clone())
                        .unwrap_or_else(|| "missing-attempt".into()),
                    project_id,
                    agent_id,
                    session_id,
                    outcome: UsageOutcome::Failed,
                },
            )?;
            persist_turn(state, &mut session, &user_message, &assistant, &command.command_id).await?;
            publish(&mut bridge, &subscription, next_event, ChatStreamPayload::Error { code: agent_protocol::chat_stream::ChatErrorCode::InvalidStream }, &command.command_id)?;
            return Err(ChatBridgeError::new(ChatBridgeErrorCode::InvalidStream, &command.command_id));
        }
    };
    Ok(SendChatCommandOutput {
        command_id: command.command_id.clone(),
        stream_id,
        state: output_state,
        provider_id: state.provider_id.as_str().to_string(),
        model_id: state.model_id.as_str().to_string(),
        provider_state: "selected",
        capability: "confirmed",
        attempt_number: 1,
    })
}

#[cfg(test)]
fn chat_output(command_id: &str, stream_id: String, state: &'static str) -> SendChatCommandOutput {
    SendChatCommandOutput {
        command_id: command_id.to_string(),
        stream_id,
        state,
        provider_id: MOCK_PROVIDER_ID.into(),
        model_id: MOCK_MODEL_ID.into(),
        provider_state: "selected",
        capability: "confirmed",
        attempt_number: 1,
    }
}

fn chat_output_for_state(
    state: &ChatBridgeState,
    command_id: &str,
    stream_id: String,
    output_state: &'static str,
) -> SendChatCommandOutput {
    SendChatCommandOutput {
        command_id: command_id.to_string(),
        stream_id,
        state: output_state,
        provider_id: state.provider_id.as_str().to_string(),
        model_id: state.model_id.as_str().to_string(),
        provider_state: "selected",
        capability: "confirmed",
        attempt_number: 1,
    }
}

struct MissingUsage {
    execution_id: String,
    attempt_id: String,
    project_id: ProjectId,
    agent_id: agent_core::ids::AgentId,
    session_id: SessionId,
    outcome: UsageOutcome,
}

fn record_missing_usage(
    state: &ChatBridgeState,
    command_id: &str,
    usage: MissingUsage,
) -> Result<(), ChatBridgeError> {
    let provider_id = state.provider_id.clone();
    let model_id = state.model_id.clone();
    let event = UsageEvent {
        schema_version: USAGE_SCHEMA_VERSION,
        attempt_id: usage.attempt_id,
        execution_id: usage.execution_id,
        project_id: usage.project_id,
        agent_id: usage.agent_id,
        session_id: usage.session_id,
        provider_id: Some(provider_id),
        model_id: Some(model_id),
        input_tokens: None,
        output_tokens: None,
        cost_micros: None,
        currency: None,
        source: UsageSource::Missing,
        confidence: UsageConfidence::Unavailable,
        outcome: usage.outcome,
        terminal: true,
    };
    state
        .usage
        .lock()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, command_id))?
        .record(event)
        .map(|_| ())
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, command_id))
}

async fn persist_turn(
    state: &ChatBridgeState,
    session: &mut agent_core::session::Session,
    user: &Message,
    assistant: &Message,
    command_id: &str,
) -> Result<(), ChatBridgeError> {
    state
        .messages
        .update(&session.project_id, assistant, MessageStatus::Draft)
        .await
        .map_err(|error| map_message_storage(error, command_id))?;
    let expected = session.updated_at;
    session.add_message(user.clone());
    session.add_message(assistant.clone());
    state
        .sessions
        .update(session, expected)
        .await
        .map_err(|error| map_session_storage(error, command_id))
}

fn publish(
    bridge: &mut StreamBridge<AppSink>,
    subscription: &ChatStreamSubscription,
    sequence: u64,
    payload: ChatStreamPayload,
    command_id: &str,
) -> Result<(), ChatBridgeError> {
    let event = ChatStreamEvent::new(subscription, sequence, payload)
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidStream, command_id))?;
    bridge
        .publish(event)
        .and_then(|_| bridge.flush().map(|_| ()))
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Event, command_id))
}

#[tauri::command]
pub fn cancel_chat_command(
    state: State<'_, ChatBridgeState>,
    input: CancelChatCommandInput,
) -> Result<CancelChatCommandOutput, ChatBridgeError> {
    if input.caller.caller_id != DESKTOP_CALLER_ID || input.caller.class != DESKTOP_CALLER_CLASS {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, &input.command_id));
    }
    let session_id = input
        .session_id
        .parse::<SessionId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, &input.command_id))?;
    let active = state
        .active
        .lock()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, &input.command_id))?;
    if let Some(turn) = active.get(&input.command_id) {
        if turn.session_id == session_id && turn.caller == input.caller {
            turn.cancellation.cancel();
            return Ok(CancelChatCommandOutput { accepted: true });
        }
    }
    Ok(CancelChatCommandOutput { accepted: false })
}

#[tauri::command]
pub async fn list_chat_messages(
    state: State<'_, ChatBridgeState>,
    input: ListChatMessagesInput,
) -> Result<ListChatMessagesOutput, ChatBridgeError> {
    if input.caller.caller_id != DESKTOP_CALLER_ID || input.caller.class != DESKTOP_CALLER_CLASS {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, "chat"));
    }
    let project_id = input
        .project_id
        .parse::<ProjectId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, "chat"))?;
    let agent_id = input
        .agent_id
        .parse::<agent_core::ids::AgentId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, "chat"))?;
    let session_id = input
        .session_id
        .parse::<SessionId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, "chat"))?;
    let session = state
        .sessions
        .get_by_id(&project_id, &session_id)
        .await
        .map_err(|error| map_session_storage(error, "chat"))?
        .ok_or_else(|| ChatBridgeError::new(ChatBridgeErrorCode::NotFound, "chat"))?;
    if session.agent_id != agent_id {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, "chat"));
    }
    let limit = input.limit.unwrap_or(MAX_MESSAGE_PAGE).clamp(1, MAX_MESSAGE_PAGE);
    let offset = input.offset.unwrap_or(0).min(10_000);
    let messages = state
        .messages
        .list(&project_id, &session_id, offset as u32, limit as u32)
        .await
        .map_err(|error| map_message_storage(error, "chat"))?
        .into_iter()
        .map(|message| ChatMessageSummary {
            id: message.id.to_string(),
            role: match message.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
                MessageRole::Tool => "tool",
                MessageRole::ToolResult => "tool_result",
            },
            text: message.content,
            status: message.status,
            sequence: message.sequence,
            generation: message.generation,
        })
        .collect();
    Ok(ListChatMessagesOutput { messages, limit, offset })
}

#[tauri::command]
pub async fn get_chat_usage(
    state: State<'_, ChatBridgeState>,
    input: GetChatUsageInput,
) -> Result<GetChatUsageOutput, ChatBridgeError> {
    if input.caller.caller_id != DESKTOP_CALLER_ID || input.caller.class != DESKTOP_CALLER_CLASS {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, "chat"));
    }
    let project_id = input
        .project_id
        .parse::<ProjectId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, "chat"))?;
    let agent_id = input
        .agent_id
        .parse::<agent_core::ids::AgentId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, "chat"))?;
    let session_id = input
        .session_id
        .parse::<SessionId>()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::InvalidCommand, "chat"))?;
    let session = state
        .sessions
        .get_by_id(&project_id, &session_id)
        .await
        .map_err(|error| map_session_storage(error, "chat"))?
        .ok_or_else(|| ChatBridgeError::new(ChatBridgeErrorCode::NotFound, "chat"))?;
    if session.agent_id != agent_id {
        return Err(ChatBridgeError::new(ChatBridgeErrorCode::Unauthorized, "chat"));
    }
    let usage = state
        .usage
        .lock()
        .map_err(|_| ChatBridgeError::new(ChatBridgeErrorCode::Internal, "chat"))?
        .read_model(&project_id, &agent_id, &session_id);
    Ok(GetChatUsageOutput { usage })
}

fn map_command_error(error: ChatCommandError) -> ChatBridgeErrorCode {
    match error {
        ChatCommandError::Invalid => ChatBridgeErrorCode::InvalidCommand,
        ChatCommandError::Capacity => ChatBridgeErrorCode::Internal,
        ChatCommandError::Lock => ChatBridgeErrorCode::Internal,
    }
}

fn map_message_storage(error: MessageStorageError, correlation_id: &str) -> ChatBridgeError {
    let code = match error {
        MessageStorageError::ScopeMismatch => ChatBridgeErrorCode::Unauthorized,
        MessageStorageError::Invalid
        | MessageStorageError::DuplicateSequence
        | MessageStorageError::OutOfOrder { .. }
        | MessageStorageError::StaleGeneration => ChatBridgeErrorCode::InvalidCommand,
        MessageStorageError::NotFound
        | MessageStorageError::Conflict
        | MessageStorageError::Serialization(_)
        | MessageStorageError::Database(_) => ChatBridgeErrorCode::Storage,
    };
    ChatBridgeError::new(code, correlation_id)
}

fn map_session_storage(error: SessionStorageError, correlation_id: &str) -> ChatBridgeError {
    let code = match error {
        SessionStorageError::ScopeMismatch => ChatBridgeErrorCode::Unauthorized,
        SessionStorageError::NotFound => ChatBridgeErrorCode::NotFound,
        SessionStorageError::Invalid => ChatBridgeErrorCode::InvalidCommand,
        SessionStorageError::Conflict
        | SessionStorageError::Serialization(_)
        | SessionStorageError::Database(_) => ChatBridgeErrorCode::Storage,
    };
    ChatBridgeError::new(code, correlation_id)
}

fn map_domain_error(error: agent_core::error::DomainError, correlation_id: &str) -> ChatBridgeError {
    let code = match error {
        agent_core::error::DomainError::NotFound(_) => ChatBridgeErrorCode::NotFound,
        agent_core::error::DomainError::PermissionDenied { .. } => ChatBridgeErrorCode::Unauthorized,
        agent_core::error::DomainError::Validation(_) => ChatBridgeErrorCode::InvalidCommand,
        _ => ChatBridgeErrorCode::Storage,
    };
    ChatBridgeError::new(code, correlation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_provider_is_explicitly_opt_in_for_release_builds() {
        assert_eq!(MOCK_PROVIDER_ENV, "HANK_E2E_MOCK_PROVIDER");
        let source = include_str!("chat.rs");
        assert!(source.contains("cfg!(debug_assertions)"));
        assert!(source.contains("ProviderUnavailable"));
        assert!(source.contains("HANK_E2E_MOCK_PROVIDER"));
    }

    #[test]
    fn normalized_fixture_result_has_explicit_provider_capability_metadata() {
        let output = chat_output("command-1", "stream-command-1".into(), "completed");
        assert_eq!(output.provider_id, MOCK_PROVIDER_ID);
        assert_eq!(output.model_id, MOCK_MODEL_ID);
        assert_eq!(output.provider_state, "selected");
        assert_eq!(output.capability, "confirmed");
        assert_eq!(output.attempt_number, 1);
    }

    #[test]
    fn chat_does_not_autoconnect_fixture_credentials() {
        let source = include_str!("chat.rs");
        assert!(!source.contains(".credentials\n        .connect"));
        assert!(source.contains(".credentials\n        .resolve_ref"));
    }
}
