---
id: ADR-HAR-002
status: proposed
owner: architecture-owner
date: 2026-08-22
---

# ADR-HAR-002 — Context authority, conflict and budget

## Context

Context sources have different authority and freshness. Concatenating all available text lets low-trust conversation, skill, memory or tool output displace approved policy.

## Decision

Context Compiler uses the fixed order Security → Architecture/ADR → Project → Workflow → Task → Relevant Code → Decision/Failure/Project Memory → Skills → Conversation. Lower-authority conflicts are retained as bounded conflict records. ContextBudget reserves output and assigns task-class buckets without filling the whole window.

## Alternatives

- Flat prompt concatenation: rejected because authority is implicit.
- Memory-first context: rejected because memory provenance is weaker than approved ADR/security sources.

## Consequences

### Positive

- Context selection becomes reproducible and debuggable through manifests.

### Negative

- Every source needs provenance, freshness and token-cost metadata.

## Risks and threat boundary

User/provider/tool/skill/memory text is untrusted. It cannot create authority, override Security, or suppress conflict metadata.

## Evidence

- sha: `N/A for proposed`
- tree: `N/A for proposed`
- policy: `post-270-entry-gate`

## Rollback and supersession

Pin the prior authority/budget policy revision and retain manifests as evidence; do not reuse stale context after rollback.
