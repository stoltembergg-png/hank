# Workflow crash recovery runbook

`RecoveryStore` is a bounded startup/restart boundary. It does not schedule work and never
invokes a provider, tool, Python worker, or capability.

1. Acquire the run lease with a runner identity, finite TTL, and the current persisted epoch.
2. Reject a second runner while the lease is active.
3. After expiry, acquire a new lease and increment the epoch; old runner identities fail fencing.
4. Scan at most the requested candidate budget in deterministic `run_id` order.
5. A node left in `running` is marked `unknown` with `unknown_effect=1` and a persisted,
   redacted recovery report. It requires human/operator reconciliation before execution.
6. Do not retry unknown effects automatically. A corrupted journal, invalid identity, missing
   project scope, or invalid budget fails closed.

This procedure does not claim zero data loss, full database recovery, scheduler behavior, or
automatic approval/reconciliation. Rollback is to stop the scanner and preserve the journal;
no migration is rewritten or deleted.
