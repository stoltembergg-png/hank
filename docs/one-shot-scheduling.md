# One-shot scheduling

One-shot jobs have a single epoch-millisecond `due_at` encoded by `Trigger::OneShot`.
A caller must provide an explicit clock value and an actor matching the persisted owner.

## Lifecycle

1. `active`, enabled job waits until `now_ms >= due_at`.
2. Optional `expires_at_ms` is a strict upper bound; at or after expiry the claim is rejected.
3. `claim_one_shot` atomically writes `claim_id` and `consumed_at_ms`.
4. A repeated request with the same project, job and claim key returns the same receipt.
5. A different claim key, disabled/archived job, wrong owner or wrong project fails closed.

The operation records consumption; it does not execute a workflow, start a worker, enqueue work,
or emit notifications. PR-195 owns broader scheduler persistence and leases.

The conditional update is the authority during concurrent claims. The caller must treat a successful
receipt as consumed before any external execution and use the claim key for recovery/replay handling.
