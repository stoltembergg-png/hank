# Planning reconciliation

`PlanningReconciliation` is the bounded, provider-neutral reconciliation
boundary for the Harness Planning pipeline. The Application layer supplies a
versioned request containing the planner identity, reviewer findings and the
`project_id`/`run_id`/`trace_id` scope. The contract returns an immutable
`FinalPlan` artifact or an explicit cancelled outcome.

## Decision rules

- Findings are deduplicated by affected contract, normalized consequence and
  evidence digest. Every original `finding_id` remains in `FinalPlan.findings`;
  deduplication only coalesces the decision and preserves reviewer provenance.
- A group with conflicting reviewer dispositions is `HUMAN_REQUIRED`.
- An unresolved policy/product conflict is always `HUMAN_REQUIRED`.
- High and critical findings require at least one structurally valid,
  scope-matching `Verified` evidence reference for any automatic disposition.
  Missing, stale or unverified evidence escalates to `HUMAN_REQUIRED`.
- A missing low/medium disposition is retained as `Defer`; it is not silently
  treated as approval.
- Planner, reviewer and judge identities cannot overlap. The judge is
  advisory to this artifact and cannot approve itself or create capabilities.

The approved maximum is two reconciliation rounds. An input beyond that bound
is rejected with `RoundOverflow`; the reconciler never loops until convergence.

## Artifact and rollback

`FinalPlan` is schema-versioned, project/run/trace scoped, idempotent for the
same input and serialized with unknown fields rejected. It exposes no plan
execution operation and does not access UI, storage, providers, tools or
secrets. Metrics contain only bounded counters for dispositions and conflicts;
raw reviewer text is not used as authority.

Rollback is data-only: `FinalPlan::reopen` creates a new draft carrying the
original findings and its fingerprint in `reopened_from`. The original artifact
is not mutated. A later Application command must submit that draft through the
normal authorization and execution gates.

Evidence resolver binding and durable persistence are intentionally deferred to
the following planning card; this contract validates scope, shape and trust
state without inventing external evidence.
