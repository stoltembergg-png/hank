# Model selector contract

`ModelSelectorPage` consumes a typed discovery/application-service snapshot and persists only an abstract provider/model selection through a typed bridge command.

## Selection rules

- only `available` options with `supported` state for every policy modality are selectable;
- unknown, unsupported, disabled, expired, unavailable, malformed, whitespace-containing, oversized, or endpoint-like identifiers are disabled;
- unavailable options remain visible with an explicit bounded reason;
- no automatic fallback is attempted;
- an empty compatible set produces an explicit degradation state;
- selection updates carry project, agent, provider, model and optimistic `expected_version` metadata;
- stale/concurrency conflicts are shown without navigation or silent overwrite;
- loading and service errors are explicit and do not fabricate options.

## Boundary and security

The default desktop client exposes only `get_model_selector_snapshot` and `update_model_selection`. The component has no network call on uncontrolled render, provider SDK, credential input, token, key, secret, arbitrary endpoint, model installation or prompt execution. Untrusted reasons are redacted before entering the DOM.

## Accessibility

- semantic `main` and header landmarks;
- labeled `radiogroup` with keyboard-actionable native radios;
- disabled incompatible options remain discoverable with reasons;
- `role=status` for loading/degradation;
- `role=alert` for load/save/conflict errors;
- visible focus styles and bounded responsive layout.

## Tests

`frontend/tests/model_selector_ac_tests.test.tsx` covers:

- loading state;
- compatible options and disabled/expired/unknown reasons;
- modality/capability filtering;
- service-only persistence;
- stale conflict preservation;
- no-compatible-options degradation;
- secret/token/endpoint redaction;
- accessibility landmarks and controls.

## ONP mapping

- T-367 — Adicionar model selector provider-neutral [concluida]