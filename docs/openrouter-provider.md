# OpenRouter provider adapter contract

`provider-adapter-openrouter` is an isolated route descriptor and wrapper over the existing OpenAI-compatible adapter.

## Route identity

The descriptor declares exactly one direct route for each logical model:

- `openai/gpt-4o-mini` → provider `openai`, model `gpt-4o-mini`;
- `anthropic/claude-3-5-sonnet` → provider `anthropic`, model `claude-3-5-sonnet`.

`RouteMetadata` preserves logical model, upstream provider, upstream model, and route label. The table is deterministic and bounded; no fallback or hidden upstream route is synthesized.

## Validation and mapping

Before delegation, the wrapper validates:

- provider identity is `openrouter`;
- logical route is declared;
- normalized request is valid;
- requested capabilities are supported by the declared route.

The wrapper delegates only the validated upstream model to the compatible adapter, then rewrites response/stream identity back to `openrouter` and the logical route model. Upstream status/error categories remain explicit.

## Security boundaries

- Endpoint must pass the shared HTTPS policy;
- credential remains an opaque `CredentialRef`;
- no credential UI, OAuth, fallback, arbitrary endpoint injection, or policy bypass;
- no prompt, route secret, or raw upstream error is logged by this crate;
- no tool execution is introduced.

## Tests

`crates/provider-adapters/openrouter/tests/provider_contract.rs` covers:

- deterministic route metadata and capabilities;
- wrong provider, unknown route, and unsupported capability rejection;
- complete mapping with logical provider/model identity;
- streaming terminal identity preservation;
- retryable upstream error and endpoint/credential restrictions.

## ONP mapping

- T-358 — Adicionar adapter/descriptor OpenRouter sem fallback implícito [concluida]