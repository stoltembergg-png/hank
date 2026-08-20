# Provider registry contract

`provider-core::registry` is the provider-neutral application boundary for registering and resolving model providers. It owns provider identity, immutable descriptor metadata, enabled/disabled lifecycle, capability lookup, deterministic listing, and registry sealing.

## Contract

- Registration accepts an `Arc<dyn ModelProvider>` and stores a cloned opaque adapter handle;
- provider IDs are ordered and duplicate registration fails explicitly;
- lookup returns only enabled providers;
- disabled providers remain listed/described but cannot be resolved for execution;
- capability lookup considers enabled providers only and returns an explicit mismatch when none qualifies;
- provider listing is deterministic because storage is a `BTreeMap`;
- `seal()` prevents registration and enable/disable mutation while preserving reads;
- the registry uses `RwLock` and is safe for concurrent read access;
- no credential material, fallback execution, plugin loading, UI, or provider-specific logic is introduced.

## Capability source

`ModelProvider::capabilities()` now returns the canonical `provider_core::capabilities::CapabilityReport`. The former `ProviderCapabilities` summary type had no remaining workspace consumers and was removed after the workspace-wide usage audit.

## Tests

`crates/provider-core/tests/registry_contract.rs` covers:

- valid registration;
- duplicate ID;
- existing/missing lookup;
- enable/disable;
- disabled provider rejection;
- descriptor retrieval;
- capability filtering and mismatch;
- deterministic registered/enabled listing;
- sealed registry reads;
- registration and lifecycle mutation after seal;
- concurrent reads across multiple threads.

## ONP mapping

- T-360 — Implementar provider registry provider-neutral [concluida]