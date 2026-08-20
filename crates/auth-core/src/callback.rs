//! Validated deep-link/browser callback boundary for OAuth.

use super::{
    AuthorizationCode, OAuthCallback, OAuthError, OAuthFlowContext, OAuthFlowId, OAuthFlowManager,
    OAuthState, PkceVerifier, RedirectUri, TokenExchangeBackend,
};
use provider_core::credentials::{AccountId, CredentialAccessContext, CredentialAccount};
use provider_core::{CredentialRef, ProviderId};
use std::collections::BTreeMap;
use std::sync::Mutex;
use thiserror::Error;

const MAX_CALLBACK_URL_LEN: usize = 2048;

#[derive(Debug, Clone)]
pub struct CallbackUrl {
    pub flow_id: OAuthFlowId,
    pub provider_id: ProviderId,
    pub account_id: AccountId,
    pub state: OAuthState,
    pub code: AuthorizationCode,
}

impl CallbackUrl {
    pub fn parse(value: &str) -> Result<Self, CallbackError> {
        const PREFIX: &str = "hank://oauth/callback?";
        if value.len() > MAX_CALLBACK_URL_LEN
            || !value.starts_with(PREFIX)
            || value.chars().any(char::is_control)
            || value.contains('#')
        {
            return Err(CallbackError::Malformed);
        }
        let mut fields = BTreeMap::new();
        for pair in value[PREFIX.len()..].split('&') {
            let (key, val) = pair.split_once('=').ok_or(CallbackError::Malformed)?;
            if val.is_empty() || fields.insert(key, val).is_some() {
                return Err(CallbackError::Malformed);
            }
        }
        if fields
            .keys()
            .any(|key| !matches!(*key, "flow" | "provider" | "account" | "state" | "code"))
            || fields.len() != 5
        {
            return Err(CallbackError::Malformed);
        }
        Ok(Self {
            flow_id: OAuthFlowId::parse(fields["flow"]).map_err(CallbackError::OAuth)?,
            provider_id: ProviderId::parse(fields["provider"])
                .map_err(|_| CallbackError::Malformed)?,
            account_id: AccountId::parse(fields["account"])
                .map_err(|_| CallbackError::Malformed)?,
            state: OAuthState::parse(fields["state"]).map_err(CallbackError::OAuth)?,
            code: AuthorizationCode::parse(fields["code"]).map_err(CallbackError::OAuth)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CallbackError {
    #[error("OAuth callback is malformed")]
    Malformed,
    #[error("OAuth callback provider does not match flow")]
    ProviderMismatch,
    #[error("OAuth callback account does not match flow")]
    AccountMismatch,
    #[error("OAuth callback project access is unauthorized")]
    Unauthorized,
    #[error("OAuth callback was cancelled")]
    Cancelled,
    #[error("OAuth callback flow error: {0}")]
    OAuth(#[from] OAuthError),
}

#[derive(Debug, Clone)]
pub struct OAuthCallbackResult {
    pub flow_id: OAuthFlowId,
    pub provider_id: ProviderId,
    pub account_id: AccountId,
    pub credential_ref: CredentialRef,
}

impl OAuthCallbackResult {
    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }
    pub fn account_id(&self) -> &AccountId {
        &self.account_id
    }
}

#[derive(Clone)]
struct CallbackBinding {
    account: CredentialAccount,
    redirect_uri: RedirectUri,
}

pub struct OAuthCallbackHandler<E> {
    manager: OAuthFlowManager<E>,
    bindings: Mutex<BTreeMap<OAuthFlowId, CallbackBinding>>,
}

impl<E: TokenExchangeBackend> OAuthCallbackHandler<E> {
    pub fn new(exchange: E) -> Self {
        Self {
            manager: OAuthFlowManager::new(exchange),
            bindings: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn begin(
        &self,
        account: CredentialAccount,
        redirect_uri: RedirectUri,
        state: OAuthState,
        challenge: super::CodeChallenge,
        access: CredentialAccessContext,
        flow_context: OAuthFlowContext,
    ) -> Result<super::AuthorizationRequest, CallbackError> {
        validate_access(&access, &account)?;
        let request = self
            .manager
            .begin(
                account.provider_id.clone(),
                redirect_uri.clone(),
                state,
                challenge,
                flow_context,
            )
            .map_err(CallbackError::OAuth)?;
        self.bindings
            .lock()
            .map_err(|_| CallbackError::OAuth(OAuthError::ExchangeFailed))?
            .insert(
                request.flow_id,
                CallbackBinding {
                    account,
                    redirect_uri,
                },
            );
        Ok(request)
    }

    pub fn complete(
        &self,
        callback_url: &str,
        access: CredentialAccessContext,
        flow_context: OAuthFlowContext,
        verifier: PkceVerifier,
    ) -> Result<OAuthCallbackResult, CallbackError> {
        let callback = CallbackUrl::parse(callback_url)?;
        if access.cancellation.is_cancelled() {
            return Err(CallbackError::Cancelled);
        }
        let binding = self
            .bindings
            .lock()
            .map_err(|_| CallbackError::OAuth(OAuthError::ExchangeFailed))?
            .get(&callback.flow_id)
            .cloned()
            .ok_or(CallbackError::OAuth(OAuthError::NotFound))?;
        validate_access(&access, &binding.account)?;
        if callback.provider_id != binding.account.provider_id {
            return Err(CallbackError::ProviderMismatch);
        }
        if callback.account_id != binding.account.account_id {
            return Err(CallbackError::AccountMismatch);
        }
        let oauth_callback =
            OAuthCallback::new(callback.state, binding.redirect_uri, callback.code)
                .map_err(CallbackError::OAuth)?;
        let credential_ref = self
            .manager
            .complete(callback.flow_id, oauth_callback, verifier, flow_context)
            .map_err(CallbackError::OAuth)?;
        Ok(OAuthCallbackResult {
            flow_id: callback.flow_id,
            provider_id: binding.account.provider_id,
            account_id: binding.account.account_id,
            credential_ref,
        })
    }
}

fn validate_access(
    access: &CredentialAccessContext,
    account: &CredentialAccount,
) -> Result<(), CallbackError> {
    if access.cancellation.is_cancelled() {
        return Err(CallbackError::Cancelled);
    }
    if access.project_id != account.project_id {
        return Err(CallbackError::Unauthorized);
    }
    Ok(())
}
