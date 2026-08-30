use plugin_core::*;

fn valid(id: &str) -> PluginManifest {
    PluginManifest::new(
        id,
        "1.0.0",
        "api-1",
        "bin/plugin",
        vec!["read".into()],
        vec!["linux".into()],
        vec![],
        "signer-1",
        "provenance-1",
        Isolation::Process,
    )
    .unwrap()
}

// @spec:AC-1389
#[test]
fn manifest_is_canonical_bounded_and_unsigned_is_untrusted() {
    let manifest = valid("plugin-a");
    assert_eq!(manifest.trust(), TrustState::Trusted);
    assert_eq!(manifest.digest(), valid("plugin-a").digest());
    let unsigned = PluginManifest::new(
        "plugin-b",
        "1.0.0",
        "api-1",
        "bin/plugin",
        vec!["read".into()],
        vec!["linux".into()],
        vec![],
        "",
        "",
        Isolation::Process,
    )
    .unwrap();
    assert_eq!(unsigned.trust(), TrustState::Untrusted);
    assert!(matches!(
        PluginManifest::new(
            "plugin-c",
            "1.0.0",
            "api-1",
            "bin/plugin",
            vec!["admin".into()],
            vec!["linux".into()],
            vec![],
            "s",
            "p",
            Isolation::Process
        ),
        Err(ManifestError::CapabilityDenied)
    ));
}

// @spec:AC-1390
#[test]
fn dependency_graph_and_compatibility_are_fail_closed() {
    let a = PluginManifest::new(
        "a",
        "1.0.0",
        "api-1",
        "bin/a",
        vec!["read".into()],
        vec!["linux".into()],
        vec!["b".into()],
        "s",
        "p",
        Isolation::Process,
    )
    .unwrap();
    let b = PluginManifest::new(
        "b",
        "1.0.0",
        "api-1",
        "bin/b",
        vec!["read".into()],
        vec!["linux".into()],
        vec!["a".into()],
        "s",
        "p",
        Isolation::Process,
    )
    .unwrap();
    assert_eq!(
        DependencyGraph::validate(&[a, b]),
        Err(ManifestError::DependencyCycle)
    );
    assert!(matches!(
        PluginManifest::new(
            "bad",
            "not-semver",
            "api-1",
            "bin/x",
            vec!["read".into()],
            vec!["linux".into()],
            vec![],
            "s",
            "p",
            Isolation::Process
        ),
        Err(ManifestError::InvalidVersion)
    ));
}
