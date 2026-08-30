//! Bounded MCP-over-HTTP policy; network execution remains in the HTTP adapter.
const MAX: usize = 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMcpState {
    Active,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMcpError {
    InvalidUrl,
    InsecureScheme,
    UrlCredentials,
    InvalidLimits,
    BodyTooLarge,
    RetryDenied,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpMcpRequest {
    endpoint: String,
    method: HttpMethod,
    max_body: usize,
    allow_http: bool,
    retry_limit: u8,
    retries: u8,
    state: HttpMcpState,
}
impl HttpMcpRequest {
    pub fn new(
        endpoint: &str,
        method: HttpMethod,
        max_body: usize,
        allow_http: bool,
        retry_limit: u8,
    ) -> Result<Self, HttpMcpError> {
        let url = reqwest::Url::parse(endpoint).map_err(|_| HttpMcpError::InvalidUrl)?;
        if url.username() != "" || url.password().is_some() {
            return Err(HttpMcpError::UrlCredentials);
        }
        if url.scheme() == "http" && !allow_http {
            return Err(HttpMcpError::InsecureScheme);
        }
        if !matches!(url.scheme(), "http" | "https") || max_body == 0 || max_body > MAX {
            return Err(HttpMcpError::InvalidLimits);
        }
        Ok(Self {
            endpoint: endpoint.into(),
            method,
            max_body,
            allow_http,
            retry_limit,
            retries: 0,
            state: HttpMcpState::Active,
        })
    }
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub fn accept_body(&self, size: usize) -> Result<(), HttpMcpError> {
        if size > self.max_body {
            Err(HttpMcpError::BodyTooLarge)
        } else {
            Ok(())
        }
    }
    pub fn can_retry(&mut self, idempotency_key: bool) -> bool {
        if self.state != HttpMcpState::Active || self.retries >= self.retry_limit {
            return false;
        }
        if matches!(self.method, HttpMethod::Post) && !idempotency_key {
            return false;
        }
        self.retries += 1;
        true
    }
    pub fn cancel(&mut self) -> HttpMcpState {
        self.state = HttpMcpState::Cancelled;
        self.state
    }
    pub fn accepts_work(&self) -> bool {
        self.state == HttpMcpState::Active
    }
}
