# Remote credential isolation

> Status: PR-250 implementation note
> Spec: `.spec/features/remote-credential-isolation/spec.md`
> Task: T-1461 (broker de credencial remoto scoped e redigido)

## Goal

Stop plaintext credentials from crossing nodes or projects and stop any peer
from resolving material outside its own scope. The broker is **transport
neutral** — it only hands out opaque, scoped, time-bounded handles for
credentials that already exist locally.

## Boundaries

- The broker never stores, serializes, sends or logs secret material. It only
  records the `CredentialRef` (an opaque identifier) of a credential that the
  caller already owns locally.
- Scope is exactly `(node, project, actor)`. Any divergence on any of the three
  fields fails closed.
- Leases are bounded by a per-call duration. Expired or revoked leases fail
  closed. Stale cleanup never reopens a lease.
- The broker is bounded to `MAX_CREDENTIAL_LEASES = 256` active leases and
  retains only the last `MAX_CREDENTIAL_AUDIT_EVENTS = 256` redacted events.

## What is and is not in this card

In scope:

- `crates/remote-core/src/credential_broker.rs` — the broker implementation.
- `crates/remote-core/tests/credential_broker_contract.rs` — contract tests.
- Redacted audit (scope + reason only).

Out of scope (deferred to later cards):

- OS keychain / Stronghold backend adapters (PR-252+).
- Migration of pre-existing secrets (PR-256).
- Transport of scoped references over a socket (PR-248, PR-249, PR-251).
- UI for rotating/revoking leases.

## How to use

```rust
use agent_protocol::ids::ProjectId;
use agent_protocol::remote_protocol::NodeId;
use provider_core::CredentialRef;
use remote_core::credential_broker::CredentialScope;
use remote_adapter::new_credential_broker;

let broker = new_credential_broker()?;
let scope = CredentialScope::new(NodeId::new("node-1")?, project, "agent-1")?;
let reference = CredentialRef::parse("cred_alpha")?;
let lease = broker.issue(scope, reference, 60_000)?;
// Transport the full `CredentialLease` to a peer. The broker will refuse
// `resolve` and `revoke` that present a caller-supplied scope or handle
// instead of the original lease; the lease is the broker-issued access
// context.
let resolved = broker.resolve(&lease)?;
broker.revoke(&lease)?;
```

The broker receives clock and entropy ports from its adapter/composition root.
Production code calls `remote_adapter::new_credential_broker()`, which selects
`SystemClock` + `OsEntropy`; `OsEntropy` reads 128 bits from the operating system
CSPRNG through `getrandom`. If that source fails, construction returns
`CredentialBrokerError::EntropyUnavailable` and does not fall back to a
time-derived or counter-derived seed. Tests inject deterministic clock and
entropy stubs through `CredentialBroker::with_clock_and_entropy`.
The caller cannot pick the start time of an issued lease, nor pick the
timestamp used to evaluate expiry, so a peer cannot bypass a lease
deadline by submitting a backdated timestamp.

## Threat model summary

- **Plaintext egress** — neutralized: the handle carries only a SHA-256 digest of
  the scope plus the local reference, never the value.
- **Cross-scope resolve** — denied: every resolve re-validates node, project and
  actor. The audit log records the failing scope.
- **Replay after expiry/revoke** — denied: expiry is checked against the lease
  deadline; revoke sets a flag that is checked before scope. The audit log
  records the precise reason.
- **Broker capacity abuse** — denied: at `MAX_CREDENTIAL_LEASES` active leases,
  further `issue` calls return `CapacityExhausted` and the failure is audited.
- **Secret leak via audit log** — prevented: only `(node, project, actor)` and a
  reason enum are kept. The contract test asserts that no label like
  `cred_alpha` ever appears in a debug-formatted event.

## Required check evidence

- `cargo fmt --all -- --check` — PASS
- `cargo clippy -p remote-core --all-targets -- -D warnings` — PASS
- `cargo test -p remote-core` — 17 broker contract tests plus 4 daemon and
  8 event-stream contract tests PASS; `cargo test -p remote-adapter` — 2
  adapter contract tests PASS
