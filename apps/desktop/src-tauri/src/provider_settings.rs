//! Provider-account bridge for the desktop shell.
//!
//! The bridge is intentionally fail-closed in production.  The only backend
//! currently enabled here is the explicit MockProvider fixture used by the
//! desktop contract/E2E suite.  Real provider credentials must be added
//! through the secure-secret backend before this module is enabled for a
//! release build; no token or secret crosses this boundary.

use agent_core::ids::ProjectId;
use agent_core::project::{Project, ProjectRepository, ProjectStatus};
use agent_runtime::project_repo::SqliteProjectRepository;
use agent_runtime::SqliteStorage;
use auth_core::callback::{CallbackError, OAuthCallbackHandler, CallbackUrl};
use auth_core::{
    AuthorizationCode, CodeChallenge, OAuthError, OAuthFlowContext, OAuthState, PkceVerifier,
    RedirectUri, TokenExchangeBackend,
};
use provider_core::credentials::{
    AccountId, CredentialAccessContext, CredentialAccount, CredentialService,
    CredentialServiceError, InMemoryCredentialService, ProjectScopeId,
};
use provider_core::{CancellationToken, CredentialRef, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use uuid::Uuid;

const MOCK_PROVIDER_ID: &str = "mock";
const MOCK_ACCOUNT_ID: &str = "account_mock";
const MOCK_CREDENTIAL_REF: &str = "cred_oauth_fixture";
const MOCK_REDIRECT_URI: &str = "http://127.0.0.1/hank/oauth/callback";
const MOCK_AUTHORIZATION_URI: &str = "hank://oauth/authorize";
const MOCK_ACTOR_ID: &str = "desktop-webview";
const MOCK_ENV: &str = "HANK_E2E_MOCK_PROVIDER";
const OAUTH_TTL_MS: u64 = 5 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAccountState {
    Connected,
    Pending,
    Revoked,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderAccountStatus {
    pub provider_id: String,
    pub account_id: String,
    pub display_name: String,
    pub state: ProviderAccountState,
    pub has_credential_ref: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthFlowState {
    Pending,
    Connected,
    Invalid,
    Expired,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthErrorCode {
    StateMismatch,
    RedirectMismatch,
    ProviderMismatch,
    AccountMismatch,
    Stale,
    Replay,
    Expired,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthStartResult {
    pub flow_id: String,
    pub state: OAuthFlowState,
    pub authorization_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthAccountStatus {
    #[serde(flatten)]
    pub status: ProviderAccountStatus,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OAuthFlowStatus {
    pub flow_id: String,
    pub state: OAuthFlowState,
    pub error_code: Option<OAuthErrorCode>,
    pub account: Option<OAuthAccountStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListProviderAccountsInput {
    pub project_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartProviderOAuthInput {
    pub project_id: String,
    pub provider_id: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthStatusInput {
    pub project_id: String,
    pub flow_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisconnectProviderInput {
    pub project_id: String,
    pub provider_id: String,
    pub account_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompleteProviderOAuthInput {
    pub project_id: String,
    pub callback_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSettingsErrorCode {
    InvalidInput,
    Unauthorized,
    Unavailable,
    NotFound,
    Conflict,
    StateMismatch,
    RedirectMismatch,
    ProviderMismatch,
    AccountMismatch,
    Replay,
    Expired,
    Stale,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ProviderSettingsBridgeError {
    pub code: ProviderSettingsErrorCode,
    pub message: String,
}

impl std::fmt::Display for ProviderSettingsBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ProviderSettingsBridgeError {}

impl ProviderSettingsBridgeError {
    fn new(code: ProviderSettingsErrorCode, message: &'static str) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct AccountKey {
    project_id: String,
    provider_id: String,
    account_id: String,
}

#[derive(Debug, Clone)]
struct AccountRecord {
    account: CredentialAccount,
    status: ProviderAccountState,
    credential_ref: Option<CredentialRef>,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct FlowRecord {
    project_scope: ProjectScopeId,
    account_key: AccountKey,
    account: CredentialAccount,
    verifier: PkceVerifier,
    expires_at_ms: u64,
    status: OAuthFlowState,
    error_code: Option<OAuthErrorCode>,
}

struct FixtureTokenExchange;

impl TokenExchangeBackend for FixtureTokenExchange {
    fn exchange(
        &self,
        provider_id: &ProviderId,
        _code: AuthorizationCode,
        _verifier: PkceVerifier,
    ) -> Result<CredentialRef, OAuthError> {
        if provider_id.as_str() != MOCK_PROVIDER_ID {
            return Err(OAuthError::ExchangeFailed);
        }
        CredentialRef::parse(MOCK_CREDENTIAL_REF).map_err(|_| OAuthError::MalformedToken)
    }
}

#[derive(Clone)]
pub struct ProviderSettingsBridgeState {
    projects: Arc<SqliteProjectRepository>,
    credentials: Arc<InMemoryCredentialService>,
    oauth: Arc<OAuthCallbackHandler<FixtureTokenExchange>>,
    accounts: Arc<Mutex<BTreeMap<AccountKey, AccountRecord>>>,
    flows: Arc<Mutex<BTreeMap<String, FlowRecord>>>,
    enabled: bool,
}

impl ProviderSettingsBridgeState {
    pub fn new(
        storage: &SqliteStorage,
        credentials: Arc<InMemoryCredentialService>,
    ) -> Self {
        Self {
            projects: Arc::new(SqliteProjectRepository::new(storage.pool().clone())),
            credentials,
            oauth: Arc::new(OAuthCallbackHandler::new(FixtureTokenExchange)),
            accounts: Arc::new(Mutex::new(BTreeMap::new())),
            flows: Arc::new(Mutex::new(BTreeMap::new())),
            enabled: cfg!(debug_assertions)
                || std::env::var(MOCK_ENV).ok().as_deref() == Some("1"),
        }
    }

    pub fn enabled_for_fixture(&self) -> bool {
        self.enabled
    }

    fn ensure_enabled(&self) -> Result<(), ProviderSettingsBridgeError> {
        if self.enabled {
            Ok(())
        } else {
            Err(ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::Unavailable,
                "provider credential service is unavailable",
            ))
        }
    }

    async fn load_project(
        &self,
        project_id: &str,
    ) -> Result<(Project, ProjectScopeId), ProviderSettingsBridgeError> {
        let parsed = project_id.parse::<ProjectId>().map_err(|_| {
            ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::InvalidInput,
                "invalid project id",
            )
        })?;
        let project = self
            .projects
            .get_by_id(&parsed)
            .await
            .map_err(|_| {
                ProviderSettingsBridgeError::new(
                    ProviderSettingsErrorCode::Internal,
                    "could not load project",
                )
            })?
            .ok_or_else(|| {
                ProviderSettingsBridgeError::new(
                    ProviderSettingsErrorCode::NotFound,
                    "project not found",
                )
            })?;
        if project.status != ProjectStatus::Active {
            return Err(ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::Unauthorized,
                "project is not active",
            ));
        }
        let scope = ProjectScopeId::parse(format!("project_{parsed}"))
            .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "invalid project scope"))?;
        Ok((project, scope))
    }

    fn fixture_account(
        scope: &ProjectScopeId,
    ) -> Result<CredentialAccount, ProviderSettingsBridgeError> {
        CredentialAccount::new(
            scope.clone(),
            ProviderId::parse(MOCK_PROVIDER_ID).map_err(|_| {
                ProviderSettingsBridgeError::new(
                    ProviderSettingsErrorCode::Internal,
                    "invalid provider fixture",
                )
            })?,
            AccountId::parse(MOCK_ACCOUNT_ID).map_err(|_| {
                ProviderSettingsBridgeError::new(
                    ProviderSettingsErrorCode::Internal,
                    "invalid account fixture",
                )
            })?,
        )
        .map_err(|_| {
            ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::Internal,
                "invalid provider account",
            )
        })
    }

    fn key(account: &CredentialAccount) -> AccountKey {
        AccountKey {
            project_id: account.project_id.as_str().to_string(),
            provider_id: account.provider_id.as_str().to_string(),
            account_id: account.account_id.as_str().to_string(),
        }
    }

    fn status(record: &AccountRecord) -> ProviderAccountStatus {
        ProviderAccountStatus {
            provider_id: record.account.provider_id.as_str().to_string(),
            account_id: record.account.account_id.as_str().to_string(),
            display_name: "Mock Provider".to_string(),
            state: record.status,
            has_credential_ref: record.credential_ref.is_some(),
            updated_at: record.updated_at.clone(),
        }
    }

    fn access(
        scope: ProjectScopeId,
        cancellation: CancellationToken,
    ) -> Result<CredentialAccessContext, ProviderSettingsBridgeError> {
        CredentialAccessContext::new(scope, MOCK_ACTOR_ID.to_string(), cancellation).map_err(|_| {
            ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::Unauthorized,
                "provider access is unauthorized",
            )
        })
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0)
    }

    fn new_flow_material() -> Result<(OAuthState, PkceVerifier, CodeChallenge), ProviderSettingsBridgeError> {
        let state = OAuthState::parse(format!("state_{}", Uuid::new_v4().simple()))
            .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "could not create OAuth state"))?;
        let verifier = PkceVerifier::parse(format!(
            "{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        ))
        .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "could not create PKCE verifier"))?;
        let challenge = CodeChallenge::from_verifier(&verifier);
        Ok((state, verifier, challenge))
    }

    fn authorization_url(flow_id: &str, state: &OAuthState, challenge: &CodeChallenge) -> String {
        format!(
            "{MOCK_AUTHORIZATION_URI}?flow={flow_id}&provider={MOCK_PROVIDER_ID}&account={MOCK_ACCOUNT_ID}&state={}&code_challenge={}",
            state.as_str(),
            challenge.as_str()
        )
    }

    fn validate_fixture_account(
        provider_id: &str,
        account_id: &str,
    ) -> Result<(), ProviderSettingsBridgeError> {
        if provider_id != MOCK_PROVIDER_ID {
            return Err(ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::ProviderMismatch,
                "provider is not available",
            ));
        }
        if account_id != MOCK_ACCOUNT_ID {
            return Err(ProviderSettingsBridgeError::new(
                ProviderSettingsErrorCode::AccountMismatch,
                "provider account is not available",
            ));
        }
        Ok(())
    }
}

pub fn bridge_state(
    storage: &SqliteStorage,
    credentials: Arc<InMemoryCredentialService>,
) -> ProviderSettingsBridgeState {
    ProviderSettingsBridgeState::new(storage, credentials)
}

#[tauri::command]
pub async fn list_provider_accounts(
    state: State<'_, ProviderSettingsBridgeState>,
    input: ListProviderAccountsInput,
) -> Result<Vec<ProviderAccountStatus>, ProviderSettingsBridgeError> {
    state.ensure_enabled()?;
    let (_project, scope) = state.load_project(&input.project_id).await?;
    let account = ProviderSettingsBridgeState::fixture_account(&scope)?;
    let key = ProviderSettingsBridgeState::key(&account);
    let mut accounts = state.accounts.lock().map_err(|_| {
        ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
    })?;
    let record = accounts.entry(key).or_insert_with(|| AccountRecord {
        account,
        status: ProviderAccountState::Revoked,
        credential_ref: None,
        updated_at: chrono::Utc::now().to_rfc3339(),
    });
    Ok(vec![ProviderSettingsBridgeState::status(record)])
}

#[tauri::command]
pub async fn start_provider_oauth(
    state: State<'_, ProviderSettingsBridgeState>,
    input: StartProviderOAuthInput,
) -> Result<OAuthStartResult, ProviderSettingsBridgeError> {
    state.ensure_enabled()?;
    let (_project, scope) = state.load_project(&input.project_id).await?;
    ProviderSettingsBridgeState::validate_fixture_account(&input.provider_id, &input.account_id)?;
    let account = ProviderSettingsBridgeState::fixture_account(&scope)?;
    let key = ProviderSettingsBridgeState::key(&account);
    {
        let accounts = state.accounts.lock().map_err(|_| {
            ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
        })?;
        if let Some(record) = accounts.get(&key) {
            if matches!(record.status, ProviderAccountState::Connected | ProviderAccountState::Pending) {
                return Err(ProviderSettingsBridgeError::new(
                    ProviderSettingsErrorCode::Conflict,
                    "provider account already has a pending or active connection",
                ));
            }
        }
    }
    let (oauth_state, verifier, challenge) = ProviderSettingsBridgeState::new_flow_material()?;
    let now = ProviderSettingsBridgeState::now_ms();
    let flow_context = OAuthFlowContext::new(now, now.saturating_add(OAUTH_TTL_MS), CancellationToken::new())
        .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "could not create OAuth flow"))?;
    let access = ProviderSettingsBridgeState::access(scope.clone(), CancellationToken::new())?;
    let redirect = RedirectUri::parse(MOCK_REDIRECT_URI)
        .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "invalid OAuth redirect"))?;
    let request = state
        .oauth
        .begin(account.clone(), redirect, oauth_state, challenge, access, flow_context)
        .map_err(map_callback_error)?;
    let flow_id = request.flow_id.as_str();
    state
        .flows
        .lock()
        .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable"))?
        .insert(
            flow_id.clone(),
            FlowRecord {
                project_scope: scope,
                account_key: key.clone(),
                account: account.clone(),
                verifier,
                expires_at_ms: request.expires_at_ms,
                status: OAuthFlowState::Pending,
                error_code: None,
            },
        );
    let mut accounts = state.accounts.lock().map_err(|_| {
        ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
    })?;
    let record = accounts.entry(key).or_insert_with(|| AccountRecord {
        account: account.clone(),
        status: ProviderAccountState::Revoked,
        credential_ref: None,
        updated_at: chrono::Utc::now().to_rfc3339(),
    });
    record.status = ProviderAccountState::Pending;
    record.updated_at = chrono::Utc::now().to_rfc3339();
    Ok(OAuthStartResult {
        flow_id: flow_id.clone(),
        state: OAuthFlowState::Pending,
        authorization_url: ProviderSettingsBridgeState::authorization_url(
            flow_id.as_str(),
            &request.state,
            &request.code_challenge,
        ),
    })
}

#[tauri::command]
pub async fn get_provider_oauth_status(
    state: State<'_, ProviderSettingsBridgeState>,
    input: OAuthStatusInput,
) -> Result<OAuthFlowStatus, ProviderSettingsBridgeError> {
    state.ensure_enabled()?;
    let (_project, scope) = state.load_project(&input.project_id).await?;
    let mut flows = state.flows.lock().map_err(|_| {
        ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
    })?;
    let flow = flows.get_mut(&input.flow_id).ok_or_else(|| {
        ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Stale, "OAuth flow is stale")
    })?;
    if flow.project_scope != scope {
        return Err(ProviderSettingsBridgeError::new(
            ProviderSettingsErrorCode::Unauthorized,
            "OAuth flow belongs to another project",
        ));
    }
    if flow.status == OAuthFlowState::Pending && ProviderSettingsBridgeState::now_ms() >= flow.expires_at_ms {
        flow.status = OAuthFlowState::Expired;
        flow.error_code = Some(OAuthErrorCode::Expired);
    }
    let account = if flow.status == OAuthFlowState::Connected {
        let accounts = state.accounts.lock().map_err(|_| {
            ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
        })?;
        accounts.get(&flow.account_key).map(|record| OAuthAccountStatus {
            status: ProviderSettingsBridgeState::status(record),
            project_id: input.project_id.clone(),
        })
    } else {
        None
    };
    Ok(OAuthFlowStatus {
        flow_id: input.flow_id,
        state: flow.status,
        error_code: flow.error_code,
        account,
    })
}

