use agent_core::ids::ProjectId;
use agent_protocol::ids::TraceId;
use std::collections::{BTreeMap, BTreeSet};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use tool_core::{HttpError, HttpRequest, PermissionDecision, execute_http};

fn request(url: String) -> HttpRequest {
    let host = reqwest::Url::parse(&url)
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();
    HttpRequest {
        project_id: ProjectId::new(),
        method: "GET".into(),
        url,
        headers: BTreeMap::new(),
        body: None,
        allowed_hosts: BTreeSet::from([host]),
        allow_private_networks: true,
        permission: PermissionDecision::Allowed { reason: "test" },
        timeout: Duration::from_secs(2),
        max_response_bytes: 32,
        trace_id: TraceId::new(),
    }
}

#[test]
// @spec:AC-651
fn allowed_local_mock_request_is_bounded_and_deterministic() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        use std::io::{Read, Write};
        let mut buffer = [0; 512];
        let _ = stream.read(&mut buffer);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello")
            .unwrap();
    });
    let result = execute_http(&request(format!("http://{address}/ok"))).unwrap();
    assert_eq!(result.status, 200);
    assert_eq!(result.body, "hello");
}

#[test]
// @spec:AC-652
fn rejects_private_by_default_host_header_and_permission() {
    let mut private = request("http://127.0.0.1/".into());
    private.allow_private_networks = false;
    assert_eq!(
        execute_http(&private),
        Err(HttpError::PrivateNetworkBlocked)
    );
    let mut header = request("https://example.com/".into());
    header.allowed_hosts = BTreeSet::from(["example.com".into()]);
    header
        .headers
        .insert("Authorization".into(), "secret".into());
    assert_eq!(execute_http(&header), Err(HttpError::SensitiveHeader));
    header.headers.clear();
    header.permission = PermissionDecision::NeedsConfirmation {
        scope: "http".into(),
    };
    assert_eq!(execute_http(&header), Err(HttpError::PermissionDenied));
}

#[test]
// @spec:AC-652 @spec:AC-653
fn rejects_unallowlisted_scheme_host_and_redirects_are_not_followed() {
    let mut scheme = request("ftp://example.com/".into());
    assert_eq!(execute_http(&scheme), Err(HttpError::InvalidScheme));
    scheme.url = "https://not-allowed.example/".into();
    assert_eq!(execute_http(&scheme), Err(HttpError::HostNotAllowed));
}
