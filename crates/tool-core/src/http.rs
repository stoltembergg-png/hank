//! Bounded HTTP client contract with explicit egress policy.

use crate::PermissionDecision;
use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use reqwest::Method;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub project_id: ProjectId,
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub allowed_hosts: BTreeSet<String>,
    pub allow_private_networks: bool,
    pub permission: PermissionDecision,
    pub timeout: Duration,
    pub max_response_bytes: usize,
    pub trace_id: TraceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub trace_id: TraceId,
    pub status: u16,
    pub body: String,
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HttpError {
    #[error("permission denied")]
    PermissionDenied,
    #[error("invalid URL")]
    InvalidUrl,
    #[error("HTTP scheme is not allowed")]
    InvalidScheme,
    #[error("host is not allowlisted")]
    HostNotAllowed,
    #[error("private network target is blocked")]
    PrivateNetworkBlocked,
    #[error("sensitive header is not allowed")]
    SensitiveHeader,
    #[error("HTTP method is invalid")]
    InvalidMethod,
    #[error("HTTP limits are invalid")]
    InvalidLimits,
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("response is not valid UTF-8")]
    InvalidResponse,
}

pub fn execute_http(request: &HttpRequest) -> Result<HttpResponse, HttpError> {
    if !request.permission.is_allowed() {
        return Err(HttpError::PermissionDenied);
    }
    if request.timeout.is_zero() || request.max_response_bytes == 0 {
        return Err(HttpError::InvalidLimits);
    }
    let url = reqwest::Url::parse(&request.url).map_err(|_| HttpError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(HttpError::InvalidScheme);
    }
    let host = url
        .host_str()
        .ok_or(HttpError::InvalidUrl)?
        .to_ascii_lowercase();
    if !request.allowed_hosts.contains(&host) {
        return Err(HttpError::HostNotAllowed);
    }
    if !request.allow_private_networks && is_private_host(&host) {
        return Err(HttpError::PrivateNetworkBlocked);
    }
    for key in request.headers.keys() {
        let lower = key.to_ascii_lowercase();
        if lower == "authorization"
            || lower == "cookie"
            || lower.contains("token")
            || lower.contains("secret")
        {
            return Err(HttpError::SensitiveHeader);
        }
    }
    let method =
        Method::from_bytes(request.method.as_bytes()).map_err(|_| HttpError::InvalidMethod)?;
    let client = Client::builder()
        .timeout(request.timeout)
        .redirect(Policy::none())
        .build()
        .map_err(|_| HttpError::RequestFailed)?;
    let mut headers = HeaderMap::new();
    for (key, value) in &request.headers {
        let name = HeaderName::try_from(key).map_err(|_| HttpError::SensitiveHeader)?;
        let value = HeaderValue::try_from(value).map_err(|_| HttpError::SensitiveHeader)?;
        headers.insert(name, value);
    }
    let mut builder = client.request(method, url).headers(headers);
    if let Some(body) = &request.body {
        builder = builder.body(body.clone());
    }
    let response = builder.send().map_err(|_| HttpError::RequestFailed)?;
    let status = response.status().as_u16();
    let bytes = response.bytes().map_err(|_| HttpError::RequestFailed)?;
    let truncated = bytes.len() > request.max_response_bytes;
    let body = String::from_utf8(bytes[..bytes.len().min(request.max_response_bytes)].to_vec())
        .map_err(|_| HttpError::InvalidResponse)?;
    Ok(HttpResponse {
        trace_id: request.trace_id,
        status,
        body,
        truncated,
    })
}

fn is_private_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") || host == "::1" {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|ip| match ip {
            IpAddr::V4(ip) => {
                ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
            }
            IpAddr::V6(ip) => ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local(),
        })
        .unwrap_or(false)
}
