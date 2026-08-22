---
id: ADR-HAR-001
status: proposed
owner: architecture-owner
date: 2026-08-22
---

# ADR-HAR-001 — Provider-neutral Harness Run identity

## Context

Agent state, checkpoint, evidence and budget ownership must survive model/provider selection, fallback and hot swap without allowing provider SDK concerns into Core.

## Decision

A Harness Run is a project/agent-scoped durable aggregate independent of provider/model. Provider/model selection is an attempt attribute. The run records schema version, run/project/agent/session/task/trace IDs, state/generation, policy/schema revisions and references to budget, checkpoint, memory and evidence.

## Alternatives

- Provider-specific run aggregates: rejected because they couple state persistence to SDK identity.
- Raw prompt/completion persistence in Run: rejected because it increases privacy and secret exposure.

## Consequences

### Positive

- Model hot swap, fallback, replay and shadow share one Run lineage.
- `agent-core` remains provider-neutral.

### Negative

- Provider attempts require explicit versioned adapter records.

## Risks and threat boundary

Provider/model output is untrusted attempt data and cannot mutate Run authority. Cross-project or stale generation access fails closed.

## Evidence

- sha: `N/A for proposed`
- tree: `N/A for proposed`
- policy: `post-270-entry-gate`

## Rollback and supersession

Disable new Run creation by policy and retain immutable records for compatibility readers. A successor ADR supersedes this decision only with exact-SHA contract evidence.
