# Provider health check contract

`provider-core::health` provides bounded provider availability evidence without running chat generation or exposing credential material.

## Input and boundary

A health request is project-scoped and names only provider/account metadata. The service resolves the opaque credential through `CredentialService`, checks `ProviderRegistry` enabled state, and delegates the bounded probe to an injected `HealthProbe`. `DefaultHealthProbe` calls the existing provider-neutral `ModelProvider::health()` contract; tests can inject deterministic outcomes for external categories.

## Stable statuses and reasons

Results expose `check_id`, provider/account metadata, stable `HealthStatus`, stable `HealthReason::code()`, cache-hit state, measured probe latency, and evidence timestamp. The taxonomy distinguishes:

- healthy;
- provider degraded/unhealthy;
- unconfigured/missing or revoked credential;
- disabled or missing provider;
- rate limited;
- quota exceeded;
- timeout;
- outage;
- invalid credential;
- unsupported.

No result can claim `Healthy` when credential resolution fails, the provider is disabled, or the probe returns a failure category.

## Bounds, cancellation and debounce

`HealthCheckPolicy` bounds timeout, minimum interval and cache age. Results are cached only by project/provider/account metadata, never by credential ref or raw provider response. Requests inside the minimum interval return the prior evidence with `cache_hit=true`, preventing probe storms; expired cache entries are removed. Cancellation is checked before credential resolution, before probing, and by the probe future.

Timeout/rate/quota/outage/invalid-credential categories are supplied by the injected probe boundary and remain explicit in the normalized result. No retry storm, fallback execution, arbitrary endpoint, cloud telemetry or health UI is introduced.

## Tests

`crates/provider-core/tests/health_contract.rs` covers:

- healthy evidence and redaction;
- missing and revoked credentials;
- rate limit, quota, timeout, outage, invalid credential and unsupported mappings;
- disabled provider;
- debounce/cache behavior;
- cancellation;
- policy/request bounds;
- default probe delegation to `ModelProvider::health()`.

## ONP mapping

- T-368 — Adicionar provider health check service [concluida]