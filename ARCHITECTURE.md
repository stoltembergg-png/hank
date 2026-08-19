# Hank architecture

## Authority and scope

This document explains the boundaries of the approved architecture. The normative,
machine-checkable source is `.planning/contracts/architecture-graph.json` together
with its schema and invalid fixtures. The executable validator is
`tools/w0-contract-validator.mjs architecture`; a passing document check does not
claim runtime integration or production behavior.

## Context map

```text
UI adapters (Tauri / CLI / fake)
              |
              v
       application-api
              |
              v
          agent-core <--- infrastructure adapters
              ^
              |
       agent-runtime
```

- `agent-core`: domain rules, ports and invariants. It is reusable Rust library code
  and must not depend on Tauri, Tokio, SQLx or concrete providers.
- `application-api`: use cases, authorization and request/result/event envelopes.
- `agent-runtime`: execution lifecycle, cancellation, retry, leases and recovery.
- `infrastructure`: storage, provider, tool and event adapters behind ports/contracts.
- `tauri-shell`: desktop window/bridge/events/packaging; it calls application APIs.
- `cli-adapter`: non-Tauri application surface using the same application APIs.
- `fake-adapter`: deterministic contract-test surface using the same application APIs.

## Ownership and evolution

Every layer has one owner, a responsibility, a lifecycle and a contract in the graph.
New edges require an explicit graph update, an allowed edge, tests, and review of
security boundaries. A new adapter must depend on application contracts, not reach
through them to SQLite, filesystem, providers or private domain rules.

The core Rust library remains portable and reusable. Concrete SDKs, storage engines,
Tauri APIs and process-specific resources stay outside the core. Compatibility is
preserved by versioned commands/results/events and by adapters that translate at the
boundary.

## Threat boundaries

- Secrets enter through an approved permission/authorization path and never through
  frontend imports or domain objects.
- Frontend code must not import SQLite, SQLx, filesystem APIs or concrete providers.
- `agent-core` must not import Tauri or concrete infrastructure.
- Tauri and CLI surfaces must not bypass `application-api`.
- Project isolation, provider permissions and event/run identity are checked at the
  application/runtime boundary.

## Lifecycle and evidence

The graph describes intended boundaries; it is not evidence that a feature or adapter
has executed. Evidence is bound to SHA, tree and policy/schema identity. Statuses are
`PASS`, `FAIL`, `BLOCKED` or `NO_PROOF`; pending or stale evidence is never promoted.

## Change and rollback

Architecture changes are introduced as a small queue card with updated graph fixture,
negative boundary test and documentation. Rollback reverts the graph/document/test
change together. Do not freeze APIs that have not been proven by implementation and
contract tests.
