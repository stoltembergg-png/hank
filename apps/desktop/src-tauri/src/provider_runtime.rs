//! Desktop-owned façade for the OpenAI-compatible provider adapter.
//!
//! The adapter remains provider-neutral and synchronous. This façade is the
//! boundary that carries the validated application envelope, resolves an
//! opaque credential at send time, and maps all provider/transport failures to
//! the stable `ModelProvider` error taxonomy.

use crate::provider_transport::{
    CredentialMaterialResolver, DesktopHttpTransport, ReqwestExecutor, StoreCredentialResolver,
};
use futures_util::stream;
use provider_adapter_openai::{
    AdapterError, EndpointPolicy, OpenAiModel, OpenAiProvider, OpenAiProviderDescriptor,
    ProviderDescriptorError,
};
use provider_core::capabilities::CapabilityReport;
use provider_core::response::{
    FinishReason as NormalizedFinishReason, OutputPartKind, ResponseStatus,
};
use provider_core::stream::StreamEventPayload;
use provider_core::{
    CancellationToken, FinishReason, HealthStatus, ModelDescriptor, ModelProvider,
    ModelProviderError, ProviderFuture, ProviderId, ProviderRequest, ProviderResponse,
    ProviderStream, ProviderStreamEvent, StreamConfig, Usage,
};
use std::sync::Arc;
use std::time::Duration;

const PROVIDER_TIMEOUT: Duration = Duration::from_secs(45);

pub struct OpenAiRuntimeProvider<R, E = ReqwestExecutor>
where
    R: CredentialMaterialResolver + Clone + 'static,
    E: crate::provider_transport::RequestExecutor,
{
    descriptor: OpenAiProviderDescriptor,
    endpoint: EndpointPolicy,
    resolver: R,
    executor: E,
    timeout: Duration,
}

impl<R> OpenAiRuntimeProvider<R, ReqwestExecutor>
where
    R: CredentialMaterialResolver + Clone + 'static,
{
    pub fn new(
        endpoint: EndpointPolicy,
        resolver: R,
        timeout: Duration,
    ) -> Result<Self, ModelProviderError> {
        if timeout.is_zero() {
            return Err(ModelProviderError::InvalidRequest);
        }
        let executor = ReqwestExecutor::new().map_err(|_| ModelProviderError::Unavailable)?;
        Ok(Self::with_executor(endpoint, resolver, executor, timeout))
    }
}

impl<R, E> OpenAiRuntimeProvider<R, E>
where
    R: CredentialMaterialResolver + Clone + 'static,
    E: crate::provider_transport::RequestExecutor,
{
    pub fn with_executor(
        endpoint: EndpointPolicy,
        resolver: R,
        executor: E,
        timeout: Duration,
    ) -> Self {
        Self {
            descriptor: OpenAiProviderDescriptor::new(),
            endpoint,
            resolver,
            executor,
            timeout,
        }
    }

    pub fn default_timeout() -> Duration {
        PROVIDER_TIMEOUT
    }

    fn normalized_request(
        &self,
        request: ProviderRequest,
    ) -> Result<
        (
            provider_core::request::NormalizedRequest,
            provider_core::CredentialRef,
        ),
        ModelProviderError,
    > {
        let normalized = request
            .normalized
            .ok_or(ModelProviderError::InvalidRequest)?;
        if normalized.provider_id != *self.descriptor.provider_id()
            || normalized.model_id != request.model_id
        {
            return Err(ModelProviderError::InvalidRequest);
        }
        normalized
            .validate()
            .map_err(|_| ModelProviderError::InvalidRequest)?;
        Ok((*normalized, request.credential_ref))
    }

    fn transport(&self) -> DesktopHttpTransport<R, E> {
        DesktopHttpTransport::with_executor(self.resolver.clone(), self.executor.clone())
    }
}

