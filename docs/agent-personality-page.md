# Agent Personality Page contract

`PersonalityPage` edits only the Agent `Personality` schema through the injected `AgentApiClient` boundary. It does not access storage, Tauri internals, providers, prompts, memory, or local persistence.

## Scope

- Personality name, description, traits, and communication style;
- Bounded validation aligned with the domain schema;
- Safe plain-text preview;
- Explicit Agent-layer precedence metadata;
- Save/cancel flows with optimistic concurrency;
- Stale, archived/inactive, forbidden, and generic error handling.

Out of scope:

- System/security/project/workflow instructions;
- Model or provider selection;
- Tool permissions, autonomy, memory, skills, or execution;
- Prompt rendering or HTML interpretation;
- Local storage of drafts.

## Service boundary

The page receives an `AgentApiClient` and uses:

```ts
apiClient.get(projectId, agentId)
apiClient.update({
  project_id,
  agent_id,
  personality,
  expected_version,
})
```

The update DTO contains only the personality fields and the optimistic version. Identity, status, policy, and other Agent fields are not sent by this page.

## Bounds and validation

- Personality name: required, trimmed, maximum 120 characters;
- Description: optional, trimmed, maximum 4,000 characters;
- Traits: maximum 32 entries, no blank entries, maximum 80 characters each;
- Communication style: closed enum (`formal`, `casual`, `technical`, `concise`, `verbose`);
- Rejects `api_key`, `authorization:`, `password`, and `ignore previous instructions` markers case-insensitively;
- Invalid input is shown in an alert and never reaches the update service;
- Oversized content is rejected, never silently truncated.

## Precedence and preview

The page displays:

- `Camada: Agent`;
- A warning that personality guides style and tone but does not replace security/system instructions;
- A plain-text safe preview using React text nodes, never HTML injection;
- No controls for security/system layers or arbitrary instructions.

## Lifecycle and errors

- Loading state is announced with `role="status"`;
- Missing/load failures show retryable `role="alert"`;
- Inactive/suspended Agents have editing controls disabled and display a terminal notice;
- Save uses `expected_version` from the last fetch;
- Stale/concurrency failures remain on the page with a reload/retry message;
- Cancel without changes returns immediately; unsaved changes require explicit confirmation;
- Submit is locked while the update is in flight.

## Tests

`frontend/tests/agent_personality_ac_tests.test.tsx` covers:

- Loading and loaded form states;
- Agent-layer precedence warning and plain-text preview;
- Personality-only update payload and version propagation;
- Required, oversized, blank-trait, and injection/secret validation;
- Stale update error mapping;
- No-change and cancel behavior;
- Unsaved-change confirmation;
- Inactive-agent protection;
- Accessible labels, form, and controls.

## ONP mapping

- T-344 — Adicionar página de personalidade do Agent [concluida]