#[tauri::command]
pub async fn complete_provider_oauth(
    state: State<'_, ProviderSettingsBridgeState>,
    input: CompleteProviderOAuthInput,
) -> Result<OAuthFlowStatus, ProviderSettingsBridgeError> {
    state.ensure_enabled()?;
    let (_project, scope) = state.load_project(&input.project_id).await?;
    let callback = CallbackUrl::parse(&input.callback_url).map_err(map_callback_error)?;
    let flow_id = callback.flow_id.as_str();
    let flow = state
        .flows
        .lock()
        .map_err(|_| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable"))?
        .get(flow_id.as_str())
        .cloned()
        .ok_or_else(|| ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Stale, "OAuth flow is stale"))?;
    if flow.project_scope != scope {
        return Err(ProviderSettingsBridgeError::new(
            ProviderSettingsErrorCode::Unauthorized,
            "OAuth flow belongs to another project",
        ));
    }
    if flow.status == OAuthFlowState::Connected {
        return Err(ProviderSettingsBridgeError::new(
            ProviderSettingsErrorCode::Replay,
            "OAuth flow was already completed",
        ));
    }
    let now = ProviderSettingsBridgeState::now_ms();
    let context = OAuthFlowContext::new(now, flow.expires_at_ms, CancellationToken::new())
        .map_err(|error| map_callback_error(CallbackError::OAuth(error)))?;
    let access = ProviderSettingsBridgeState::access(scope, CancellationToken::new())?;
    let credential_ref = match state.oauth.complete(&input.callback_url, access.clone(), context, flow.verifier) {
        Ok(result) => result.credential_ref,
        Err(error) => {
            if let Ok(mut flows) = state.flows.lock() {
                if let Some(record) = flows.get_mut(flow_id.as_str()) {
                    record.status = flow_state_from_callback_error(&error);
                    record.error_code = flow_error_code(&error);
                }
            }
            return Err(map_callback_error(error));
        }
    };
    state
        .credentials
        .connect(access, flow.account.clone(), credential_ref.clone())
        .map_err(map_credential_error)?;
    let account_status = {
        let mut accounts = state.accounts.lock().map_err(|_| {
            ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
        })?;
        let record = accounts.get_mut(&flow.account_key).ok_or_else(|| {
            ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider account state unavailable")
        })?;
        record.status = ProviderAccountState::Connected;
        record.credential_ref = Some(credential_ref);
        record.updated_at = chrono::Utc::now().to_rfc3339();
        ProviderSettingsBridgeState::status(record)
    };
    if let Ok(mut flows) = state.flows.lock() {
        if let Some(record) = flows.get_mut(flow_id.as_str()) {
            record.status = OAuthFlowState::Connected;
            record.error_code = None;
        }
    }
    Ok(OAuthFlowStatus {
        flow_id,
        state: OAuthFlowState::Connected,
        error_code: None,
        account: Some(OAuthAccountStatus {
            status: account_status,
            project_id: input.project_id,
        }),
    })
}

