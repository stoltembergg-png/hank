# Normalized Response contract

`provider_core::response::NormalizedResponse` is the provider-neutral response envelope consumed after adapter mapping. It does not implement transport, streaming, persistence, or provider-specific SDK conversion.

## Envelope

The response carries:

- schema version;
- request/correlation IDs;
- opaque provider/model metadata;
- terminal `ResponseStatus`;
- forward-compatible `FinishReason`;
- bounded output parts;
- optional usage and cost;
- bounded typed provider error;
- provider contract version and latency metadata.

## Terminality and forward compatibility

Statuses distinguish:

- `complete`;
- `error`;
- `cancelled`;
- `limit`;
- `unknown`.

Finish reasons include stop, length, content filter, tool call, cancelled, error, and explicit unknown. Unknown status/reason never claims successful completion.

## Usage/cost semantics

`usage` and `cost` are optional. Missing data remains absent rather than being represented as fabricated zero values. Cost is bounded by micro-units and a short alphabetic currency code.

## Error taxonomy and redaction

Provider errors carry a stable code, bounded redacted message, and retryability flag. Error messages containing API keys, authorization headers, passwords, secrets, tokens, or bearer values are rejected. Raw provider payloads are not included in summaries.

## Bounds

- Output parts: max 64;
- Part: max 1 MiB;
- Total output: max 2 MiB;
- Provider version: max 64 characters;
- Error detail: max 1024 characters;
- Latency: max 24 hours;
- Usage/cost values are bounded.

`redacted_summary()` exposes IDs, status, finish reason, counts, sizes, presence of usage/cost, and error code only.

## Tests

`crates/provider-core/tests/response_contract.rs` covers:

- deterministic serde roundtrip and redacted summary;
- complete/error/cancelled/limit/unknown status handling;
- unknown finish reason forward compatibility;
- optional usage/cost without false zero values;
- oversized parts, missing error, unredacted secrets, and invalid response rejection;
- retryability taxonomy without raw provider payload.

## ONP mapping

- T-352 — Definir normalized response provider-neutral [concluida]