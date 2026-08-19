# Event bus contract

The runtime event bus is bounded and typed. `EventBus::bounded` rejects zero capacity;
`publish` returns an explicit error when closed or without subscribers; subscribers
observe FIFO order and receive `Lagged` when a slow consumer exceeds the bounded queue.

Shutdown is explicit, no unbounded queue is created, and subscriber lag is observable
through the returned error. Persistence, remote delivery and UI consumers remain out
of scope for PR-024. Tests cover order, close and bounded lag behavior.