#[tauri::command]
pub async fn disconnect_provider_account(
    state: State<'_, ProviderSettingsBridgeState>,
    input: DisconnectProviderInput,
) -> Result<ProviderAccountStatus, ProviderSettingsBridgeError> {
    state.ensure_enabled()?;
    let (_project, scope) = state.load_project(&input.project_id).await?;
    ProviderSettingsBridgeState::validate_fixture_account(&input.provider_id, &input.account_id)?;
    let account = ProviderSettingsBridgeState::fixture_account(&scope)?;
    let key = ProviderSettingsBridgeState::key(&account);
    let access = ProviderSettingsBridgeState::access(scope, CancellationToken::new())?;
    let mut accounts = state.accounts.lock().map_err(|_| {
        ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::Internal, "provider state unavailable")
    })?;
    let record = accounts.get_mut(&key).ok_or_else(|| {
        ProviderSettingsBridgeError::new(ProviderSettingsErrorCode::NotFound, "provider account not found")
    })?;
    if record.status == ProviderAccountState::Connected {
        state
            .credentials
            .disconnect(access, account)
            .map_err(map_credential_error)?;
    }
    record.status = ProviderAccountState::Revoked;
    record.credential_ref = None;
    record.updated_at = chrono::Utc::now().to_rfc3339();
    Ok(ProviderSettingsBridgeState::status(record))
}

