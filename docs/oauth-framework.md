# OAuth framework contract

`auth-core` defines the provider-neutral OAuth flow boundary. It does not contain provider client IDs, client secrets, authorization endpoints, callback UI, token storage, or provider API calls.

## Flow

- `OAuthFlowManager::begin` binds provider identity, exact redirect URI, state, S256 PKCE challenge, deadline, and a bounded one-shot flow ID;
- `OAuthCallback` carries only redacted wrapper types for state, redirect, and authorization code;
- `complete` validates cancellation, expiry, replay, state, exact redirect, and S256 verifier/challenge before exchange;
- `TokenExchangeBackend` is injected and returns only an opaque `CredentialRef` for credential-service handoff;
- failed or unavailable exchange never exposes code/token material;
- expired sessions are purged and active flow count is bounded.

## Security boundaries

- redirect URIs accept HTTPS or loopback HTTP only;
- callback state is exact-match and single-use;
- PKCE uses SHA-256 and base64url without padding;
- authorization code and verifier Debug output is redacted;
- malformed state, code, verifier, redirect, token, replay, timeout, cancellation and capacity failures are typed;
- no token is persisted, logged, placed in a URL, or returned to UI.

## Tests

`crates/auth-core/tests/oauth_contract.rs` covers:

- authorization request construction;
- opaque credential handoff;
- wrong state/redirect and replay;
- expiry and cancellation;
- malformed token and redirect validation;
- sensitive wrapper redaction.

## ONP mapping

- T-363 — Adicionar OAuth framework provider-neutral [concluida]