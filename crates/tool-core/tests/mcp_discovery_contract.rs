use tool_core::mcp_discovery::*;

fn manifest() -> ToolManifest {
    ToolManifest::new(
        "rev-1",
        vec!["read".into()],
        vec![
            ToolDescription::new("alpha", 64),
            ToolDescription::new("beta", 32),
        ],
    )
    .unwrap()
}
fn request() -> DiscoveryRequest {
    DiscoveryRequest::new("server-a", true, true, 100, vec!["read".into()]).unwrap()
}

// @spec:AC-1385
#[test]
fn authorized_manifest_stages_disabled_entries_without_execution() {
    let result = Discovery::process(request(), manifest()).unwrap();
    assert_eq!(result.entries().len(), 2);
    assert!(
        result
            .entries()
            .iter()
            .all(|e| e.state() == EntryState::Pending)
    );
    assert!(result.entries().iter().all(|e| !e.execution_enabled()));
    assert!(matches!(
        Discovery::process(
            DiscoveryRequest::new("server-a", false, true, 100, vec!["read".into()]).unwrap(),
            manifest()
        ),
        Err(DiscoveryError::TransportUnauthorized)
    ));
}

// @spec:AC-1386
#[test]
fn duplicates_unknown_capabilities_and_refresh_never_widen_trust() {
    let duplicate = ToolManifest::new(
        "rev-1",
        vec!["read".into()],
        vec![
            ToolDescription::new("alpha", 64),
            ToolDescription::new("alpha", 64),
        ],
    );
    assert!(matches!(duplicate, Err(DiscoveryError::DuplicateTool)));
    let unknown = ToolManifest::new(
        "rev-1",
        vec!["execute".into()],
        vec![ToolDescription::new("alpha", 64)],
    );
    assert!(matches!(
        Discovery::process(request(), unknown.unwrap()),
        Err(DiscoveryError::CapabilityDenied)
    ));
    let result = Discovery::process(request(), manifest()).unwrap();
    assert!(
        !result
            .refresh("rev-2", vec![ToolDescription::new("gamma", 64)])
            .unwrap()
            .entries()[0]
            .execution_enabled()
    );
}
