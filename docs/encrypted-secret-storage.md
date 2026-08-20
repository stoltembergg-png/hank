# Encrypted secret storage boundary

`secrets-core` defines the secure storage seam required by the credential service without introducing plaintext persistence or provider-specific code.

## Backend contract

`SecureSecretBackend` represents a platform-provided OS keychain or Tauri Stronghold backend. It exposes only bounded operations:

- `put`;
- `get`;
- `delete`;
- `rotate`;
- capability/status reporting.

`SecureSecretStore<B>` enforces project/account binding, cancellation, backend availability, opaque `CredentialRef` identity, and bounded `SecretMaterial` before delegating to the backend.

`SecretMaterial` has no public `Debug`/`Display` payload and zeroes its owned byte buffer on drop. Errors expose only stable categories; no secret bytes are included.

## Fail-closed policy

No fallback to SQLite, `.env`, localStorage, files, logs, artifacts, or plaintext configuration exists. If the platform backend reports unavailable, every operation returns `Unavailable`.

The current integration test backend is an in-memory mock keychain. It is explicitly test-only and makes no secure-persistence claim. OS keychain and Tauri Stronghold implementations remain platform adapters behind this boundary.

## Tests

`crates/secrets-core/tests/secret_store_contract.rs` covers:

- mocked keychain roundtrip;
- project scope and account binding;
- delete/revoke;
- rotation under the same opaque ref;
- unavailable backend;
- bounded/malformed material;
- cancellation before backend access.

## ONP mapping

- T-362 — Adicionar encrypted secret storage boundary [concluida]