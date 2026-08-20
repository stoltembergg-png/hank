# Ollama provider adapter contract

`provider-adapter-ollama` is an isolated Ollama adapter with localhost endpoint validation.

## Descriptor

`OllamaProviderDescriptor` exposes:

- provider ID `ollama`;
- descriptor version `ollama-descriptor-1`;
- deterministic mappings for `llama3.1:8b` and `llama3.2:3b`;
- explicit text capability;
- explicit unsupported image/audio/video modes;
- streaming supported;
- tool-use, vision, audio-input explicitly unsupported;
- bounded model-specific context/output limits.

No discovery, OAuth, credential storage, model installation, process launch, or shell execution is included.

## Mapping

Ollama request mapping is kept inside the adapter:

- normalized messages become Ollama `messages[]` with role mapping;
- generation limits and temperature map to `options.num_predict` and `options.temperature`;
- operation uses `POST /api/chat`;
- credential ref is passed only through the shared transport boundary.

Responses map `message.content`, `done`, `model`, and `eval_count` to normalized contracts. Streaming uses incremental JSON chunks with `message.content` and `done`. HTTP errors are generic, typed, retryable where appropriate, and redacted.

## Endpoint validation

Only HTTPS endpoints with a localhost host (`localhost`, `127.0.0.1`, or `::1`) are accepted. The allowlist is enforced in `provider-core::transport::EndpointPolicy`. Arbitrary remote endpoints and non-HTTPS URLs are rejected.

## Tests

`crates/provider-adapters/ollama/tests/provider_contract.rs` covers:

- deterministic descriptor/capability mapping;
- wrong provider, unknown model, and unsupported capability rejection;
- complete mapping with logical provider/model identity;
- streaming terminal identity preservation;
- endpoint format validation and credential restrictions.

## ONP mapping

- T-359 — Adicionar adapter/descriptor Ollama com validação de endpoint local [concluida]