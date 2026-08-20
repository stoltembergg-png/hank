//! Provider-neutral credential service boundary.
//!
//! This module deliberately accepts only opaque [`CredentialRef`] values. It
//! does not persist, retrieve, or inspect secret material. Secure storage is a
//! later boundary and must implement this contract without plaintext fallback.

use crate::{CancellationToken, CredentialRef, ModelProviderError, ProviderId};
use std::collections::BTreeMap;
use std::sync::RwLock;
use thiserror::Error;

const MAX_SCOPE_ID_LEN: usize = 128;
const MAX_ACCOUNT_ID_LEN: usize = 128;
const MAX_ACTOR_ID_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ProjectScopeId(String);

impl ProjectScopeId {
    pub fn parse(value: impl Into<String>) -> Result<Self, CredentialServiceError> {
        let value = value.into();
        validate_prefixed(&value, "project_", MAX_SCOPE_ID_LEN)
            .map_err(|_| CredentialServiceError::InvalidIdentity)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct AccountId(String);

impl AccountId {
    pub fn parse(value: impl Into<String>) -> Result<Self, CredentialServiceError> {
        let value = value.into();
        validate_prefixed(&value, "account_", MAX_ACCOUNT_ID_LEN)
            .map_err(|_| CredentialServiceError::InvalidIdentity)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct CredentialAccount {
    pub project_id: ProjectScopeId,
    pub provider_id: ProviderId,
    pub account_id: AccountId,
}

impl CredentialAccount {
    pub fn new(
        project_id: ProjectScopeId,
        provider_id: ProviderId,
        account_id: AccountId,
    ) -> Result<Self, CredentialServiceError> {
        Ok(Self {
            project_id,
            provider_id,
            account_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CredentialAccessContext {
    pub project_id: ProjectScopeId,
    pub actor_id: String,
    pub cancellation: CancellationToken,
}

impl CredentialAccessContext {
    pub fn new(
        project_id: ProjectScopeId,
        actor_id: String,
        cancellation: CancellationToken,
    ) -> Result<Self, CredentialServiceError> {
        if actor_id.trim().is_empty()
            || actor_id.len() > MAX_ACTOR_ID_LEN
            || actor_id.chars().any(char::is_control)
        {
            return Err(CredentialServiceError::InvalidIdentity);
        }
        Ok(Self {
            project_id,
            actor_id,
            cancellation,
        })
    }

    fn validate(&self, account: &CredentialAccount) -> Result<(), CredentialServiceError> {
        if self.cancellation.is_cancelled() {
            return Err(CredentialServiceError::Cancelled);
        }
        if self.project_id != account.project_id {
            return Err(CredentialServiceError::Unauthorized);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialServiceState {
    Connected,
    Revoked,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialStatus {
    pub account: CredentialAccount,
    pub state: CredentialServiceState,
    pub credential_ref: Option<CredentialRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CredentialServiceError {
    #[error("credential identity is invalid")]
    InvalidIdentity,
    #[error("credential access is unauthorized")]
    Unauthorized,
    #[error("credential is missing")]
    Missing,
    #[error("credential is revoked")]
    Revoked,
    #[error("credential service is unavailable")]
    Unavailable,
    #[error("credential operation was cancelled")]
    Cancelled,
    #[error("credential account already has an active connection")]
    Conflict,
    #[error("credential reference is invalid")]
    InvalidReference,
    #[error("credential service internal state is unavailable")]
    Internal,
}

impl From<ModelProviderError> for CredentialServiceError {
    fn from(error: ModelProviderError) -> Self {
        match error {
            ModelProviderError::InvalidCredentialRef => Self::InvalidReference,
            _ => Self::InvalidIdentity,
        }
    }
}

pub trait CredentialService: Send + Sync {
    fn connect(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        credential_ref: CredentialRef,
    ) -> Result<CredentialStatus, CredentialServiceError>;

    fn disconnect(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialStatus, CredentialServiceError>;

    fn status(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialStatus, CredentialServiceError>;

    fn resolve_ref(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialRef, CredentialServiceError>;
}

#[derive(Debug, Clone)]
struct CredentialRecord {
    state: CredentialServiceState,
    credential_ref: Option<CredentialRef>,
}

/// Deterministic contract implementation. It stores only opaque refs and
/// metadata; it is not a secure persistence backend.
pub struct InMemoryCredentialService {
    available: bool,
    records: RwLock<BTreeMap<CredentialAccount, CredentialRecord>>,
}

impl InMemoryCredentialService {
    pub fn new() -> Self {
        Self {
            available: true,
            records: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn unavailable() -> Self {
        Self {
            available: false,
            records: RwLock::new(BTreeMap::new()),
        }
    }

    fn ensure_available(&self) -> Result<(), CredentialServiceError> {
        if self.available {
            Ok(())
        } else {
            Err(CredentialServiceError::Unavailable)
        }
    }

    fn status_from(account: CredentialAccount, record: &CredentialRecord) -> CredentialStatus {
        CredentialStatus {
            account,
            state: record.state,
            credential_ref: record.credential_ref.clone(),
        }
    }
}

impl Default for InMemoryCredentialService {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialService for InMemoryCredentialService {
    fn connect(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
        credential_ref: CredentialRef,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        context.validate(&account)?;
        self.ensure_available()?;
        let mut records = self
            .records
            .write()
            .map_err(|_| CredentialServiceError::Internal)?;
        if let Some(existing) = records.get(&account) {
            if existing.state == CredentialServiceState::Connected {
                return Err(CredentialServiceError::Conflict);
            }
        }
        let record = CredentialRecord {
            state: CredentialServiceState::Connected,
            credential_ref: Some(credential_ref),
        };
        records.insert(account.clone(), record.clone());
        Ok(Self::status_from(account, &record))
    }

    fn disconnect(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        context.validate(&account)?;
        self.ensure_available()?;
        let mut records = self
            .records
            .write()
            .map_err(|_| CredentialServiceError::Internal)?;
        let record = records
            .get_mut(&account)
            .ok_or(CredentialServiceError::Missing)?;
        record.state = CredentialServiceState::Revoked;
        record.credential_ref = None;
        Ok(Self::status_from(account, record))
    }

    fn status(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialStatus, CredentialServiceError> {
        context.validate(&account)?;
        self.ensure_available()?;
        let records = self
            .records
            .read()
            .map_err(|_| CredentialServiceError::Internal)?;
        let record = records
            .get(&account)
            .ok_or(CredentialServiceError::Missing)?;
        Ok(Self::status_from(account, record))
    }

    fn resolve_ref(
        &self,
        context: CredentialAccessContext,
        account: CredentialAccount,
    ) -> Result<CredentialRef, CredentialServiceError> {
        let status = self.status(context, account)?;
        if status.state != CredentialServiceState::Connected {
            return Err(CredentialServiceError::Revoked);
        }
        status.credential_ref.ok_or(CredentialServiceError::Revoked)
    }
}

fn validate_prefixed(value: &str, prefix: &str, max_len: usize) -> Result<(), ()> {
    if value.len() <= prefix.len()
        || value.len() > max_len
        || !value.starts_with(prefix)
        || value.chars().any(char::is_control)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(());
    }
    Ok(())
}