fn map_credential_error(error: CredentialServiceError) -> ProviderSettingsBridgeError {
    let code = match error {
        CredentialServiceError::Unauthorized => ProviderSettingsErrorCode::Unauthorized,
        CredentialServiceError::Missing => ProviderSettingsErrorCode::NotFound,
        CredentialServiceError::Conflict => ProviderSettingsErrorCode::Conflict,
        CredentialServiceError::Unavailable => ProviderSettingsErrorCode::Unavailable,
        CredentialServiceError::Revoked => ProviderSettingsErrorCode::Unauthorized,
        CredentialServiceError::Cancelled
        | CredentialServiceError::InvalidIdentity
        | CredentialServiceError::InvalidReference
        | CredentialServiceError::Internal => ProviderSettingsErrorCode::Internal,
    };
    ProviderSettingsBridgeError::new(code, "provider credential operation failed")
}

fn map_callback_error(error: CallbackError) -> ProviderSettingsBridgeError {
    let code = match error {
        CallbackError::ProviderMismatch => ProviderSettingsErrorCode::ProviderMismatch,
        CallbackError::AccountMismatch => ProviderSettingsErrorCode::AccountMismatch,
        CallbackError::Unauthorized => ProviderSettingsErrorCode::Unauthorized,
        CallbackError::Cancelled => ProviderSettingsErrorCode::Unavailable,
        CallbackError::Malformed => ProviderSettingsErrorCode::InvalidInput,
        CallbackError::OAuth(ref oauth) => match oauth {
            OAuthError::StateMismatch => ProviderSettingsErrorCode::StateMismatch,
            OAuthError::RedirectMismatch => ProviderSettingsErrorCode::RedirectMismatch,
            OAuthError::Replay => ProviderSettingsErrorCode::Replay,
            OAuthError::Expired => ProviderSettingsErrorCode::Expired,
            OAuthError::NotFound => ProviderSettingsErrorCode::Stale,
            _ => ProviderSettingsErrorCode::Internal,
        },
    };
    ProviderSettingsBridgeError::new(code, "OAuth provider operation failed")
}

