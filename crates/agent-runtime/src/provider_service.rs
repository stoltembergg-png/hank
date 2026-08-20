//! Single provider-neutral application/invocation boundary for Agent Runtime.

use futures_util::StreamExt;
use provider_core::capabilities::CapabilityError;
use provider_core::credentials::{
    CredentialAccessContext, CredentialAccount, CredentialService, CredentialServiceError,
};
use provider_core::fallback::{
    FallbackCandidate, FallbackFailure, FallbackPolicy, FallbackReason, FallbackTerminal,
};
use provider_core::registry::{ProviderRegistry, RegistryError};
use provider_core::request::NormalizedRequest;
use provider_core::{
    CancellationToken, FinishReason, ModelProviderError, ProviderRequest, ProviderResponse,
    StreamConfig, Usage,
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct InvocationRequest {
    pub normalized: NormalizedRequest,
    pub account: CredentialAccount,
    pub access: CredentialAccessContext,
    pub fallback_candidates: Vec<FallbackCandidate>,
}

impl InvocationRequest {
    pub fn new(
        normalized: NormalizedRequest,
        account: CredentialAccount,
        access: CredentialAccessContext,
        fallback_candidates: Vec<FallbackCandidate>,
    ) -> Result<Self, InvocationError> {
        normalized
            .validate()
            .map_err(|_| InvocationError::InvalidRequest)?;
        let project_id =
            provider_core::credentials::ProjectScopeId::parse(normalized.project_id.clone())
                .map_err(|_| InvocationError::InvalidRequest)?;
        if account.project_id != project_id
            || access.project_id != project_id
            || account.provider_id != normalized.provider_id
        {
            return Err(InvocationError::Unauthorized);
        }
        Ok(Self {
            normalized,
            account,
            access,
            fallback_candidates,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationResult {
    pub attempt_id: String,
    pub attempt_number: u32,
    pub provider_id: provider_core::ProviderId,
    pub model_id: provider_core::ModelId,
    pub text: String,
    pub finish_reason: FinishReason,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationStreamEvent {
    pub attempt_id: String,
    pub sequence: u64,
    pub text: String,
    pub terminal: bool,
}

#[derive(Debug, Error)]
pub enum InvocationError {
    #[error("provider invocation request is invalid")]
    InvalidRequest,
    #[error("provider invocation access is unauthorized")]
    Unauthorized,
    #[error("credential service rejected invocation: {0}")]
    Credential(#[from] CredentialServiceError),
    #[error("provider registry rejected invocation: {0}")]
    Registry(#[from] RegistryError),
    #[error("provider capability mismatch: {0}")]
    Capability(#[from] CapabilityError),
    #[error("provider operation failed: {0}")]
    Provider(#[from] ModelProviderError),
    #[error("provider invocation was cancelled")]
    Cancelled,
    #[error("fallback terminated invocation: {0:?}")]
    Fallback(FallbackTerminal),
    #[error("provider stream ended without a terminal event")]
    StreamIncomplete,
    #[error("provider application service state is unavailable")]
    Internal,
}

pub struct ProviderApplicationService {
    registry: Arc<ProviderRegistry>,
    credentials: Arc<dyn CredentialService>,
    fallback_policy: FallbackPolicy,
}

impl ProviderApplicationService {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        credentials: Arc<dyn CredentialService>,
        fallback_policy: FallbackPolicy,
    ) -> Self {
        Self {
            registry,
            credentials,
            fallback_policy,
        }
    }

    pub async fn complete(
        &self,
        request: InvocationRequest,
    ) -> Result<InvocationResult, InvocationError> {
        let mut current = request.normalized.clone();
        let mut account = request.account.clone();
        let mut attempts_used = 0;

        loop {
            self.ensure_not_cancelled(&request.access.cancellation)?;
            let (provider, credential_ref) =
                self.resolve_provider(&request.access, &account, &current)?;
            let attempt_number = attempts_used + 1;
            let attempt_id = attempt_id(&current.request_id, attempt_number);
            let provider_request = ProviderRequest::new(
                attempt_id.clone(),
                current.model_id.clone(),
                credential_ref,
                prompt_from(&current),
            )
            .map_err(|_| InvocationError::InvalidRequest)?;

            match provider
                .complete(provider_request, request.access.cancellation.clone())
                .await
            {
                Ok(response) => {
                    return Ok(map_response(
                        response,
                        current.provider_id.clone(),
                        attempt_id,
                        attempt_number,
                    ))
                }
                Err(ModelProviderError::Cancelled) => return Err(InvocationError::Cancelled),
                Err(error) => {
                    let decision = self.decide_fallback(
                        &request,
                        &current,
                        &account,
                        attempts_used,
                        map_provider_failure(&error),
                    )?;
                    match decision {
                        provider_core::fallback::FallbackDecision::Retry(attempt) => {
                            current.provider_id = attempt.provider_id.clone();
                            current.model_id = attempt.model_id.clone();
                            account = attempt.account;
                            attempts_used = attempt.attempt_number;
                        }
                        provider_core::fallback::FallbackDecision::Terminal(terminal) => {
                            return Err(InvocationError::Fallback(terminal))
                        }
                    }
                }
            }
        }
    }

    pub async fn stream(
        &self,
        request: InvocationRequest,
    ) -> Result<Vec<InvocationStreamEvent>, InvocationError> {
        let mut current = request.normalized.clone();
        let mut account = request.account.clone();
        let mut attempts_used = 0;

        loop {
            self.ensure_not_cancelled(&request.access.cancellation)?;
            let (provider, credential_ref) =
                self.resolve_provider(&request.access, &account, &current)?;
            let attempt_number = attempts_used + 1;
            let attempt_id = attempt_id(&current.request_id, attempt_number);
            let provider_request = ProviderRequest::new(
                attempt_id.clone(),
                current.model_id.clone(),
                credential_ref,
                prompt_from(&current),
            )
            .map_err(|_| InvocationError::InvalidRequest)?;
            let stream = match provider.stream(
                provider_request,
                request.access.cancellation.clone(),
                StreamConfig::new(1024).map_err(|_| InvocationError::InvalidRequest)?,
            ) {
                Ok(stream) => stream,
                Err(ModelProviderError::Cancelled) => return Err(InvocationError::Cancelled),
                Err(error) => {
                    let decision = self.decide_fallback(
                        &request,
                        &current,
                        &account,
                        attempts_used,
                        map_provider_failure(&error),
                    )?;
                    match decision {
                        provider_core::fallback::FallbackDecision::Retry(attempt) => {
                            current.provider_id = attempt.provider_id.clone();
                            current.model_id = attempt.model_id.clone();
                            account = attempt.account;
                            attempts_used = attempt.attempt_number;
                            continue;
                        }
                        provider_core::fallback::FallbackDecision::Terminal(terminal) => {
                            return Err(InvocationError::Fallback(terminal))
                        }
                    }
                }
            };

            match collect_stream(stream, &attempt_id, &request.access.cancellation).await {
                Ok(events) => return Ok(events),
                Err(ModelProviderError::Cancelled) => return Err(InvocationError::Cancelled),
                Err(error) => {
                    let decision = self.decide_fallback(
                        &request,
                        &current,
                        &account,
                        attempts_used,
                        map_provider_failure(&error),
                    )?;
                    match decision {
                        provider_core::fallback::FallbackDecision::Retry(attempt) => {
                            current.provider_id = attempt.provider_id.clone();
                            current.model_id = attempt.model_id.clone();
                            account = attempt.account;
                            attempts_used = attempt.attempt_number;
                        }
                        provider_core::fallback::FallbackDecision::Terminal(terminal) => {
                            return Err(InvocationError::Fallback(terminal))
                        }
                    }
                }
            }
        }
    }

    fn resolve_provider(
        &self,
        access: &CredentialAccessContext,
        account: &CredentialAccount,
        request: &NormalizedRequest,
    ) -> Result<
        (
            Arc<dyn provider_core::ModelProvider>,
            provider_core::CredentialRef,
        ),
        InvocationError,
    > {
        let credential_ref = self
            .credentials
            .resolve_ref(access.clone(), account.clone())?;
        let descriptor = self.registry.get_descriptor(&request.provider_id)?;
        request.validate_against_capabilities(&descriptor.capabilities)?;
        let provider = self.registry.get(&request.provider_id)?;
        Ok((provider, credential_ref))
    }

    fn decide_fallback(
        &self,
        original: &InvocationRequest,
        current: &NormalizedRequest,
        account: &CredentialAccount,
        attempts_used: u32,
        reason: FallbackReason,
    ) -> Result<provider_core::fallback::FallbackDecision, InvocationError> {
        let project_id =
            provider_core::credentials::ProjectScopeId::parse(current.project_id.clone())
                .map_err(|_| InvocationError::InvalidRequest)?;
        let fallback_request = provider_core::fallback::FallbackRequest::new(
            current.request_id.clone(),
            project_id,
            account.clone(),
            current.provider_id.clone(),
            current.model_id.clone(),
            current.capabilities.clone(),
            original.fallback_candidates.clone(),
            FallbackFailure::new(reason),
            attempts_used,
            0,
            0,
            original.access.cancellation.clone(),
        )
        .map_err(|_| InvocationError::InvalidRequest)?;
        self.fallback_policy
            .decide(fallback_request)
            .map_err(|_| InvocationError::Internal)
    }

    fn ensure_not_cancelled(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), InvocationError> {
        if cancellation.is_cancelled() {
            Err(InvocationError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn prompt_from(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn attempt_id(request_id: &str, attempt_number: u32) -> String {
    format!("{request_id}:attempt_{attempt_number}")
}

fn map_response(
    response: ProviderResponse,
    provider_id: provider_core::ProviderId,
    attempt_id: String,
    attempt_number: u32,
) -> InvocationResult {
    InvocationResult {
        attempt_id,
        attempt_number,
        provider_id,
        model_id: response.model_id,
        text: response.text,
        finish_reason: response.finish_reason,
        usage: response.usage,
    }
}

fn map_provider_failure(error: &ModelProviderError) -> FallbackReason {
    match error {
        ModelProviderError::UnsupportedOperation(_) => FallbackReason::Unsupported,
        ModelProviderError::InvalidRequest => FallbackReason::InvalidRequest,
        ModelProviderError::InvalidCredentialRef => FallbackReason::Authentication,
        ModelProviderError::Cancelled => FallbackReason::PolicyDenied,
        ModelProviderError::Unavailable | ModelProviderError::Internal => FallbackReason::Outage,
        ModelProviderError::Backpressure => FallbackReason::PolicyDenied,
        ModelProviderError::InvalidProviderId | ModelProviderError::InvalidModelId => {
            FallbackReason::InvalidRequest
        }
    }
}

async fn collect_stream(
    mut stream: provider_core::ProviderStream<'_>,
    attempt_id: &str,
    cancellation: &CancellationToken,
) -> Result<Vec<InvocationStreamEvent>, ModelProviderError> {
    let mut events = Vec::new();
    let mut terminal = false;
    while let Some(event) = stream.next().await {
        if cancellation.is_cancelled() {
            return Err(ModelProviderError::Cancelled);
        }
        let event = event?;
        terminal = event.terminal;
        events.push(InvocationStreamEvent {
            attempt_id: attempt_id.to_string(),
            sequence: event.sequence,
            text: event.text,
            terminal: event.terminal,
        });
        if terminal {
            break;
        }
    }
    if terminal {
        Ok(events)
    } else {
        Err(ModelProviderError::Unavailable)
    }
}
