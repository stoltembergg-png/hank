# Credential service contract

`provider-core::credentials` defines the application boundary between provider settings/use cases and secure credential storage.

## Boundary

The service accepts only an opaque `CredentialRef`. It never accepts API keys, tokens, passwords, connection strings, or plaintext secret material. Secure OS keychain/Stronghold storage is deliberately deferred to PR-069.

`CredentialAccount` binds a provider account to a project scope. Every operation receives `CredentialAccessContext`; cross-project access is rejected before lookup, and cancelled operations fail closed.

## Lifecycle

- `connect` binds an opaque ref to an authorized project/provider/account;
- duplicate active connections return `Conflict`;
- `status` returns bounded account metadata, state, and an optional opaque ref;
- `resolve_ref` returns a ref only for an authorized connected account;
- `disconnect` transitions the account to `Revoked` and clears the returned ref;
- missing, revoked, unavailable, invalid, unauthorized, and cancelled states are typed;
- service errors never include secret material.

`InMemoryCredentialService` is a deterministic contract fixture only. It stores opaque refs in a bounded in-memory map and is not a persistence or secure-storage implementation.

## Tests

`crates/provider-core/tests/credentials_contract.rs` covers:

- connect/status/ref lifecycle;
- cross-project authorization failure;
- revocation and post-disconnect resolution failure;
- plaintext credential rejection at the opaque-ref boundary;
- unavailable service fail-closed behavior;
- cancellation;
- bounded deterministic identities and redaction;
- concurrent status reads.

## ONP mapping

- T-361 — Adicionar credential service provider-neutral [concluida]