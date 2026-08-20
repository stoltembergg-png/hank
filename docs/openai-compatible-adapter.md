# OpenAI-compatible adapter contract

`provider-adapter-openai-compatible` is an isolated protocol adapter over `provider-core`. It contains no real HTTP client, credential storage, OAuth, registry, UI, retry orchestration, shell access, or tool execution.

## Boundary

The adapter accepts normalized request/response/stream contracts and an injected `HttpTransport`:

- `EndpointPolicy` accepts only bounded HTTPS base URLs without userinfo, query, fragment, control characters, or host port syntax;
- `CredentialRef` remains opaque and is passed to transport separately from the serialized request body;
- transport receives a bounded `HttpRequest`, timeout, and cancellation token;
- tests use an offline in-memory transport fixture.

A production transport is responsible for resolving the opaque credential reference. This crate never reads or stores a secret.

## Completion mapping

The adapter maps:

- normalized message roles to compatible message roles;
- model, token budget, temperature, stream flag, and metadata-only tools to a bounded JSON body;
- successful choices to `NormalizedResponse` text parts;
- provider finish reasons to the normalized taxonomy;
- optional prompt/completion usage without fabricating missing values;
- HTTP status classes to typed normalized provider errors.

HTTP errors are generic and redacted. Provider response payloads are not copied into `ProviderErrorInfo`.

## Streaming mapping

Offline JSON chunk fixtures map to `StreamEvent` start/delta/tool/usage/finish/error/cancel events. `StreamValidator` enforces sequence, generation, terminality, and no post-terminal data. An incomplete stream is rejected explicitly.

Cancellation is deterministic: a cancelled transport returns `TransportError::Cancelled`, and cancellation observed during chunk mapping produces one terminal cancel event.

## Bounds and failure modes

- Request and response bodies: max 2 MiB;
- Adapter timeout must be non-zero;
- malformed JSON returns `MalformedResponse`;
- 429 maps to retryable `RateLimited`;
- timeout/cancel are typed transport errors;
- oversized payloads fail before mapping;
- invalid endpoint and credential references fail closed;
- no implicit retry is performed.

## Tests

`crates/provider-adapters/openai-compatible/tests/adapter_contract.rs` covers:

- complete mapping and correlation preservation;
- credential redaction from body and `Debug` request output;
- streaming chunk mapping and terminality;
- rate limit, timeout, and cancellation taxonomy;
- malformed response and incomplete stream rejection;
- endpoint, credential, and size validation;
- provider error redaction for secret-like payloads.

## ONP mapping

- T-354 — Implementar adapter OpenAI-compatible isolado [concluida]