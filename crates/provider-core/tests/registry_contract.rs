use provider_core::{
    capabilities::{CapabilityFeature, ModelModality},
    registry::{ProviderRegistry, RegistryError},
    MockProvider, ProviderId,
};
use std::sync::Arc;

fn mock_provider() -> Arc<MockProvider> {
    Arc::new(MockProvider::new(
        ProviderId::parse("mock-provider").unwrap(),
        "0.1",
    ))
}

#[test]
fn registry_register_and_get() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider.clone()).unwrap();

    let fetched = registry
        .get(&ProviderId::parse("mock-provider").unwrap())
        .unwrap();
    assert_eq!(fetched.provider_id().as_str(), "mock-provider");
    assert_eq!(fetched.version(), "0.1");
}

#[test]
fn registry_duplicate_id_fails() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider.clone()).unwrap();
    let err = registry.register(provider).unwrap_err();
    assert!(matches!(err, RegistryError::DuplicateId(_)));
}

#[test]
fn registry_get_not_found() {
    let registry = ProviderRegistry::new();
    let err = match registry.get(&ProviderId::parse("unknown").unwrap()) {
        Err(err) => err,
        Ok(_) => panic!("unknown provider unexpectedly resolved"),
    };
    assert!(matches!(err, RegistryError::NotFound(_)));
}

#[test]
fn registry_disable_enable() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider).unwrap();
    let pid = ProviderId::parse("mock-provider").unwrap();

    assert!(registry.is_enabled(&pid).unwrap());
    registry.set_enabled(&pid, false).unwrap();
    assert!(!registry.is_enabled(&pid).unwrap());
    let err = match registry.get(&pid) {
        Err(err) => err,
        Ok(_) => panic!("disabled provider unexpectedly resolved"),
    };
    assert!(matches!(err, RegistryError::Disabled(_)));
    registry.set_enabled(&pid, true).unwrap();
    assert!(registry.is_enabled(&pid).unwrap());
    assert!(registry.get(&pid).is_ok());
}

#[test]
fn registry_get_descriptor() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider).unwrap();
    let pid = ProviderId::parse("mock-provider").unwrap();

    let desc = registry.get_descriptor(&pid).unwrap();
    assert_eq!(desc.provider_id.as_str(), "mock-provider");
    assert_eq!(desc.version, "0.1");
    assert!(desc
        .capabilities
        .supports_feature(CapabilityFeature::Streaming));
}

#[test]
fn registry_find_by_capability() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider).unwrap();

    let found = registry
        .find_by_capability(ModelModality::Text, Some(CapabilityFeature::Streaming))
        .unwrap();
    assert_eq!(found.provider_id().as_str(), "mock-provider");

    let err = match registry.find_by_capability(ModelModality::Audio, None) {
        Err(err) => err,
        Ok(_) => panic!("unsupported capability unexpectedly resolved"),
    };
    assert!(matches!(err, RegistryError::CapabilityMismatch(_)));
}

#[test]
fn registry_list_providers_deterministic() {
    let registry = ProviderRegistry::new();
    let p1 = Arc::new(MockProvider::new(
        ProviderId::parse("provider-a").unwrap(),
        "1.0",
    ));
    let p2 = Arc::new(MockProvider::new(
        ProviderId::parse("provider-b").unwrap(),
        "1.0",
    ));
    registry.register(p1).unwrap();
    registry.register(p2).unwrap();

    let list = registry.list_providers().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].as_str(), "provider-a");
    assert_eq!(list[1].as_str(), "provider-b");
}

#[test]
fn registry_list_enabled_only() {
    let registry = ProviderRegistry::new();
    let p1 = Arc::new(MockProvider::new(
        ProviderId::parse("enabled-provider").unwrap(),
        "1.0",
    ));
    let p2 = Arc::new(MockProvider::new(
        ProviderId::parse("disabled-provider").unwrap(),
        "1.0",
    ));
    registry.register(p1).unwrap();
    registry.register(p2).unwrap();
    registry
        .set_enabled(&ProviderId::parse("disabled-provider").unwrap(), false)
        .unwrap();

    let enabled = registry.list_enabled_providers().unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].as_str(), "enabled-provider");
}

#[test]
fn registry_seal_prevents_registration() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider).unwrap();
    registry.seal().unwrap();

    let pid = ProviderId::parse("mock-provider").unwrap();
    let err = registry.set_enabled(&pid, false).unwrap_err();
    assert!(matches!(err, RegistryError::Sealed));

    let new_provider = Arc::new(MockProvider::new(
        ProviderId::parse("new-provider").unwrap(),
        "0.1",
    ));
    let err = registry.register(new_provider).unwrap_err();
    assert!(matches!(err, RegistryError::Sealed));
}

#[test]
fn registry_seal_allows_reads() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider).unwrap();
    registry.seal().unwrap();

    let pid = ProviderId::parse("mock-provider").unwrap();
    assert!(registry.get(&pid).is_ok());
    assert!(registry.get_descriptor(&pid).is_ok());
    assert!(registry.list_providers().is_ok());
}

#[test]
fn registry_find_by_capability_none() {
    let registry = ProviderRegistry::new();
    let provider = mock_provider();
    registry.register(provider).unwrap();

    let err = match registry
        .find_by_capability(ModelModality::Video, Some(CapabilityFeature::AudioInput))
    {
        Err(err) => err,
        Ok(_) => panic!("unsupported capability unexpectedly resolved"),
    };
    assert!(matches!(err, RegistryError::CapabilityMismatch(_)));
}

#[test]
fn registry_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(ProviderRegistry::new());
    let provider = mock_provider();
    registry.register(provider).unwrap();

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let reg = registry.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    let _ = reg.get(&ProviderId::parse("mock-provider").unwrap());
                    let _ = reg.list_providers();
                    let _ = reg.list_enabled_providers();
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
}
