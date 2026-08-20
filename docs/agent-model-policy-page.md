# Agent Model Policy Page contract

`ModelPolicyPage` edits an abstract model policy through a dedicated `ModelPolicyApiClient` service boundary. It does not use `AgentApiClient`'s full policy update because the current runtime DTO requires a complete `AgentPolicyConfig`; sending a partial policy would be unsafe and non-functional.

## Scope

- Abstract provider and model identifiers;
- Bounded max tokens, context window, and temperature;
- Explicit required modalities: text, image, audio, video;
- Capability hints with `supported`, `unsupported`, or `unknown` state;
- Provider state: `available`, `unsupported`, or `unknown`;
- Optimistic versioned update through a typed bridge boundary;
- Clear no-provider/unsupported state;
- Accessible form, cancel, stale conflict, and validation feedback.

Out of scope:

- SDK/provider selection or discovery;
- Network calls, provider health checks, or adapters;
- API keys, passwords, tokens, endpoints, URLs, or credential storage;
- Arbitrary model parameter maps;
- Fallback executor or runtime model routing.

## Typed service boundary

The page consumes:

```ts
apiClient.get(projectId, agentId)
apiClient.update({
  project_id,
  agent_id,
  policy,
  expected_version,
})
```

`DesktopModelPolicyApiClient` invokes `get_agent_model_policy` and `update_agent_model_policy` only when the typed desktop bridge exists. In browser/test environments without a bridge, `get` returns `null`, which renders an explicit unsupported/no-provider state instead of fabricated support.

## Policy bounds

- Provider ID: required, maximum 120 characters, no URL/endpoint syntax;
- Model ID: required, maximum 200 characters, no URL/endpoint syntax;
- `max_tokens`: optional integer 1..1,000,000;
- `max_context_tokens`: optional integer 1..2,000,000;
- `temperature`: optional finite number 0..2;
- At least one unique modality is required;
- Only allowlisted fields are sent; no `parameters` map is exposed or emitted.

## Capability semantics

- `supported` means the service explicitly reported support;
- `unsupported` is visible and never converted into support;
- `unknown` remains unknown and is not treated as supported;
- Provider state `unsupported` and `unknown` are shown as non-success states while preserving the abstract policy fields.

## Lifecycle and safety

- Loading state uses `role="status"`;
- Fetch failure is retryable and uses `role="alert"`;
- Save is disabled without changes or while submitting;
- Stale/version errors remain visible and do not navigate away;
- Cancel without changes returns immediately; unsaved changes require confirmation;
- No provider, SDK, endpoint, network, or credential control is rendered.

## Tests

`frontend/tests/agent_model_policy_ac_tests.test.tsx` covers:

- Loading and provider-neutral form;
- Capability states and provider unsupported state;
- Allowlisted update payload and optimistic version;
- URL/endpoint rejection;
- Numeric bounds and temperature validation;
- Modality requirement;
- No-provider state without invented support;
- Stale conflict handling;
- Cancel/confirmation behavior;
- Accessible form and absence of credential controls.

## ONP mapping

- T-345 — Adicionar página de política de modelo do Agent [concluida]