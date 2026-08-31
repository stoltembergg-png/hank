# Claim/Evidence contract

`agent-core::claim_evidence` is the provider-neutral domain boundary for
turning resolver output into an auditable fact state. A `Claim` stores a
bounded digest, required evidence kinds and an expected identity. It does not
store claim prose as authority.

`EvidenceRecord` carries a bounded evidence digest, resolver identifier,
evidence kind, bounded reason and the exact `EvidenceScope` of the project,
run, trace, identity digest, commit/tree, policy and schema. A record with a
different claim or identity is rejected before a state transition is applied.

The initial claim state is `NoProof`. The only path to `Verified` is
`Claim::apply_resolution` with every required evidence kind represented by a
`Verified` record whose claim and scope match exactly. `Unverified`, `Stale`,
`Conflicting` and `NoProof` remain explicit states; stale or conflicting
records never silently become proof. Resolutions and evidence records retain a
bounded reason for observability without treating that reason as authority.

The contract is bounded and versioned. Unknown fields, unsupported schema
versions, duplicate references, malformed digests, control characters and
secret-like metadata fail closed. Reapplying the same state and evidence
references is idempotent.

This module deliberately does not read Git, filesystem, CI, network or
secrets, and it cannot execute, approve or merge anything. Concrete resolvers
and the PlanningReconciliation adapter must be implemented in later cards.
