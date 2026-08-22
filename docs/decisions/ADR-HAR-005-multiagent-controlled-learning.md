---
id: ADR-HAR-005
status: proposed
owner: architecture-owner
date: 2026-08-22
---

# ADR-HAR-005 — Multi-agent coordination and controlled learning

## Context

Chat-only coordination, unbounded delegation and direct self-improvement create races, authority leakage and unreviewable mutations.

## Decision

Agents coordinate through project-scoped typed blackboard objects, leases with fencing/expiry, and minimal HandoffPackets. Learning produces candidates that require evaluation, baseline comparison, approval, versioned activation and rollback. Shadow agents have zero write authority.

## Alternatives

- Shared chat as the source of coordination truth: rejected because it lacks concurrency and ownership semantics.
- Automatic skill/Harness activation: rejected because it bypasses review and reproducible evaluation.

## Consequences

### Positive

- Delegation, experiment and learning histories become crash-recoverable and auditable.

### Negative

- Lease, handoff and candidate lifecycle require explicit persistence and E2E coverage.

## Risks and threat boundary

No child inherits capabilities beyond a narrowed packet. Executor cannot self-review or self-approve. Candidate/model output cannot activate an artifact directly.

## Evidence

- sha: `N/A for proposed`
- tree: `N/A for proposed`
- policy: `post-270-entry-gate`

## Rollback and supersession

Fence/revoke leases, disable candidates and restore pinned versions atomically while retaining audit evidence. Future accepted ADRs supersede this proposal.
