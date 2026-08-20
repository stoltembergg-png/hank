# Model discovery service contract

`provider-core::discovery` resolves normalized model metadata through the provider registry and credential service before any selector UI consumes it.

## Contract

- requires project-scoped credential resolution before discovery;
- rejects missing, revoked, unavailable, cancelled, cross-project and disabled-provider requests;
- reads provider/model identity and canonical `CapabilityReport` from the registry;
- checks `CapabilityRequirement` before returning a model;
- returns bounded normalized records with provider, model, capability, source, and only a boolean credential-ref availability marker;
- paginates with a maximum page size of 64;
- caches only normalized metadata/capabilities, never credential refs or provider payloads;
- cache entries have bounded TTL and explicit invalidation;
- unknown/unsupported capabilities are returned as explicit compatibility errors;
- no selector UI, fallback, arbitrary endpoint, raw provider payload, or secret is introduced.

## Tests

`crates/provider-core/tests/discovery_contract.rs` covers:

- normalized model/capability result;
- missing/revoked credentials;
- capability requirements;
- bounded pagination;
- cache hit and invalidation;
- disabled provider and cancellation;
- credential/provider payload redaction.

## ONP mapping

- T-366 — Adicionar model discovery service [concluida]