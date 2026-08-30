use plugin_core::{DiscoveryError, Isolation, PluginDiscovery, PluginManifest, PluginStage};

fn manifest(id: &str, api: &str) -> PluginManifest {
    PluginManifest::new(
        id,
        "1.0.0",
        api,
        "bin/plugin",
        vec!["read".into()],
        vec!["linux".into()],
        vec![],
        "signer",
        "provenance",
        Isolation::Process,
    )
    .unwrap()
}

#[test]
// @spec:AC-1391
fn allowlisted_sources_stage_without_activation() {
    let discovery = PluginDiscovery::new("/srv/plugins", 8).unwrap();
    let catalog = discovery
        .discover(vec![
            ("/srv/plugins/z", manifest("z", "api-1")),
            ("/srv/plugins/a", manifest("a", "api-1")),
        ])
        .unwrap();
    assert_eq!(catalog.ids(), vec!["a", "z"]);
    assert!(catalog
        .entries()
        .iter()
        .all(|entry| entry.stage() == PluginStage::Staged && !entry.execution_enabled()));
}

#[test]
// @spec:AC-1392
fn unsafe_sources_duplicates_and_api_mismatch_fail_closed() {
    let discovery = PluginDiscovery::new("/srv/plugins", 8).unwrap();
    assert!(matches!(
        discovery.discover(vec![("/tmp/plugin", manifest("a", "api-1"))]),
        Err(DiscoveryError::SourceNotAllowed)
    ));
    assert!(matches!(
        discovery.discover(vec![
            ("/srv/plugins/a", manifest("a", "api-1")),
            ("/srv/plugins/b", manifest("a", "api-1"))
        ]),
        Err(DiscoveryError::DuplicatePlugin)
    ));
    assert!(matches!(
        discovery.discover(vec![("/srv/plugins/a", manifest("a", "api-2"))]),
        Err(DiscoveryError::ApiUnsupported)
    ));
}