fn oauth_state_from_error(error: &OAuthError) -> OAuthFlowState {
    match error {
        OAuthError::Expired => OAuthFlowState::Expired,
        OAuthError::Cancelled => OAuthFlowState::Cancelled,
        OAuthError::StateMismatch
        | OAuthError::RedirectMismatch
        | OAuthError::Replay
        | OAuthError::NotFound => OAuthFlowState::Invalid,
        _ => OAuthFlowState::Error,
    }
}

fn flow_state_from_callback_error(error: &CallbackError) -> OAuthFlowState {
    match error {
        CallbackError::Cancelled => OAuthFlowState::Cancelled,
        CallbackError::OAuth(error) => oauth_state_from_error(error),
        _ => OAuthFlowState::Invalid,
    }
}

fn oauth_error_code(error: &OAuthError) -> Option<OAuthErrorCode> {
    match error {
        OAuthError::StateMismatch => Some(OAuthErrorCode::StateMismatch),
        OAuthError::RedirectMismatch => Some(OAuthErrorCode::RedirectMismatch),
        OAuthError::Replay => Some(OAuthErrorCode::Replay),
        OAuthError::Expired => Some(OAuthErrorCode::Expired),
        _ => None,
    }
}

fn flow_error_code(error: &CallbackError) -> Option<OAuthErrorCode> {
    match error {
        CallbackError::ProviderMismatch => Some(OAuthErrorCode::ProviderMismatch),
        CallbackError::AccountMismatch => Some(OAuthErrorCode::AccountMismatch),
        CallbackError::OAuth(error) => oauth_error_code(error),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_account_never_accepts_a_different_provider_or_account() {
        assert!(ProviderSettingsBridgeState::validate_fixture_account(MOCK_PROVIDER_ID, MOCK_ACCOUNT_ID).is_ok());
        assert_eq!(
            ProviderSettingsBridgeState::validate_fixture_account("openai", MOCK_ACCOUNT_ID)
                .unwrap_err()
                .code,
            ProviderSettingsErrorCode::ProviderMismatch
        );
        assert_eq!(
            ProviderSettingsBridgeState::validate_fixture_account(MOCK_PROVIDER_ID, "account_other")
                .unwrap_err()
                .code,
            ProviderSettingsErrorCode::AccountMismatch
        );
    }

    #[test]
    fn oauth_material_is_bounded_and_does_not_expose_verifier_or_state() {
        let (state, verifier, challenge) = ProviderSettingsBridgeState::new_flow_material().unwrap();
        assert!(state.as_str().starts_with("state_"));
        assert_eq!(verifier.as_str().len(), 64);
        assert_eq!(challenge.as_str().len(), 43);
        assert!(!format!("{state:?}").contains(state.as_str()));
        assert!(!format!("{verifier:?}").contains(verifier.as_str()));
    }

    #[test]
    fn fixture_authorization_url_contains_public_flow_material_only() {
        let state = OAuthState::parse("state_fixture").unwrap();
        let verifier = PkceVerifier::parse(
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_~.",
        )
        .unwrap();
        let challenge = CodeChallenge::from_verifier(&verifier);
        let url = ProviderSettingsBridgeState::authorization_url("flow_7", &state, &challenge);

        assert!(url.starts_with("hank://oauth/authorize?"));
        assert!(url.contains("flow=flow_7"));
        assert!(url.contains("provider=mock"));
        assert!(url.contains("account=account_mock"));
        assert!(url.contains("state=state_fixture"));
        assert!(url.contains(&format!("code_challenge={}", challenge.as_str())));
        assert!(!url.contains(verifier.as_str()));
    }

    #[test]
    fn callback_errors_map_to_redacted_stable_codes() {
        let error = map_callback_error(CallbackError::OAuth(OAuthError::StateMismatch));
        assert_eq!(error.code, ProviderSettingsErrorCode::StateMismatch);
        assert!(!error.message.contains("state_"));
    }
}
