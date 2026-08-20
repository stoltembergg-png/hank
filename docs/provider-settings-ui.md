# Provider settings UI contract

`ProviderSettingsPage` is a service-only frontend for provider account status and connection intents.

## Boundary

The page receives an injected `ProviderSettingsApiClient`. The default desktop client exposes only typed bridge commands:

- `list_provider_accounts`;
- `start_provider_oauth`;
- `get_provider_oauth_status`;
- `disconnect_provider_account`.

No generic invoke, provider SDK, adapter, SQLite, localStorage, browser storage, credential material, authorization code, token, or model discovery is used.

## States and actions

The page renders bounded provider/account metadata and explicit states:

- connected;
- pending OAuth;
- revoked;
- unavailable;
- error.

Connect starts a typed OAuth intent, shows a pending state, then consumes only a status result. Callback results are checked for flow identity and project binding; stale, foreign, invalid, expired, or cancelled states remain an error and cannot mutate account status. Disconnect sends a typed service intent and renders the returned revoked status.

Only `has_credential_ref` metadata is displayed as `Credential ref opaco disponível`; the opaque reference value and all authorization data stay out of the DOM and logs.

## Accessibility

- semantic `main`, `header`, `section` and `article` landmarks;
- labeled provider status region;
- keyboard-actionable buttons;
- `role=alert` for errors;
- `role=status` for pending/success feedback;
- visible focus styles and bounded responsive layout.

## Tests

`frontend/tests/provider_settings_ac_tests.test.tsx` covers:

- loading/list/status states;
- no secret/code rendering;
- typed OAuth start and pending state;
- successful callback status;
- invalid and stale callback errors;
- typed disconnect;
- accessibility landmarks and controls.

## ONP mapping

- T-365 — Adicionar provider settings UI service-only [concluida]