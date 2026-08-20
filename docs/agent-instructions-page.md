# Agent Instructions Page contract

`InstructionsPage` edits only the `Agent` layer of the instruction hierarchy through a dedicated `AgentInstructionApiClient` boundary. It never edits or exposes system/security layers, sends prompts, or mutates runtime execution.

## Scope and invariants

- Fixed layer: `agent`; no layer selector is exposed;
- Content is bounded by the snapshot's `max_total_bytes` budget;
- Oversized content is rejected and never silently truncated;
- Provenance is displayed (`agent`, `project`, or `user` metadata from the service);
- Content is explicitly marked untrusted;
- Preview uses plain text (`pre` text nodes), never HTML interpretation;
- Optimistic version (`updated_at`) is sent on update;
- Only `{ project_id, agent_id, layer: 'agent', content, max_total_bytes, expected_version }` is sent;
- System/security layers and prompt-send/runtime actions are outside the page.

## Typed service boundary

`DesktopAgentInstructionApiClient` invokes only:

- `get_agent_instruction`
- `update_agent_instruction`

Without an instruction service bridge, the UI shows an explicit unsupported state rather than editing a fabricated hierarchy.

## Lifecycle and errors

- Loading and unsupported states use `role="status"`;
- Invalid layer or budget snapshots render an alert and no unsafe content;
- Stale/concurrency errors remain visible without navigation;
- Cancel without changes returns immediately; unsaved changes require confirmation;
- Save is disabled when unchanged or submitting;
- Character count and budget are announced with accessible metadata.

## Tests

`frontend/tests/agent_instructions_ac_tests.test.tsx` covers:

- Loading, Agent-layer form, budget, provenance, and untrusted marker;
- Agent-only update payload with version;
- Budget overflow rejection without truncation;
- No system/security selector or prompt-send action;
- Malformed non-Agent snapshot rejection;
- Stale conflict handling;
- Cancel confirmation and plain-text preview.

## ONP mapping

- T-347 — Adicionar editor de instruções da camada Agent [concluida]