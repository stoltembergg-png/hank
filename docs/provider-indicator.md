# Honest provider/model indicator contract

`frontend/src/chat/indicators/ProviderIndicator.tsx` renders only normalized provider metadata supplied by an application boundary. It does not call providers, inspect credentials, resolve endpoints or infer capabilities from user/provider text.

## States

- `selected` → Modelo selecionado;
- `fallback` → Fallback ativo;
- `unknown` → Modelo desconhecido;
- `unavailable` → Provider indisponível;
- `degraded` → Capability degradada.

Capability state is independently explicit: confirmed, unknown or unsupported. Unknown capability never renders a supported/confirmed claim.

Provider/model identifiers are bounded and reject URLs, whitespace/control characters and secret-like markers. Invalid or absent metadata renders generic `Provider não identificado` / `Modelo não identificado`. Optional attempt number is bounded to 1..1000.

The indicator is an optional `ChatPage` prop so the UI cannot invent provider metadata when no normalized event/service data exists. No raw account, endpoint, token, credential or secret is rendered or logged.

## Tests

`frontend/tests/provider-indicator.test.tsx` covers selected/fallback/unknown/unavailable/degraded states, normalized identity/attempt, malformed-secret metadata redaction and accessible missing-metadata fallback.

## ONP mapping

- T-387 — Adicionar provider/model indicators honestos [concluida]