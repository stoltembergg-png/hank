# Gemini provider adapter contract

`provider-adapter-gemini` is an isolated Gemini adapter and descriptor over the shared `provider-core::transport` boundary.

## Descriptor

`GeminiProviderDescriptor` exposes:

- provider ID `gemini`;
- descriptor version `gemini-descriptor-1`;
- deterministic mappings for `gemini-1.5-pro` and `gemini-1.5-flash`;
- explicit text/image/vision/streaming/tool-use capabilities;
- explicit unsupported audio/video modes;
- bounded model-specific context/output limits.

No discovery, OAuth, credential storage, routing/fallback, UI, or SDK dependency is included.

## Mapping

Gemini request mapping is kept inside the adapter:

- normalized messages become `contents[].parts[]`;
- assistant role maps to Gemini `model` role;
- generation limits and temperature map to `generationConfig`;
- complete operation uses `models/{model}:generateContent`;
- streaming uses `models/{model}:streamGenerateContent`;
- credential ref is passed only through the shared transport boundary.

Responses map `candidates[].content.parts[]`, `finishReason`, `modelVersion`, and `usageMetadata` to normalized contracts. Unknown finish reasons remain explicit. HTTP errors are generic, typed, retryable where appropriate, and redacted.

## Tests

`crates/provider-adapters/gemini/tests/provider_contract.rs` covers:

- deterministic descriptor/capability mapping;
- wrong provider, unknown model, and unsupported audio rejection;
- candidates/generation/usage complete mapping;
- stream ordering and terminal semantics;
- rate-limit, timeout, endpoint, and credential fail-closed behavior.

## ONP mapping

- T-357 — Adicionar adapter/descriptor Gemini provider-neutral [concluida]