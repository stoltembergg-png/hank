---
id: ADR-HAR-003
status: proposed
owner: architecture-owner
date: 2026-08-22
---

# ADR-HAR-003 — Memory provenance and Evidence Engine

## Context

Generic memory and model-text claims cannot establish provenance, retention, authority or verified fact status.

## Decision

Memory is typed as working, session, project, long-term, skill, decision or failure. Every record has project/owner, provenance, retention, version and evidence references. Claims transition only through resolver evidence to VERIFIED, UNVERIFIED, CONFLICTING, STALE or NO_PROOF.

## Alternatives

- One generic memory/vector bucket: rejected because it cannot enforce decision/failure authority.
- Claim text as fact: rejected because it enables fabricated evidence.

## Consequences

### Positive

- Failure and decision memory can be retrieved with explicit evidence lineage.

### Negative

- Candidate quarantine, deletion and retention policies need persistent lifecycle support.

## Risks and threat boundary

Model/provider text may propose candidates but cannot create trusted memory. Cross-project, poisoned, secret-bearing, fabricated or stale records fail closed.

## Evidence

- sha: `N/A for proposed`
- tree: `N/A for proposed`
- policy: `post-270-entry-gate`

## Rollback and supersession

Disable candidate activation/retrieval by policy, preserve provenance/tombstones per retention policy, and supersede only through a versioned ADR.
