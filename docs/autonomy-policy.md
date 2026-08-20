# Autonomy Policy Contract

`AutonomyPolicy` defines the formal autonomy ladder (L0–L4) for agents, governing execution bounds, tool invocation rules, and human approval boundaries.

## Autonomy Levels (L0–L4)

- **L0 (None):** Read-only and suggestion mode. Any stateful or safe tool execution requires human approval. No sub-agents or workflow spawning.
- **L1 (Assisted):** Read operations and pure tools are executed automatically. Stateful operations and sub-agent spawning require human approval.
- **L2 (Semi-Autonomous):** Bounded execution within project scope. Sub-agents and workflow executions are permitted within budget. External network access and skill modifications require human approval. System config modifications are denied.
- **L3 (Autonomous):** Autonomous multi-step orchestration under strict project policies and quotas.
- **L4 (Fully Autonomous):** Full autonomous operation within sandboxed boundaries.

## Security & Transition Invariants

- **Fail-Closed by Default:** Unknown fields and unauthorized operations fail closed.
- **Explicit Escalation Approval:** Any level escalation (e.g. L1 to L2 or L0 to L4) requires an explicit, authenticated, and unexpired `AutonomyTransitionApproval`. LLMs cannot self-escalate autonomy levels.
- **Reversibility:** Downgrading an agent's autonomy level is always permitted and safe.
- **No Self-Evolution Bypass:** Runtime changes to global policies or self-modification must proceed through the versioned Git / PR / review / release pipeline.
