# Planning evidence binding

`PlanningEvidenceAdapter` is the pure bridge between a
`planning_reconciliation::ReviewerFinding` and the provider-neutral
`Claim`/`EvidenceRecord` contract. A reviewer reference is only a pointer;
the adapter requires a resolver record with the same evidence ID, digest,
claim, project/run/trace scope, identity scope and status.

## Binding rules

- A finding without resolver evidence, or with an explicitly missing
  reference, produces `NO_PROOF`.
- A fabricated reference, an orphan record, a digest/status mismatch, a
  foreign claim, or a different identity (including commit/tree) fails closed.
- `VERIFIED`, `UNVERIFIED`, `STALE` and `CONFLICTING` are projected from the
  resolver records. Conflicting evidence has precedence over stale evidence,
  which has precedence over insufficient evidence.
- `MITIGATE` is retained as mitigable only when the resulting claim is
  `VERIFIED`. Non-verified evidence remains observable and cannot authorize
  execution, approval or merge.
- Request and result data are versioned, bounded and deterministic. A
  cancelled request returns a cancellation fingerprint without a claim or
  evidence effect. Replaying the same input yields the same fingerprint.

The adapter does not resolve Git, CI, filesystem or provider evidence and does
not persist the result. Those integrations belong to later cards. Disabling
the adapter therefore has the safe rollback behavior: no binding is produced
and the finding remains `NO_PROOF` until an independent resolver path supplies
matching evidence.
