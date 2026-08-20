# Anthropic provider adapter contract

`provider-adapter-anthropic` is an isolated Anthropic adapter and descriptor over the shared `provider-core::transport` boundary.

## Descriptor

`AnthropicProviderDescriptor` exposes:

- provider ID `anthropic`;
- descriptor version `anthropic-descriptor-1`;
- deterministic mappings for `claude-3-5-sonnet` and `claude-3-7-sonnet`;
- explicit text/image/streaming/tool-use capabilities;
- explicit unsupported audio/video modes;
- bounded context/output limits.

No model discovery, OAuth, credential storage, UI, fallback routing, or secret defaults are included.

## Mapping

The adapter maps normalized requests to the Anthropic `/messages` shape, with:

- system messages separated into the system field;
- user/assistant/tool roles mapped within the adapter boundary;
- bounded `max_tokens`, temperature, and stream flag;
- `anthropic-version` header;
- opaque credential reference passed only to the injected transport.

Responses map Anthropic `content[]`, `stop_reason`, and usage into normalized response/stream contracts. HTTP errors are generic, typed, retryable where appropriate, and redacted.

## Shared transport boundary

`provider-core::transport` owns the provider-neutral HTTPS endpoint policy, bounded HTTP request/response values, credential-ref separation, cancellation-aware transport trait, and typed timeout/size errors. OpenAI-compatible reexports these types for compatibility; Anthropic consumes the shared core boundary directly.

## Tests

`crates/provider-adapters/anthropic/tests/provider_contract.rs` covers:

- deterministic descriptor/capability mapping;
- wrong provider, unknown model, and unsupported modality rejection;
- completion content/usage/identity mapping;
- stream sequence/terminal mapping;
- rate-limit, endpoint, credential, and secret-default restrictions.

## ONP mapping

- T-356 — Adicionar adapter/descriptor Anthropic e transport boundary compartilhado [concluida]