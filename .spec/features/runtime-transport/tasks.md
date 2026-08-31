# Tasks: runtime transport

> feature: runtime-transport

## T-1401 — Bounded runtime framing [concluida]

- Refs: US-1401, AC-1401
- Arquivos: crates/agent-protocol/src/runtime_transport.rs, crates/agent-protocol/tests/runtime_transport_contract.rs, docs/runtime-transport.md

## T-1402 — Session lifecycle and backpressure [concluida]

- Refs: US-1401, AC-1402
- Arquivos: crates/agent-protocol/src/runtime_transport.rs, crates/agent-protocol/tests/runtime_transport_contract.rs, .github/workflows/onp-sdd-evidence.yml, test/aggregate-runner-native-boundary.js

## Suposições

- ASM-1401: autenticação, socket/daemon e dispatch são responsabilidades de adapters posteriores.
