//! Bounded HTTPS transport owned by the desktop shell.

use provider_core::transport::{validate_request_body, HttpRequest, HttpResponse, HttpTransport, TransportError};
use provider_core::{CancellationToken, CredentialRef};
use secrets_core::SecretMaterial;
use std::sync::Arc;
use std::time::Duration;

use crate::platform_store::PlatformSecretBackend;
use crate::provider_credential_store::ProviderCredentialStore;

pub trait CredentialMaterialResolver: Send + Sync {
    fn resolve(&self, reference: &CredentialRef) -> Result<SecretMaterial, TransportError>;
}

#[derive(Clone)]
pub struct StoreCredentialResolver {
    store: Arc<ProviderCredentialStore<PlatformSecretBackend>>,
}

impl StoreCredentialResolver {
    pub fn new(store: Arc<ProviderCredentialStore<PlatformSecretBackend>>) -> Self {
        Self { store }
    }
}

impl CredentialMaterialResolver for StoreCredentialResolver {
    fn resolve(&self, reference: &CredentialRef) -> Result<SecretMaterial, TransportError> {
        self.store
            .material_for_reference(reference)
            .map_err(|error| match error {
                provider_core::credentials::CredentialServiceError::Cancelled => TransportError::Cancelled,
                provider_core::credentials::CredentialServiceError::Unavailable
                | provider_core::credentials::CredentialServiceError::Missing
                | provider_core::credentials::CredentialServiceError::Revoked
                | provider_core::credentials::CredentialServiceError::Unauthorized
                | provider_core::credentials::CredentialServiceError::InvalidIdentity
                | provider_core::credentials::CredentialServiceError::InvalidReference
                | provider_core::credentials::CredentialServiceError::Conflict
                | provider_core::credentials::CredentialServiceError::Internal => TransportError::Unavailable,
            })
    }
}

pub trait RequestExecutor: Send + Sync + Clone + 'static {
    fn execute(
        &self,
        request: HttpRequest,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<HttpResponse, TransportError>;
}

pub struct DesktopHttpTransport<R, E = ReqwestExecutor>
where
    R: CredentialMaterialResolver,
    E: RequestExecutor,
{
    resolver: Arc<R>,
    executor: E,
    max_response_bytes: usize,
}

#[derive(Clone)]
pub struct ReqwestExecutor {
    client: reqwest::blocking::Client,
}

impl ReqwestExecutor {
    pub fn new() -> Result<Self, TransportError> {
        reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map(|client| Self { client })
            .map_err(|_| TransportError::Unavailable)
    }
}

impl RequestExecutor for ReqwestExecutor {
    fn execute(
        &self,
        request: HttpRequest,
        timeout: Duration,
        max_response_bytes: usize,
    ) -> Result<HttpResponse, TransportError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|_| TransportError::Unavailable)?;
        let url = reqwest::Url::parse(&request.url).map_err(|_| TransportError::Unavailable)?;
        let mut builder = self.client.request(method, url).timeout(timeout);
        for (name, value) in request.headers {
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| TransportError::Unavailable)?;
            let value = reqwest::header::HeaderValue::from_str(&value)
                .map_err(|_| TransportError::Unavailable)?;
            builder = builder.header(name, value);
        }
        let response = builder
            .body(request.body)
            .send()
            .map_err(|error| {
                if error.is_timeout() {
                    TransportError::Timeout
                } else {
                    TransportError::Unavailable
                }
            })?;
        if response
            .content_length()
            .is_some_and(|length| length > max_response_bytes as u64)
        {
            return Err(TransportError::ResponseTooLarge);
        }
        let status = response.status().as_u16();
        let mut body = Vec::new();
        use std::io::Read;
        response
            .take(max_response_bytes.saturating_add(1) as u64)
            .read_to_end(&mut body)
            .map_err(|_| TransportError::Unavailable)?;
        if body.len() > max_response_bytes {
            return Err(TransportError::ResponseTooLarge);
        }
        Ok(HttpResponse::with_status(status, body))
    }
}

impl<R: CredentialMaterialResolver> DesktopHttpTransport<R, ReqwestExecutor> {
    pub fn new(resolver: R) -> Result<Self, TransportError> {
        Ok(Self::with_executor(resolver, ReqwestExecutor::new()?))
    }
}

