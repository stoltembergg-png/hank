//! Provider-neutral OAuth authorization flow framework.

pub mod callback;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use provider_core::{CredentialRef, ProviderId};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use thiserror::Error;

const MAX_URI_LEN: usize = 512;
const MAX_STATE_LEN: usize = 128;
const MAX_CODE_LEN: usize = 2048;
const MAX_VERIFIER_LEN: usize = 128;
const MAX_ACTIVE_FLOWS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct OAuthFlowId(u64);

impl OAuthFlowId {
    pub fn parse(value: &str) -> Result<Self, OAuthError> {
        let number = value
            .strip_prefix("flow_")
            .ok_or(OAuthError::NotFound)?
            .parse::<u64>()
            .map_err(|_| OAuthError::NotFound)?;
        if number == 0 {
            return Err(OAuthError::NotFound);
        }
        Ok(Self(number))
    }

    pub fn as_str(&self) -> String {
        format!("flow_{}", self.0)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct OAuthState(String);

impl OAuthState {
    pub fn parse(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        validate_token(&value, "state_", MAX_STATE_LEN)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OAuthState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OAuthState([REDACTED])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct PkceVerifier(String);

impl PkceVerifier {
    pub fn parse(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        if !(43..=MAX_VERIFIER_LEN).contains(&value.len())
            || value
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !"-._~".contains(c))
        {
            return Err(OAuthError::InvalidPkce);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PkceVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PkceVerifier([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeChallenge(String);

impl CodeChallenge {
    pub fn parse(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        if value.len() != 43
            || value
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && !"-_".contains(c))
        {
            return Err(OAuthError::InvalidPkce);
        }
        Ok(Self(value))
    }

    pub fn from_verifier(verifier: &PkceVerifier) -> Self {
        let digest = Sha256::digest(verifier.as_str().as_bytes());
        Self(URL_SAFE_NO_PAD.encode(digest))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationCode(String);

impl AuthorizationCode {
    pub fn parse(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        if value.trim().is_empty()
            || value.len() > MAX_CODE_LEN
            || value.chars().any(char::is_control)
        {
            return Err(OAuthError::MalformedCode);
        }
        Ok(Self(value))
    }
}

impl fmt::Debug for AuthorizationCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthorizationCode([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectUri(String);

impl RedirectUri {
    pub fn parse(value: impl Into<String>) -> Result<Self, OAuthError> {
        let value = value.into();
        let valid_scheme = value.starts_with("https://")
            || value.starts_with("http://localhost")
            || value.starts_with("http://127.0.0.1")
            || value.starts_with("http://[::1]");
        if !valid_scheme
            || value.trim() != value
            || value.len() > MAX_URI_LEN
            || value.chars().any(char::is_control)
            || value
                .split("//")
                .nth(1)
                .and_then(|v| v.split('/').next())
                .unwrap_or_default()
                .contains('@')
        {
            return Err(OAuthError::InvalidRedirect);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct OAuthFlowContext {
    pub now_ms: u64,
    pub deadline_ms: u64,
    pub cancellation: provider_core::CancellationToken,
}

impl OAuthFlowContext {
    pub fn new(
        now_ms: u64,
        deadline_ms: u64,
        cancellation: provider_core::CancellationToken,
    ) -> Result<Self, OAuthError> {
        if deadline_ms <= now_ms {
            return Err(OAuthError::Expired);
        }
        Ok(Self {
            now_ms,
            deadline_ms,
            cancellation,
        })
    }

    fn check(&self) -> Result<(), OAuthError> {
        if self.cancellation.is_cancelled() {
            return Err(OAuthError::Cancelled);
        }
        if self.now_ms >= self.deadline_ms {
            return Err(OAuthError::Expired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizationRequest {
    pub flow_id: OAuthFlowId,
    pub provider_id: ProviderId,
    pub redirect_uri: RedirectUri,
    pub state: OAuthState,
    pub code_challenge: CodeChallenge,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct OAuthCallback {
    pub state: OAuthState,
    pub redirect_uri: RedirectUri,
    pub code: AuthorizationCode,
}

impl OAuthCallback {
    pub fn new(
        state: OAuthState,
        redirect_uri: RedirectUri,
        code: AuthorizationCode,
    ) -> Result<Self, OAuthError> {
        Ok(Self {
            state,
            redirect_uri,
            code,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OAuthError {
    #[error("OAuth flow is not found")]
    NotFound,
    #[error("OAuth state does not match")]
    StateMismatch,
    #[error("OAuth redirect URI does not match")]
    RedirectMismatch,
    #[error("OAuth flow was already consumed")]
    Replay,
    #[error("OAuth flow expired")]
    Expired,
    #[error("OAuth operation was cancelled")]
    Cancelled,
    #[error("OAuth flow capacity is exhausted")]
    Capacity,
    #[error("OAuth redirect URI is invalid")]
    InvalidRedirect,
    #[error("OAuth PKCE value is invalid")]
    InvalidPkce,
    #[error("OAuth authorization code is malformed")]
    MalformedCode,
    #[error("OAuth token exchange returned a malformed token")]
    MalformedToken,
    #[error("OAuth token exchange failed")]
    ExchangeFailed,
}

pub trait TokenExchangeBackend: Send + Sync {
    fn exchange(
        &self,
        provider_id: &ProviderId,
        code: AuthorizationCode,
        verifier: PkceVerifier,
    ) -> Result<CredentialRef, OAuthError>;
}

struct FlowRecord {
    provider_id: ProviderId,
    redirect_uri: RedirectUri,
    state: OAuthState,
    code_challenge: CodeChallenge,
    expires_at_ms: u64,
    used: bool,
}

pub struct OAuthFlowManager<E> {
    exchange: E,
    next_flow: AtomicU64,
    flows: Mutex<BTreeMap<OAuthFlowId, FlowRecord>>,
}

impl<E: TokenExchangeBackend> OAuthFlowManager<E> {
    pub fn new(exchange: E) -> Self {
        Self {
            exchange,
            next_flow: AtomicU64::new(1),
            flows: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn begin(
        &self,
        provider_id: ProviderId,
        redirect_uri: RedirectUri,
        state: OAuthState,
        code_challenge: CodeChallenge,
        context: OAuthFlowContext,
    ) -> Result<AuthorizationRequest, OAuthError> {
        context.check()?;
        let flow_id = OAuthFlowId(self.next_flow.fetch_add(1, Ordering::Relaxed));
        let record = FlowRecord {
            provider_id: provider_id.clone(),
            redirect_uri: redirect_uri.clone(),
            state: state.clone(),
            code_challenge: code_challenge.clone(),
            expires_at_ms: context.deadline_ms,
            used: false,
        };
        let mut flows = self.flows.lock().map_err(|_| OAuthError::ExchangeFailed)?;
        flows.retain(|_, record| record.expires_at_ms > context.now_ms);
        if flows.len() >= MAX_ACTIVE_FLOWS {
            return Err(OAuthError::Capacity);
        }
        flows.insert(flow_id, record);
        Ok(AuthorizationRequest {
            flow_id,
            provider_id,
            redirect_uri,
            state,
            code_challenge,
            expires_at_ms: context.deadline_ms,
        })
    }

    pub fn complete(
        &self,
        flow_id: OAuthFlowId,
        callback: OAuthCallback,
        verifier: PkceVerifier,
        context: OAuthFlowContext,
    ) -> Result<CredentialRef, OAuthError> {
        context.check()?;
        let mut flows = self.flows.lock().map_err(|_| OAuthError::ExchangeFailed)?;
        let record = flows.get_mut(&flow_id).ok_or(OAuthError::NotFound)?;
        if record.used {
            return Err(OAuthError::Replay);
        }
        if context.now_ms >= record.expires_at_ms {
            return Err(OAuthError::Expired);
        }
        if callback.state != record.state {
            return Err(OAuthError::StateMismatch);
        }
        if callback.redirect_uri != record.redirect_uri {
            return Err(OAuthError::RedirectMismatch);
        }
        if CodeChallenge::from_verifier(&verifier) != record.code_challenge {
            return Err(OAuthError::InvalidPkce);
        }
        record.used = true;
        self.exchange
            .exchange(&record.provider_id, callback.code, verifier)
    }
}

fn validate_token(value: &str, prefix: &str, max_len: usize) -> Result<(), OAuthError> {
    if value.len() <= prefix.len()
        || value.len() > max_len
        || !value.starts_with(prefix)
        || value.chars().any(char::is_control)
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(OAuthError::InvalidPkce);
    }
    Ok(())
}
