# OAuth callback handling contract

`auth-core::callback` connects a bounded deep-link callback parser to the OAuth flow manager and credential handoff without adding Tauri commands or provider-specific code.

## Callback route

Only the exact `hank://oauth/callback` route is accepted. The query must contain exactly one bounded value for each field:

- `flow`;
- `provider`;
- `account`;
- `state`;
- `code`.

Unknown fields, duplicates, malformed values, fragments, control characters, foreign schemes, and oversized URLs fail closed. The parser never logs or displays authorization code material.

## Binding and lifecycle

`OAuthCallbackHandler` binds provider/account/project and browser redirect at flow start. Completion requires:

- authorized project context;
- matching provider;
- matching account;
- matching OAuth state and redirect;
- valid PKCE verifier;
- unexpired, uncancelled, not-yet-consumed flow.

A valid callback dispatches the existing token exchange backend and returns only `CredentialRef` metadata. A successful flow cannot be consumed twice. Invalid, foreign, replayed, expired, cancelled, and exchange failures have typed terminal errors.

No open redirect, generic invoke, provider API call, token storage, UI callback, or deep-link permission broadening is introduced.

## Tests

`crates/auth-core/tests/callback_contract.rs` covers:

- valid callback and opaque credential result;
- malformed route/query and duplicate/unknown fields;
- provider/account/project mismatch;
- state mismatch;
- replay and timeout;
- code redaction.

## ONP mapping

- T-364 — Adicionar tratamento de OAuth callback provider-neutral [concluida]