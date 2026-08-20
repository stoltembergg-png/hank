# Agent Autonomy Page contract

`AutonomyPage` edits the Agent autonomy policy through a dedicated `AutonomyApiClient` boundary. It displays level consequences and decisions but does not execute operations, schedule work, modify skills, create workflows, or allow an LLM to change policy.

## Levels and invariants

The page exposes the closed L0–L4 enum:

- L0 — Nenhum;
- L1 — Assistido;
- L2 — Semi-autônomo;
- L3 — Autônomo;
- L4 — Totalmente autônomo.

Policy flags are checked against the domain invariants for L0, L1, and L2. The consecutive-step bound is 1..1000. The page renders the decision matrix for `read_data`, safe/stateful tools, subagents, workflows, skills, network, and system configuration.

## Transition safety

- Downgrades are reversible and do not require approval;
- Escalations require bounded `approver_id` (max 128), reason (max 256), and optional expiration;
- Missing/invalid approval blocks the update before the service call;
- Stale/concurrency failures remain visible and keep the selected form state for rollback/retry;
- Unsupported service state is explicit and never enables autonomous behavior;
- No approval is inferred from an LLM or UI preview.

## Typed service boundary

`DesktopAutonomyApiClient` invokes only:

- `get_agent_autonomy_policy`
- `update_agent_autonomy_policy`

The update DTO contains project/agent scope, the complete validated policy, optional transition approval, and `expected_version`.

## Security boundary

The page has no controls for scheduler, execution, Git/PR workflow, self-evolution, secrets, or tool invocation. It shows the fail-closed message `sem autoelevação silenciosa` and that the LLM cannot alter the policy.

## Tests

`frontend/tests/agent_autonomy_ac_tests.test.tsx` covers:

- Loading, current level, consequence matrix, and approval indicators;
- Unauthorized escalation rejection;
- Approved escalation payload;
- Downgrade without approval;
- Malformed policy flag rejection;
- Stale conflict and visible rollback state;
- Unsupported service state;
- Cancel confirmation and accessibility/security metadata.

## ONP mapping

- T-348 — Adicionar página de autonomia do Agent [concluida]