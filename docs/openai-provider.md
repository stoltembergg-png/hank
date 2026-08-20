# OpenAI provider descriptor contract

`provider-adapter-openai` declares the OpenAI provider identity and planning-time model mapping over `provider-adapter-openai-compatible`.

## Descriptor

`OpenAiProviderDescriptor` exposes:

- provider ID `openai`;
- descriptor version `openai-descriptor-1`;
- deterministic model entries for `gpt-4o-mini` and `gpt-4o`;
- capability reports with explicit modalities, features, limits, source, and version.

The descriptor is static. It performs no model discovery, network call, fallback routing, credential storage, OAuth flow, or UI configuration.

## Capability mapping

- Both declared models support text, streaming, and tool-use capability negotiation;
- `gpt-4o` declares image/vision support;
- `gpt-4o-mini` rejects image/vision requests explicitly;
- unsupported audio/video modes are explicit failures;
- context/output limits are bounded and deterministic.

Unknown or undeclared models fail before the adapter. Requests targeting another provider fail with `ProviderMismatch`.

## Adapter wiring

`OpenAiProvider<T>` wraps the compatible adapter with an injected `HttpTransport`, `EndpointPolicy`, opaque `CredentialRef`, and bounded timeout. It validates provider/model/capability compatibility before delegation and rewrites only the normalized provider identity from `openai-compatible` to `openai`.

Credentials remain outside request JSON and are never stored, logged, or resolved by this crate.

## Tests

`crates/provider-adapters/openai/tests/provider_contract.rs` covers:

- deterministic descriptor models/version/capabilities;
- wrong-provider, unknown-model, and unsupported-capability rejection;
- complete mapping with OpenAI identity;
- stream mapping with OpenAI identity and terminal event;
- endpoint/credential restrictions and absence of unknown capability defaults.

## ONP mapping

- T-355 — Adicionar descriptor/provider OpenAI provider-neutral [concluida]