impl<R, E> ModelProvider for OpenAiRuntimeProvider<R, E>
where
    R: CredentialMaterialResolver + Clone + 'static,
    E: crate::provider_transport::RequestExecutor,
{
    fn provider_id(&self) -> &ProviderId {
        self.descriptor.provider_id()
    }

    fn version(&self) -> &str {
        self.descriptor.version()
    }

    fn capabilities(&self) -> CapabilityReport {
        self.descriptor
            .capabilities(OpenAiModel::Gpt4o)
            .expect("static OpenAI model descriptor")
            .clone()
    }

    fn complete(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
    ) -> ProviderFuture<'_, Result<ProviderResponse, ModelProviderError>> {
        let prepared = self.normalized_request(request);
        let endpoint = self.endpoint.clone();
        let transport = self.transport();
        let timeout = self.timeout;
        Box::pin(async move {
            let (normalized, credential_ref) = prepared?;
            let result = tokio::task::spawn_blocking(move || {
                let provider = OpenAiProvider::new(endpoint, credential_ref, transport, timeout)
                    .map_err(map_descriptor_error)?;
                provider
                    .complete(normalized, &cancellation)
                    .map_err(map_descriptor_error)
            })
            .await
            .map_err(|_| ModelProviderError::Internal)??;
            map_response(result)
        })
    }

    fn stream(
        &self,
        request: ProviderRequest,
        cancellation: CancellationToken,
        config: StreamConfig,
    ) -> Result<ProviderStream<'_>, ModelProviderError> {
        let (normalized, credential_ref) = self.normalized_request(request)?;
        let provider = OpenAiProvider::new(
            self.endpoint.clone(),
            credential_ref,
            self.transport(),
            self.timeout,
        )
        .map_err(map_descriptor_error)?;
        let events = provider
            .stream(normalized, &cancellation)
            .map_err(map_descriptor_error)?;
        if events.len() > config.max_buffered_events {
            return Err(ModelProviderError::Backpressure);
        }
        let mapped = events.into_iter().map(map_stream_event);
        Ok(Box::pin(stream::iter(mapped)))
    }

    fn list_models(&self) -> ProviderFuture<'_, Result<Vec<ModelDescriptor>, ModelProviderError>> {
        let models = self
            .descriptor
            .models()
            .iter()
            .map(|model| ModelDescriptor {
                model_id: model.model_id.clone(),
                display_name: model.model_id.as_str().to_string(),
            })
            .collect();
        Box::pin(async move { Ok(models) })
    }

    fn health(&self) -> ProviderFuture<'_, Result<HealthStatus, ModelProviderError>> {
        // Health has no project/account context, so it must not claim a live
        // provider connection. An invocation performs the credential and
        // network checks under the scoped request.
        Box::pin(async { Ok(HealthStatus::Unavailable) })
    }
}

fn map_response(
    response: provider_core::response::NormalizedResponse,
) -> Result<ProviderResponse, ModelProviderError> {
    if response.status != ResponseStatus::Complete {
        return Err(ModelProviderError::Unavailable);
    }
    let finish_reason = map_finish_reason(response.finish_reason)?;
    let text = response
        .parts
        .iter()
        .filter(|part| part.kind == OutputPartKind::Text)
        .map(|part| part.content.as_str())
        .collect::<String>();
    let usage = response.usage.unwrap_or(provider_core::response::Usage {
        input_tokens: 0,
        output_tokens: 0,
    });
    Ok(ProviderResponse {
        model_id: response.model_id,
        text,
        finish_reason,
        usage: Usage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
        },
    })
}

fn map_finish_reason(reason: NormalizedFinishReason) -> Result<FinishReason, ModelProviderError> {
    match reason {
        NormalizedFinishReason::Stop => Ok(FinishReason::Stop),
        NormalizedFinishReason::Length => Ok(FinishReason::Length),
        NormalizedFinishReason::Cancelled => Ok(FinishReason::Cancelled),
        NormalizedFinishReason::ContentFilter
        | NormalizedFinishReason::ToolCall
        | NormalizedFinishReason::Error
        | NormalizedFinishReason::Unknown => Err(ModelProviderError::Unavailable),
    }
}

fn map_stream_event(
    event: provider_core::stream::StreamEvent,
) -> Result<ProviderStreamEvent, ModelProviderError> {
    let (text, terminal) = match event.payload {
        StreamEventPayload::Delta { part } if part.kind == OutputPartKind::Text => {
            (part.content, false)
        }
        StreamEventPayload::Finish { .. }
        | StreamEventPayload::Error { .. }
        | StreamEventPayload::Cancel { .. } => (String::new(), true),
        StreamEventPayload::Start { .. }
        | StreamEventPayload::ToolRequest { .. }
        | StreamEventPayload::Usage { .. }
        | StreamEventPayload::Delta { .. }
        | StreamEventPayload::Unknown => (String::new(), false),
    };
    Ok(ProviderStreamEvent {
        sequence: event.sequence,
        text,
        terminal,
    })
}

fn map_descriptor_error(error: ProviderDescriptorError) -> ModelProviderError {
    match error {
        ProviderDescriptorError::ProviderMismatch
        | ProviderDescriptorError::UnsupportedModel(_)
        | ProviderDescriptorError::UnsupportedCapability(_)
        | ProviderDescriptorError::InvalidRequest
        | ProviderDescriptorError::Adapter(AdapterError::Credential) => {
            ModelProviderError::InvalidRequest
        }
        ProviderDescriptorError::Adapter(AdapterError::Transport(transport)) => match transport {
            provider_core::transport::TransportError::Cancelled => ModelProviderError::Cancelled,
            provider_core::transport::TransportError::RequestTooLarge => {
                ModelProviderError::InvalidRequest
            }
            provider_core::transport::TransportError::Timeout
            | provider_core::transport::TransportError::Unavailable
            | provider_core::transport::TransportError::ResponseTooLarge => {
                ModelProviderError::Unavailable
            }
        },
        ProviderDescriptorError::Adapter(AdapterError::Response(_)) => {
            ModelProviderError::Unavailable
        }
        ProviderDescriptorError::Adapter(AdapterError::MalformedResponse)
        | ProviderDescriptorError::Adapter(AdapterError::Stream(_))
        | ProviderDescriptorError::Adapter(AdapterError::IncompleteStream)
        | ProviderDescriptorError::Adapter(AdapterError::Endpoint(_))
        | ProviderDescriptorError::Adapter(AdapterError::InvalidRequest) => {
            ModelProviderError::Internal
        }
    }
}

