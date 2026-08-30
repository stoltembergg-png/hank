use tool_core::mcp_http::*;

fn valid() -> HttpMcpRequest {
    HttpMcpRequest::new("https://mcp.example.test", HttpMethod::Get, 1024, true, 2).unwrap()
}

// @spec:AC-1381
#[test]
fn endpoint_policy_is_tls_bound_and_limited() {
    let request = valid();
    assert_eq!(request.endpoint(), "https://mcp.example.test");
    assert!(matches!(
        HttpMcpRequest::new("http://mcp.example.test", HttpMethod::Get, 1024, false, 2),
        Err(HttpMcpError::InsecureScheme)
    ));
    assert!(matches!(
        HttpMcpRequest::new(
            "https://user:pass@mcp.example.test",
            HttpMethod::Get,
            1024,
            true,
            2
        ),
        Err(HttpMcpError::UrlCredentials)
    ));
    assert!(matches!(
        HttpMcpRequest::new("https://mcp.example.test", HttpMethod::Get, 0, true, 2),
        Err(HttpMcpError::InvalidLimits)
    ));
}

// @spec:AC-1382
#[test]
fn retry_is_idempotent_only_and_cancel_is_terminal() {
    let mut get = valid();
    assert!(get.can_retry(false));
    let mut post =
        HttpMcpRequest::new("https://mcp.example.test", HttpMethod::Post, 1024, true, 1).unwrap();
    assert!(!post.can_retry(false));
    assert!(post.can_retry(true));
    assert!(!post.can_retry(true));
    let mut request =
        HttpMcpRequest::new("https://mcp.example.test", HttpMethod::Get, 1024, true, 2).unwrap();
    assert_eq!(request.cancel(), HttpMcpState::Cancelled);
    assert_eq!(request.cancel(), HttpMcpState::Cancelled);
    assert!(!request.accepts_work());
}
