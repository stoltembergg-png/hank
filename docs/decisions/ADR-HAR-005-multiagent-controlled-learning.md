# ADR-HAR-005 — Multi-agent coordination and controlled learning

- **Status:** proposed; activates only after PR-270 baseline PASS.
- **Decision:** multi-agent coordination uses project-scoped structured blackboard objects, leases with generation/fencing/expiry, and minimal HandoffPackets. Chat text is not the coordination authority.
- **Learning:** runs may create candidates for memory, skill improvements and Harness improvements. Candidates proceed through evaluation, deterministic benchmark/baseline comparison, policy/human approval, versioned activation and rollback; no direct self-modification.
- **Reviewer rule:** executor cannot be the independent reviewer or approval authority. Shadow agents have zero write authority.
- **Consequences:** collaboration is crash-recoverable and auditable; costs, budgets and evidence are tied to run/agent identities.
- **Rejected:** unbounded delegation, implicit ownership, auto-activation, shared global memory, or shadow external effects.
- **Proof required:** lease race/takeover, handoff minimality, blackboard conflict, self-approval, candidate activation/rollback and multi-agent E2E tests.
- **Rollback:** revoke active leases/skills/candidates, fence outstanding agents, retain audit lineage and return to pinned versions.