/// Convenience constructor used by the desktop shell once an explicit HTTPS
/// endpoint is configured.
pub fn configured_openai_provider(
    endpoint: EndpointPolicy,
    store: Arc<
        crate::provider_credential_store::ProviderCredentialStore<
            crate::platform_store::PlatformSecretBackend,
        >,
    >,
) -> Result<OpenAiRuntimeProvider<StoreCredentialResolver, ReqwestExecutor>, ModelProviderError> {
    OpenAiRuntimeProvider::new(
        endpoint,
        StoreCredentialResolver::new(store),
        PROVIDER_TIMEOUT,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_transport::{CredentialMaterialResolver, RequestExecutor};
    use provider_core::request::{
        CancellationMetadata, MessageRole, NormalizedMessage, NormalizedRequest, RequestBudget,
    };
    use provider_core::transport::{HttpRequest, HttpResponse, TransportError};
    use secrets_core::SecretMaterial;

    #[derive(Clone)]
    struct Resolver;
    impl CredentialMaterialResolver for Resolver {
        fn resolve(
            &self,
            _: &provider_core::CredentialRef,
        ) -> Result<SecretMaterial, TransportError> {
            SecretMaterial::new(b"runtime-test-material".to_vec())
                .map_err(|_| TransportError::Unavailable)
        }
    }

    #[derive(Clone)]
    struct Executor;
    impl RequestExecutor for Executor {
        fn execute(
            &self,
            _: HttpRequest,
            _: Duration,
            _: usize,
        ) -> Result<HttpResponse, TransportError> {
            Ok(HttpResponse::ok(br#"{"id":"cmpl-1","model":"gpt-4o-mini","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}]}"#.to_vec()))
        }
    }

    fn normalized() -> provider_core::request::NormalizedRequest {
        NormalizedRequest {
            schema_version: 1,
            request_id: "req-runtime".into(),
            correlation_id: "corr-runtime".into(),
            project_id: "project-runtime".into(),
            agent_id: "agent-runtime".into(),
            session_id: Some("session-runtime".into()),
            provider_id: ProviderId::parse("openai").unwrap(),
            model_id: provider_core::ModelId::parse("gpt-4o-mini").unwrap(),
            messages: vec![NormalizedMessage {
                role: MessageRole::User,
                content: "hello".into(),
            }],
            modalities: std::collections::BTreeSet::from([
                provider_core::capabilities::ModelModality::Text,
            ]),
            capabilities: provider_core::capabilities::CapabilityRequirement {
                modalities: std::collections::BTreeSet::from([
                    provider_core::capabilities::ModelModality::Text,
                ]),
                features: std::collections::BTreeSet::new(),
                min_context_tokens: None,
                min_output_tokens: None,
            },
            tools: Vec::new(),
            budget: RequestBudget {
                max_tokens: Some(128),
                max_cost_micros: None,
            },
            cancellation: CancellationMetadata {
                cancellation_id: "cancel-runtime".into(),
                deadline_unix_ms: None,
            },
            temperature: Some(0.2),
        }
    }

    #[tokio::test]
    async fn facade_requires_scoped_normalized_envelope_and_maps_complete() {
        let provider = OpenAiRuntimeProvider::with_executor(
            EndpointPolicy::parse("https://provider.example/v1").unwrap(),
            Resolver,
            Executor,
            Duration::from_secs(5),
        );
        let request = ProviderRequest::new(
            "attempt-1",
            provider_core::ModelId::parse("gpt-4o-mini").unwrap(),
            provider_core::CredentialRef::parse("cred_runtime_ref").unwrap(),
            "hello",
        )
        .unwrap();
        assert_eq!(
            provider
                .complete(request.clone(), CancellationToken::new())
                .await
                .unwrap_err(),
            ModelProviderError::InvalidRequest
        );
        let response = provider
            .complete(
                request.with_normalized(normalized()),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(response.text, "ok");
        assert_eq!(response.model_id.as_str(), "gpt-4o-mini");
    }

    #[tokio::test]
    async fn facade_health_is_not_false_positive_and_models_are_deterministic() {
        let provider = OpenAiRuntimeProvider::with_executor(
            EndpointPolicy::parse("https://provider.example/v1").unwrap(),
            Resolver,
            Executor,
            Duration::from_secs(5),
        );
        assert_eq!(provider.health().await.unwrap(), HealthStatus::Unavailable);
        assert_eq!(provider.list_models().await.unwrap().len(), 2);
    }
}
