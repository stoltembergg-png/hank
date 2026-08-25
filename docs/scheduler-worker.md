# Scheduler worker

The worker is intentionally a bounded coordinator, not an execution engine.

Each `tick(project, now_ms)` claims at most the configured number of due runs through the durable
scheduler persistence boundary. For every claim it publishes a `DispatchEnvelope` containing only
project/job/run identity and a deterministic `scheduler:<project>:<run>` idempotency key.

The event bus is bounded. If dispatch has no consumer or is closed, the worker fails closed; the
lease remains durable and can be recovered after expiry. `renew` extends only the current owner's
live lease. `shutdown` rejects future ticks and does not erase claimed state, allowing operator
restart/reconciliation without an in-memory authority.

This PR does not execute workflows, invoke providers, poll forever, or interpret trigger policy.
