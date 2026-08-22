---
id: ADR-HAR-004
status: proposed
owner: architecture-owner
date: 2026-08-22
---

# ADR-HAR-004 — Checkpoint, replay and experiment isolation

## Context

Interrupted runs need recovery, but blind resume/replay can duplicate external effects or restore untrusted free-form state.

## Decision

Immutable checkpoints hold bounded run state, effect/idempotency state, tests, blockers, decisions, references, model/provider attempt, budgets and trace. Recovery reconciles effects first. Replay, shadow and experiments default to read-only or isolated worktree/sandbox authority.

## Alternatives

- Resume from model conversation text: rejected because it is not deterministic state.
- Replay original tool calls with inherited approvals: rejected because it can repeat destructive effects.

## Consequences

### Positive

- Recovery and comparison have durable reproducible lineage.

### Negative

- Checkpoint migrations, effect reconciliation and sandbox availability require explicit gates.

## Risks and threat boundary

No raw secrets/prompts are checkpoint fields. Replay/shadow never inherit write approval or credential authority; unavailable sandbox is BLOCKED.

## Evidence

- sha: `N/A for proposed`
- tree: `N/A for proposed`
- policy: `post-270-entry-gate`

## Rollback and supersession

Stop resume/replay scheduling, retain immutable checkpoint/evidence records, and use versioned compatibility readers for rollback.
