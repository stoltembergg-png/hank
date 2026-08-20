//! Provider-neutral transport boundary for concrete adapters.

use crate::{CancellationToken, CredentialRef};
use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;
use thiserror::Error;

const MAX_ENDPOINT_LEN: usize = 512;
const MAX_HTTP_BODY_BYTES: usize = 2_097_152;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EndpointError {
    #[error("endpoint must use https")]
    Insecure,
    #[error("endpoint is invalid or not allowlisted")]
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointPolicy {
    base_url: String,
}

impl EndpointPolicy {
    pub fn parse(value: impl Into<String>) -> Result<Self, EndpointError> {
        let value = value.into();
        let remainder = value
            .strip_prefix("https://")
            .ok_or(EndpointError::Insecure)?;
        let host = remainder.split('/').next().unwrap_or_default();
        if value.trim() != value
            || value.len() > MAX_ENDPOINT_LEN
            || host.is_empty()
            || host.contains('@')
            || host.contains(':')
            || value.chars().any(char::is_control)
            || value.contains('?')
            || value.contains('#')
        {
            return Err(EndpointError::Invalid);
        }
        Ok(Self {
            base_url: value.trim_end_matches('/').to_owned(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn path(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }
}

#[derive(Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub credential_ref: CredentialRef,
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("body_bytes", &self.body.len())
            .field("credential_ref", &self.credential_ref)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn ok(body: Vec<u8>) -> Self {
        Self { status: 200, body }
    }

    pub fn with_status(status: u16, body: Vec<u8>) -> Self {
        Self { status, body }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransportError {
    #[error("transport timed out")]
    Timeout,
    #[error("transport cancelled")]
    Cancelled,
    #[error("transport unavailable")]
    Unavailable,
    #[error("transport request exceeds bounded size")]
    RequestTooLarge,
    #[error("transport response exceeds bounded size")]
    ResponseTooLarge,
}

pub trait HttpTransport: Send + Sync {
    fn send(
        &self,
        request: HttpRequest,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<HttpResponse, TransportError>;
}

pub fn validate_request_body(body: &[u8]) -> Result<(), TransportError> {
    if body.len() > MAX_HTTP_BODY_BYTES {
        return Err(TransportError::RequestTooLarge);
    }
    Ok(())
}