impl<R, E> DesktopHttpTransport<R, E>
where
    R: CredentialMaterialResolver,
    E: RequestExecutor,
{
    const DEFAULT_MAX_RESPONSE_BYTES: usize = 2_097_152;

    pub fn with_executor(resolver: R, executor: E) -> Self {
        Self {
            resolver: Arc::new(resolver),
            executor,
            max_response_bytes: Self::DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn with_max_response_bytes(mut self, max_response_bytes: usize) -> Self {
        self.max_response_bytes = max_response_bytes;
        self
    }
}

impl<R, E> HttpTransport for DesktopHttpTransport<R, E>
where
    R: CredentialMaterialResolver,
    E: RequestExecutor,
{
    fn send(
        &self,
        mut request: HttpRequest,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<HttpResponse, TransportError> {
        if cancellation.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if timeout.is_zero() {
            return Err(TransportError::Timeout);
        }
        let url = reqwest::Url::parse(&request.url).map_err(|_| TransportError::Unavailable)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || url.username() != ""
            || url.password().is_some()
            || url.port().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(TransportError::Unavailable);
        }
        if request.headers.keys().any(|name| is_sensitive_header(name)) {
            return Err(TransportError::Unavailable);
        }
        validate_request_body(&request.body)?;
        let material = self.resolver.resolve(&request.credential_ref)?;
        let secret = material.into_bytes();
        let secret = String::from_utf8(secret).map_err(|_| TransportError::Unavailable)?;
        request
            .headers
            .insert("Authorization".into(), format!("Bearer {secret}"));
        let response = self
            .executor
            .execute(request, timeout, self.max_response_bytes)?;
        if cancellation.is_cancelled() {
            return Err(TransportError::Cancelled);
        }
        if response.body.len() > self.max_response_bytes {
            return Err(TransportError::ResponseTooLarge);
        }
        Ok(response)
    }
}

fn is_sensitive_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "authorization"
        || name == "cookie"
        || name == "set-cookie"
        || name.contains("api-key")
        || name.contains("apikey")
        || name.contains("token")
        || name.contains("secret")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct StaticResolver;

    impl CredentialMaterialResolver for StaticResolver {
        fn resolve(&self, _: &CredentialRef) -> Result<SecretMaterial, TransportError> {
            SecretMaterial::new(b"synthetic-provider-material".to_vec()).map_err(|_| TransportError::Unavailable)
        }
    }

    #[derive(Clone)]
    struct FakeExecutor {
        response: HttpResponse,
        seen: Arc<std::sync::Mutex<Vec<HttpRequest>>>,
    }

    impl RequestExecutor for FakeExecutor {
        fn execute(&self, request: HttpRequest, _: Duration, _: usize) -> Result<HttpResponse, TransportError> {
            self.seen.lock().unwrap().push(request);
            Ok(self.response.clone())
        }
    }

    fn request(url: &str) -> HttpRequest {
        HttpRequest {
            method: "POST".into(),
            url: url.into(),
            headers: std::collections::BTreeMap::from([(String::from("content-type"), String::from("application/json"))]),
            body: br#"{"prompt":"hello"}"#.to_vec(),
            credential_ref: CredentialRef::parse("cred_transport_test").unwrap(),
        }
    }

    fn transport(response: HttpResponse) -> (DesktopHttpTransport<StaticResolver, FakeExecutor>, Arc<std::sync::Mutex<Vec<HttpRequest>>>) {
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let executor = FakeExecutor { response, seen: seen.clone() };
        (DesktopHttpTransport::with_executor(StaticResolver, executor), seen)
    }

    #[test]
    fn injects_secret_only_as_bounded_authorization_header() {
        let (transport, seen) = transport(HttpResponse::ok(b"{}".to_vec()));
        let response = transport.send(request("https://provider.example/v1/chat"), Duration::from_secs(5), &CancellationToken::new()).unwrap();
        assert_eq!(response.status, 200);
        let request = seen.lock().unwrap().first().cloned().unwrap();
        assert_eq!(request.headers.get("Authorization").map(String::as_str), Some("Bearer synthetic-provider-material"));
        assert!(!String::from_utf8_lossy(&request.body).contains("synthetic-provider-material"));
    }

    #[test]
    fn rejects_insecure_endpoint_before_resolving_or_network() {
        let (transport, seen) = transport(HttpResponse::ok(b"{}".to_vec()));
        let result = transport.send(request("http://provider.example/v1/chat"), Duration::from_secs(5), &CancellationToken::new());
        assert_eq!(result, Err(TransportError::Unavailable));
        assert!(seen.lock().unwrap().is_empty());
    }

    #[test]
    fn cancellation_and_response_limits_fail_closed() {
        let (first, _) = transport(HttpResponse::ok(vec![b'x'; 16]));
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert_eq!(first.send(request("https://provider.example/v1/chat"), Duration::from_secs(5), &cancelled), Err(TransportError::Cancelled));
        let (limited, _) = transport(HttpResponse::ok(vec![b'x'; 16]));
        let limited = limited.with_max_response_bytes(8);
        assert_eq!(limited.send(request("https://provider.example/v1/chat"), Duration::from_secs(5), &CancellationToken::new()), Err(TransportError::ResponseTooLarge));
    }
}
