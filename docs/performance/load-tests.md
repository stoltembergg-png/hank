# Bounded load-smoke contract

PR-262 provides a deterministic admission/backpressure model for profiles S, M,
and L. It exercises concurrent admission, queue bounds, cancellation accounting,
repetition determinism, redacted fixture identity, and invalid-manifest rejection.

The lane intentionally does **not** sample host CPU, memory, disk, handles, or
network traffic. It does not contact providers or ratify production capacity.
`docs/performance/load-manifest.json` is the canonical workload identity; the
runner emits a receipt without timestamps or raw subprocess output.

## Policy

- `admitted + rejected == requests` for every profile.
- `max_in_flight <= concurrency` and `peak_queue <= queue`.
- `completed + cancelled == admitted`.
- Invalid or unbounded manifests fail closed.
- Capacity budgets remain informational until a later ratification decision.
